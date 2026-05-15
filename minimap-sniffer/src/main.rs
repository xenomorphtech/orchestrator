mod darkdec;
mod packet;
mod state;
mod tcp_reassembly;
mod web;

use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
#[cfg(feature = "live-pcap")]
use std::time::Instant;

use darkdec::decode_frame;
use state::WorldState;
use tcp_reassembly::{frames_from_ordered_stream, Direction};
#[cfg(feature = "live-pcap")]
use tcp_reassembly::TcpReassembler;
use web::{broadcast, new_clients, spawn_server};

#[derive(Clone, Debug)]
struct Config {
    iface: Option<String>,
    port: u16,
    bind: SocketAddr,
    list_devices: bool,
    no_promisc: bool,
    offline_stream_dir: Option<PathBuf>,
    replay_ms: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_args()?;

    if config.list_devices {
        return list_devices();
    }

    let state = Arc::new(Mutex::new(WorldState::default()));
    let clients = new_clients();
    spawn_server(config.bind, Arc::clone(&clients), Arc::clone(&state))?;

    if let Some(dir) = config.offline_stream_dir.clone() {
        run_offline_replay(dir, config.replay_ms, state, clients)?;
    } else {
        run_live_capture(config, state, clients)?;
    }

    Ok(())
}

fn parse_args() -> Result<Config, Box<dyn Error>> {
    let mut config = Config {
        iface: None,
        port: 10001,
        bind: "127.0.0.1:17891".parse()?,
        list_devices: false,
        no_promisc: false,
        offline_stream_dir: None,
        replay_ms: 0,
    };

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iface" => config.iface = Some(require_arg(&mut args, "--iface")?),
            "--port" => config.port = require_arg(&mut args, "--port")?.parse()?,
            "--bind" => config.bind = require_arg(&mut args, "--bind")?.parse()?,
            "--list-devices" => config.list_devices = true,
            "--no-promisc" => config.no_promisc = true,
            "--offline-stream-dir" => {
                config.offline_stream_dir = Some(PathBuf::from(require_arg(
                    &mut args,
                    "--offline-stream-dir",
                )?))
            }
            "--replay-ms" => config.replay_ms = require_arg(&mut args, "--replay-ms")?.parse()?,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    Ok(config)
}

fn require_arg(
    args: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, Box<dyn Error>> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value").into())
}

fn print_help() {
    println!(
        "darkdec-minimap-sniffer\n\
         \n\
         --list-devices\n\
         --iface <name-or-description>\n\
         --port <port>                    default: 10001\n\
         --bind <addr:port>               default: 127.0.0.1:17891\n\
         --no-promisc\n\
         --offline-stream-dir <path>\n\
         --replay-ms <milliseconds>"
    );
}

fn run_offline_replay(
    dir: PathBuf,
    replay_ms: u64,
    state: Arc<Mutex<WorldState>>,
    clients: web::Clients,
) -> Result<(), Box<dyn Error>> {
    let s2c = std::fs::read(dir.join("first_quest_s2c.tcpstream.bin"))?;
    let frames = frames_from_ordered_stream(Direction::S2C, &s2c);
    println!("offline_frames={}", frames.len());

    let mut t = 0.0;
    for frame in frames {
        {
            let mut state = state.lock().expect("state lock poisoned");
            state.stats.app_frames += 1;
        }
        if let Some(update) = decode_frame(&frame, t) {
            let json = {
                let mut state = state.lock().expect("state lock poisoned");
                state.apply_update(update)
            };
            broadcast(&clients, json);
            if replay_ms > 0 {
                thread::sleep(Duration::from_millis(replay_ms));
            }
        }
        t += 0.025;
    }

    println!("offline replay complete; minimap remains open");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(feature = "live-pcap")]
fn list_devices() -> Result<(), Box<dyn Error>> {
    for device in pcap::Device::list()? {
        println!("{}", device.name);
        if let Some(description) = device.desc {
            println!("  {description}");
        }
        for address in device.addresses {
            println!("  {:?}", address.addr);
        }
    }
    Ok(())
}

#[cfg(not(feature = "live-pcap"))]
fn list_devices() -> Result<(), Box<dyn Error>> {
    Err("live pcap support is disabled; build without --no-default-features".into())
}

#[cfg(feature = "live-pcap")]
fn run_live_capture(
    config: Config,
    state: Arc<Mutex<WorldState>>,
    clients: web::Clients,
) -> Result<(), Box<dyn Error>> {
    use pcap::{Capture, Error as PcapError};

    let device = select_device(config.iface.as_deref())?;
    println!("capture_device={}", device.name);
    if let Some(desc) = &device.desc {
        println!("capture_description={desc}");
    }

    let mut capture = Capture::from_device(device)?
        .promisc(!config.no_promisc)
        .snaplen(65535)
        .timeout(50)
        .open()?;
    capture.filter(&format!("tcp port {}", config.port), true)?;

    let start = Instant::now();
    let mut reassembler = TcpReassembler::default();

    loop {
        let packet = match capture.next_packet() {
            Ok(packet) => packet,
            Err(PcapError::TimeoutExpired) => continue,
            Err(error) => return Err(error.into()),
        };

        {
            let mut state = state.lock().expect("state lock poisoned");
            state.stats.packets += 1;
        }

        let Some(tcp) = packet::parse_ethernet_ipv4_tcp(packet.data, config.port) else {
            continue;
        };

        {
            let mut state = state.lock().expect("state lock poisoned");
            state.stats.tcp_payloads += 1;
        }

        let now = start.elapsed().as_secs_f64();
        for frame in reassembler.feed(&tcp) {
            {
                let mut state = state.lock().expect("state lock poisoned");
                state.stats.app_frames += 1;
            }
            if let Some(update) = decode_frame(&frame, now) {
                let json = {
                    let mut state = state.lock().expect("state lock poisoned");
                    state.apply_update(update)
                };
                broadcast(&clients, json);
            }
        }
    }
}

#[cfg(not(feature = "live-pcap"))]
fn run_live_capture(
    _config: Config,
    _state: Arc<Mutex<WorldState>>,
    _clients: web::Clients,
) -> Result<(), Box<dyn Error>> {
    Err("live pcap support is disabled; build without --no-default-features".into())
}

#[cfg(feature = "live-pcap")]
fn select_device(query: Option<&str>) -> Result<pcap::Device, Box<dyn Error>> {
    let devices = pcap::Device::list()?;
    if let Some(query) = query {
        let needle = query.to_ascii_lowercase();
        return devices
            .into_iter()
            .find(|device| {
                device.name.to_ascii_lowercase().contains(&needle)
                    || device
                        .desc
                        .as_deref()
                        .unwrap_or("")
                        .to_ascii_lowercase()
                        .contains(&needle)
            })
            .ok_or_else(|| format!("pcap device not found: {query}").into());
    }

    match pcap::Device::lookup()? {
        Some(device) => Ok(device),
        None => Err("pcap default device not found".into()),
    }
}
