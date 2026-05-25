use clap::Parser;
use std::error::Error;
use std::fmt::Display;
use std::net::Ipv4Addr;
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

    spawn_traceroute_worker(config.ip.clone(), tx);

    tui::run_tui(&config, rx)
}

fn spawn_traceroute_worker(target_ip: String, tx: OverlayTx) {
    thread::spawn(move || {
        let target = match Ipv4Addr::from_str(&target_ip) {
            Ok(ip) => ip,
            Err(err) => {
                let _ = send_hop(&tx, format!("target parse error: {err}"));
                return;
            }
        };

        run_traceroute(target, &tx, net::probe_v4);
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
    /// Target IPv4 address
    ip: String,
}

#[derive(Debug)]
pub struct Config {
    pub ip: String,
}

fn parse_and_validate() -> Result<Config, Box<dyn Error>> {
    let cli = Cli::parse();

    cli.ip
        .parse::<Ipv4Addr>()
        .map_err(|e| format!("Invalid IPv4 address '{}': {}", cli.ip, e))?;

    Ok(Config { ip: cli.ip })
}
