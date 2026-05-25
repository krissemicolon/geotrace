use clap::{ArgAction, Parser};
use std::error::Error;
use std::fmt::Display;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

mod api;
mod net;
mod tui;

const MAX_TTL: u32 = 30;
type OverlayTx = mpsc::Sender<tui::OverlayEvent>;

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_and_validate()?;
    let (tx, rx) = mpsc::channel::<tui::OverlayEvent>();

    spawn_traceroute_worker(config.mode, config.ip.clone(), tx);

    tui::run_tui(&config, rx)
}

fn spawn_traceroute_worker(mode: Mode, target_ip: String, tx: OverlayTx) {
    thread::spawn(move || match mode {
        Mode::V4 => {
            let target = match Ipv4Addr::from_str(&target_ip) {
                Ok(ip) => ip,
                Err(err) => {
                    let _ = send_hop(&tx, format!("target parse error: {err}"));
                    return;
                }
            };

            run_traceroute(target, &tx, net::probe_v4);
        }
        Mode::V6 => {
            let target = match Ipv6Addr::from_str(&target_ip) {
                Ok(ip) => ip,
                Err(err) => {
                    let _ = send_hop(&tx, format!("target parse error: {err}"));
                    return;
                }
            };

            run_traceroute(target, &tx, net::probe_v6);
        }
    });
}

fn run_traceroute<T, F>(target: T, tx: &OverlayTx, mut probe: F)
where
    T: Copy + Eq + Display,
    F: FnMut(T, u32) -> std::io::Result<Option<T>>,
{
    for ttl in 1..=MAX_TTL {
        let hop = match probe(target, ttl) {
            Ok(hop) => hop,
            Err(err) => {
                if !send_hop(tx, format!("{:>2}  ! {}", ttl, err)) {
                    break;
                }
                continue;
            }
        };

        match hop {
            Some(ip) => {
                let hop_addr = ip.to_string();
                match api::lookup_geo_info(&hop_addr) {
                    Ok(geo) => {
                        if !send_hop(
                            tx,
                            format!(
                                "{:>2}  {} ({}, {})",
                                ttl, ip, geo.continent_code, geo.country
                            ),
                        ) {
                            break;
                        }

                        if tx.send(tui::OverlayEvent::AddPoint(geo.coord)).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        if !send_hop(tx, format!("{:>2}  {}", ttl, ip)) {
                            break;
                        }
                    }
                }

                if ip == target {
                    let _ = send_hop(tx, "Reached target");
                    break;
                }
            }
            None => {
                if !send_hop(tx, format!("{:>2}  *", ttl)) {
                    break;
                }
            }
        }
    }
}

fn send_hop(tx: &OverlayTx, line: impl Into<String>) -> bool {
    tx.send(tui::OverlayEvent::AddHop(line.into())).is_ok()
}

#[derive(PartialEq, Debug, Clone, Copy)]
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
#[command(name = "atlas-rewrite", version, about = "Atlas")]
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
