use std::error::Error;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use clap::{ArgAction, Parser};

mod tui;

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_and_validate()?;

    let (tx, rx) = mpsc::channel::<tui::OverlayEvent>();

    thread::spawn(move || {
        let computed_points = [
            (-122.4194, 37.7749), // SF
            (-118.2437, 34.0522), // LA
            (8.5411, 47.3744),    // Zurich
            (-95.3698, 29.7604),  // Houston
        ];

        for point in computed_points {
            thread::sleep(Duration::from_millis(800));

            if tx.send(tui::OverlayEvent::AddPoint(point)).is_err() {
                break;
            }
        }
    });

    tui::run_tui(&config, rx)
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    V4,
    V6,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::V4 => "IPv4",
            Mode::V6 => "IPv6",
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "atlas-rewrite", version, about = "Atlas rewrite TUI scaffold")]
struct Cli {
    /// Target IP address
    ip: String,

    /// Use IPv4 mode (default when neither --v4 nor --v6 is set)
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "v6")]
    v4: bool,

    /// Use IPv6 mode
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "v4")]
    v6: bool,
}

#[derive(Debug)]
pub struct Config {
    pub ip: String,
    pub mode: Mode,
}

fn parse_and_validate() -> Result<Config, Box<dyn Error>> {
    let cli = Cli::parse();
    let mode = if cli.v6 { Mode::V6 } else { Mode::V4 };

    match mode {
        Mode::V4 => {
            cli.ip
                .parse::<Ipv4Addr>()
                .map_err(|e| format!("Invalid IPv4 address '{}': {}", cli.ip, e))?;
        }
        Mode::V6 => {
            cli.ip
                .parse::<Ipv6Addr>()
                .map_err(|e| format!("Invalid IPv6 address '{}': {}", cli.ip, e))?;
        }
    }

    Ok(Config { ip: cli.ip, mode })
}
