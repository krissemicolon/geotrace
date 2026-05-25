use std::error::Error;
use std::io;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use ::tui::Terminal;
use ::tui::backend::CrosstermBackend;
use ::tui::layout::{Constraint, Direction, Layout};
use ::tui::style::{Color, Style};
use ::tui::widgets::canvas::{Canvas, Line, Map, MapResolution, Points};
use ::tui::widgets::{Block, Borders, List, ListItem, Paragraph};
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use crate::Config;
use crate::api::Coord;

pub enum OverlayEvent {
    AddPoint(Coord),
    AddHop(String),
}

pub struct MapOverlay {
    points: Vec<Coord>,
    lines: Vec<(Coord, Coord)>,
}

impl MapOverlay {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// Adds a point to the map.
    ///
    /// If this is not the first point, a connecting line is automatically
    /// added from the previous point to this new point.
    pub fn draw_point(&mut self, coord: Coord) {
        if let Some(previous) = self.points.last().copied() {
            self.lines.push((previous, coord));
        }

        self.points.push(coord);
    }
}

pub fn run_tui(config: &Config, rx: Receiver<OverlayEvent>) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut overlay = MapOverlay::new();
    let mut hops: Vec<String> = Vec::new();

    let result = (|| -> Result<(), Box<dyn Error>> {
        loop {
            loop {
                match rx.try_recv() {
                    Ok(OverlayEvent::AddPoint(coord)) => overlay.draw_point(coord),
                    Ok(OverlayEvent::AddHop(line)) => hops.push(line),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }

            terminal.draw(|f| {
                let size = f.size();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
                    .split(size);

                let body = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                    .split(chunks[1]);

                let info =
                    Paragraph::new(format!("Target IPv4: {} | Press 'q' to quit", config.ip))
                        .block(Block::default().borders(Borders::ALL).title("Information"));

                let map = Canvas::default()
                    .block(Block::default().borders(Borders::ALL).title("Map"))
                    .x_bounds([-180.0, 180.0])
                    .y_bounds([-90.0, 90.0])
                    .paint(|ctx| {
                        ctx.draw(&Map {
                            resolution: MapResolution::High,
                            color: Color::White,
                        });

                        // Lines (alternating LightYellow/Yellow)
                        for (i, (start, end)) in overlay.lines.iter().enumerate() {
                            ctx.draw(&Line {
                                x1: start.0,
                                y1: start.1,
                                x2: end.0,
                                y2: end.1,
                                color: if i % 2 == 0 {
                                    Color::LightYellow
                                } else {
                                    Color::Yellow
                                },
                            });
                        }

                        // Points
                        if !overlay.points.is_empty() {
                            ctx.draw(&Points {
                                coords: &overlay.points,
                                color: Color::Red,
                            });
                        }
                    });

                let hop_items: Vec<ListItem> = hops
                    .iter()
                    .map(|line| ListItem::new(line.as_str()))
                    .collect();

                let hop_list = List::new(hop_items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Traceroute Hops")
                        .border_style(Style::default().fg(Color::LightYellow)),
                );

                f.render_widget(info, chunks[0]);
                f.render_widget(map, body[0]);
                f.render_widget(hop_list, body[1]);
            })?;

            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }

        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
