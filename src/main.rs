use clap::Parser;
use std::error::Error;
use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr};

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

    spawn_traceroute_worker(config.target_ip, tx);

    tui::run_tui(&config, rx)
}

fn spawn_traceroute_worker(target: Ipv4Addr, tx: OverlayTx) {
    thread::spawn(move || {
        run_traceroute(target, &tx, net::probe);
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
                                "{:>2}  {} ({}, {} - {})",
                                ttl, ip, geo.continent_code, geo.country, geo.city
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

#[derive(Parser, Debug)]
#[command(name = "atlas-rewrite", version, about = "Atlas")]
struct Cli {
    /// Target IPv4 address or domain name
    target: String,
}

#[derive(Debug)]
pub struct Config {
    pub target: String,
    pub target_ip: Ipv4Addr,
}

fn parse_and_validate() -> Result<Config, Box<dyn Error>> {
    let cli = Cli::parse();

    let target_ip = match cli.target.parse::<Ipv4Addr>() {
        Ok(ip) => ip,
        Err(_) => {
            let addrs = net::resolve_host(&cli.target)
                .map_err(|e| format!("Failed to resolve host '{}': {}", cli.target, e))?;

            addrs
                .into_iter()
                .find_map(|addr| match addr {
                    IpAddr::V4(ip) => Some(ip),
                    IpAddr::V6(_) => None,
                })
                .ok_or_else(|| format!("No IPv4 address found for host '{}'", cli.target))?
        }
    };

    Ok(Config {
        target: cli.target,
        target_ip,
    })
}
