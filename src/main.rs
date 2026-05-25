use std::error::Error;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

use clap::{ArgAction, Parser};

mod api;
mod net;
mod tui;

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_and_validate()?;

    let (tx, rx) = mpsc::channel::<tui::OverlayEvent>();
    let worker_mode = config.mode;
    let worker_ip = config.ip.clone();

    thread::spawn(move || {
        let send_hop = |line: String, tx: &mpsc::Sender<tui::OverlayEvent>| {
            tx.send(tui::OverlayEvent::AddHop(line)).is_ok()
        };

        match worker_mode {
            Mode::V4 => {
                let target = match Ipv4Addr::from_str(&worker_ip) {
                    Ok(ip) => ip,
                    Err(err) => {
                        let _ = tx.send(tui::OverlayEvent::AddHop(format!(
                            "target parse error: {err}"
                        )));
                        return;
                    }
                };

                for ttl in 1..=30 {
                    let hop = match net::probe_v4(target, ttl) {
                        Ok(hop) => hop,
                        Err(err) => {
                            if !send_hop(format!("{:>2}  ! {}", ttl, err), &tx) {
                                break;
                            }
                            continue;
                        }
                    };

                    match hop {
                        Some(ip) => {
                            let hop_host = ip.to_string();
                            match api::get_geo_from_host(&hop_host) {
                                Ok(geo) => {
                                    if !send_hop(
                                        format!(
                                            "{:>2}  {} ({}, {})",
                                            ttl, ip, geo.continent_code, geo.country
                                        ),
                                        &tx,
                                    ) {
                                        break;
                                    }

                                    if tx.send(tui::OverlayEvent::AddPoint(geo.coord)).is_err() {
                                        break;
                                    }
                                }
                                Err(_) => {
                                    if !send_hop(format!("{:>2}  {}", ttl, ip), &tx) {
                                        break;
                                    }
                                }
                            }

                            if ip == target {
                                let _ = send_hop("Reached target".to_string(), &tx);
                                break;
                            }
                        }
                        None => {
                            if !send_hop(format!("{:>2}  *", ttl), &tx) {
                                break;
                            }
                        }
                    }
                }
            }
            Mode::V6 => {
                let target = match Ipv6Addr::from_str(&worker_ip) {
                    Ok(ip) => ip,
                    Err(err) => {
                        let _ = tx.send(tui::OverlayEvent::AddHop(format!(
                            "target parse error: {err}"
                        )));
                        return;
                    }
                };

                for ttl in 1..=30 {
                    let hop = match net::probe_v6(target, ttl) {
                        Ok(hop) => hop,
                        Err(err) => {
                            if !send_hop(format!("{:>2}  ! {}", ttl, err), &tx) {
                                break;
                            }
                            continue;
                        }
                    };

                    match hop {
                        Some(ip) => {
                            let hop_host = ip.to_string();
                            match api::get_geo_from_host(&hop_host) {
                                Ok(geo) => {
                                    if !send_hop(
                                        format!(
                                            "{:>2}  {} ({}, {})",
                                            ttl, ip, geo.continent_code, geo.country
                                        ),
                                        &tx,
                                    ) {
                                        break;
                                    }

                                    if tx.send(tui::OverlayEvent::AddPoint(geo.coord)).is_err() {
                                        break;
                                    }
                                }
                                Err(_) => {
                                    if !send_hop(format!("{:>2}  {}", ttl, ip), &tx) {
                                        break;
                                    }
                                }
                            }

                            if ip == target {
                                let _ = send_hop("Reached target".to_string(), &tx);
                                break;
                            }
                        }
                        None => {
                            if !send_hop(format!("{:>2}  *", ttl), &tx) {
                                break;
                            }
                        }
                    }
                }
            }
        }
    });

    tui::run_tui(&config, rx)
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
