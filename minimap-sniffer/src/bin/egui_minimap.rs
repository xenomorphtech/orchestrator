use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
#[cfg(feature = "live-pcap")]
use std::time::Instant;

use darkdec_minimap_sniffer::darkdec::{decode_frame, DecodedUpdate};
use darkdec_minimap_sniffer::state::Stats;
use darkdec_minimap_sniffer::tcp_reassembly::{frames_from_ordered_stream, Direction};
#[cfg(feature = "live-pcap")]
use darkdec_minimap_sniffer::tcp_reassembly::TcpReassembler;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

#[derive(Clone, Debug)]
struct Config {
    iface: Option<String>,
    port: u16,
    list_devices: bool,
    no_promisc: bool,
    offline_stream_dir: Option<PathBuf>,
    replay_ms: u64,
}

#[derive(Debug)]
enum UiEvent {
    Update(DecodedUpdate),
    Stats(Stats),
    Status(String),
}

#[derive(Clone, Copy, Debug)]
struct Point {
    t: f64,
    x: f32,
    z: f32,
    rot: f32,
}

#[derive(Clone, Debug)]
struct EntityMarker {
    id: u8,
    point: Point,
    updates: u64,
}

struct MinimapApp {
    rx: Receiver<UiEvent>,
    player: Option<Point>,
    player_updates: u64,
    entities: BTreeMap<u8, EntityMarker>,
    player_trail: VecDeque<Point>,
    entity_trails: BTreeMap<u8, VecDeque<Point>>,
    stats: Stats,
    status: String,
    center: (f32, f32),
    zoom: f32,
    follow: bool,
    trails: bool,
    active_window: f64,
}

impl MinimapApp {
    fn new(rx: Receiver<UiEvent>) -> Self {
        Self {
            rx,
            player: None,
            player_updates: 0,
            entities: BTreeMap::new(),
            player_trail: VecDeque::new(),
            entity_trails: BTreeMap::new(),
            stats: Stats::default(),
            status: "waiting".to_string(),
            center: (0.0, 0.0),
            zoom: 0.22,
            follow: true,
            trails: true,
            active_window: 5.0,
        }
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                UiEvent::Update(update) => self.apply_update(update),
                UiEvent::Stats(stats) => self.stats = stats,
                UiEvent::Status(status) => self.status = status,
            }
        }
    }

    fn apply_update(&mut self, update: DecodedUpdate) {
        match update {
            DecodedUpdate::Player(update) => {
                let point = Point {
                    t: update.t,
                    x: update.x,
                    z: update.z,
                    rot: update.rot,
                };
                self.player = Some(point);
                self.player_updates += 1;
                push_limited(&mut self.player_trail, point, 2_000);
                if self.follow {
                    self.center = (point.x, point.z);
                }
            }
            DecodedUpdate::Entity(update) => {
                let point = Point {
                    t: update.t,
                    x: update.x,
                    z: update.z,
                    rot: update.rot,
                };
                let updates = self
                    .entities
                    .get(&update.id)
                    .map(|entity| entity.updates + 1)
                    .unwrap_or(1);
                self.entities.insert(
                    update.id,
                    EntityMarker {
                        id: update.id,
                        point,
                        updates,
                    },
                );
                push_limited(
                    self.entity_trails.entry(update.id).or_default(),
                    point,
                    700,
                );
            }
        }
    }

    fn draw_minimap(&mut self, ui: &mut egui::Ui) {
        let desired = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(desired, Sense::drag());
        let painter = ui.painter_at(rect);

        if response.dragged() {
            self.follow = false;
            let delta = ui.input(|input| input.pointer.delta());
            self.center.0 -= delta.x / self.zoom;
            self.center.1 += delta.y / self.zoom;
        }

        painter.rect_filled(rect, 0.0, Color32::from_rgb(12, 17, 23));
        self.draw_grid(rect, &painter);

        if self.trails {
            draw_trail(
                rect,
                &painter,
                &self.player_trail,
                self.center,
                self.zoom,
                Stroke::new(2.5, Color32::from_rgba_unmultiplied(81, 183, 255, 160)),
            );
            for trail in self.entity_trails.values() {
                draw_trail(
                    rect,
                    &painter,
                    trail,
                    self.center,
                    self.zoom,
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 103, 95, 58)),
                );
            }
        }

        let now = self.player.map(|p| p.t).unwrap_or(0.0);
        for entity in self.entities.values() {
            let age = (now - entity.point.t).max(0.0);
            let active = age <= self.active_window;
            let pos = to_screen(rect, entity.point.x, entity.point.z, self.center, self.zoom);
            let color = if active {
                Color32::from_rgb(255, 103, 95)
            } else {
                Color32::from_rgb(110, 120, 130)
            };
            painter.circle_filled(pos, if active { 4.8 } else { 3.6 }, color);
            if active {
                painter.text(
                    pos + Vec2::new(7.0, -7.0),
                    egui::Align2::LEFT_CENTER,
                    format!("0x{:02x}", entity.id),
                    egui::FontId::monospace(11.0),
                    Color32::from_rgb(238, 243, 247),
                );
            }
        }

        if let Some(player) = self.player {
            let pos = to_screen(rect, player.x, player.z, self.center, self.zoom);
            let angle = -player.rot;
            let tip = pos + rotate(Vec2::new(0.0, -12.0), angle);
            let right = pos + rotate(Vec2::new(8.0, 9.0), angle);
            let left = pos + rotate(Vec2::new(-8.0, 9.0), angle);
            painter.add(egui::Shape::convex_polygon(
                vec![tip, right, left],
                Color32::from_rgb(81, 183, 255),
                Stroke::NONE,
            ));
        }
    }

    fn draw_grid(&self, rect: Rect, painter: &egui::Painter) {
        let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 20);
        let step = (250.0 * self.zoom).max(32.0);
        let origin_x = rect.center().x - self.center.0 * self.zoom;
        let origin_y = rect.center().y + self.center.1 * self.zoom;
        let first_x = rect.left() + (origin_x - rect.left()).rem_euclid(step);
        let first_y = rect.top() + (origin_y - rect.top()).rem_euclid(step);

        let mut x = first_x;
        while x <= rect.right() {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, grid_color),
            );
            x += step;
        }

        let mut y = first_y;
        while y <= rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, grid_color),
            );
            y += step;
        }
    }
}

#[allow(deprecated)]
impl eframe::App for MinimapApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_events();
        let ctx = root_ui.ctx().clone();

        egui::TopBottomPanel::top("top").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Dark December Minimap");
                ui.separator();
                ui.label(&self.status);
                ui.separator();
                ui.monospace(format!(
                    "packets {} | frames {} | player {} | entities {}",
                    self.stats.packets,
                    self.stats.app_frames,
                    self.stats.player_updates,
                    self.stats.entity_updates
                ));
            });
        });

        egui::TopBottomPanel::bottom("bottom").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Center").clicked() {
                    if let Some(player) = self.player {
                        self.center = (player.x, player.z);
                    }
                }
                if ui.button("Clear trails").clicked() {
                    self.player_trail.clear();
                    self.entity_trails.clear();
                }
                ui.checkbox(&mut self.follow, "Follow");
                ui.checkbox(&mut self.trails, "Trails");
                ui.add(egui::Slider::new(&mut self.zoom, 0.03..=2.5).text("zoom"));
                ui.add(egui::Slider::new(&mut self.active_window, 1.0..=30.0).text("active s"));
                let active = active_count(self.player, &self.entities, self.active_window);
                ui.separator();
                ui.label(format!("{active}/{} active", self.entities.len()));
            });
        });

        egui::CentralPanel::default().show(&ctx, |ui| self.draw_minimap(ui));
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_args()?;
    if config.list_devices {
        return list_devices();
    }

    let (tx, rx) = mpsc::channel();
    start_capture_thread(config, tx);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1120.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Dark December Minimap",
        native_options,
        Box::new(|_cc| Ok(Box::new(MinimapApp::new(rx)))),
    )?;
    Ok(())
}

fn parse_args() -> Result<Config, Box<dyn Error>> {
    let mut config = Config {
        iface: None,
        port: 10001,
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
        "darkdec-egui-minimap\n\
         \n\
         --list-devices\n\
         --iface <name-or-description>\n\
         --port <port>                    default: 10001\n\
         --no-promisc\n\
         --offline-stream-dir <path>\n\
         --replay-ms <milliseconds>"
    );
}

fn start_capture_thread(config: Config, tx: Sender<UiEvent>) {
    thread::spawn(move || {
        let result = if let Some(dir) = config.offline_stream_dir.clone() {
            run_offline_replay(dir, config.replay_ms, tx.clone())
        } else {
            run_live_capture(config, tx.clone())
        };
        if let Err(error) = result {
            let _ = tx.send(UiEvent::Status(format!("error: {error}")));
        }
    });
}

fn run_offline_replay(
    dir: PathBuf,
    replay_ms: u64,
    tx: Sender<UiEvent>,
) -> Result<(), Box<dyn Error>> {
    let s2c = std::fs::read(dir.join("first_quest_s2c.tcpstream.bin"))?;
    let frames = frames_from_ordered_stream(Direction::S2C, &s2c);
    let mut stats = Stats::default();
    let mut t = 0.0;
    tx.send(UiEvent::Status(format!("offline frames: {}", frames.len())))?;

    for frame in frames {
        stats.app_frames += 1;
        if let Some(update) = decode_frame(&frame, t) {
            match &update {
                DecodedUpdate::Player(_) => stats.player_updates += 1,
                DecodedUpdate::Entity(_) => stats.entity_updates += 1,
            }
            tx.send(UiEvent::Update(update))?;
            tx.send(UiEvent::Stats(clone_stats(&stats)))?;
            if replay_ms > 0 {
                thread::sleep(Duration::from_millis(replay_ms));
            }
        }
        t += 0.025;
    }
    tx.send(UiEvent::Status("offline replay complete".to_string()))?;
    Ok(())
}

#[cfg(feature = "live-pcap")]
fn run_live_capture(config: Config, tx: Sender<UiEvent>) -> Result<(), Box<dyn Error>> {
    use darkdec_minimap_sniffer::packet::parse_ethernet_ipv4_tcp;
    use pcap::{Capture, Error as PcapError};

    let device = select_device(config.iface.as_deref())?;
    tx.send(UiEvent::Status(format!("capturing {}", device.name)))?;

    let mut capture = Capture::from_device(device)?
        .promisc(!config.no_promisc)
        .snaplen(65535)
        .timeout(50)
        .open()?;
    capture.filter(&format!("tcp port {}", config.port), true)?;

    let start = Instant::now();
    let mut reassembler = TcpReassembler::default();
    let mut stats = Stats::default();

    loop {
        let packet = match capture.next_packet() {
            Ok(packet) => packet,
            Err(PcapError::TimeoutExpired) => continue,
            Err(error) => return Err(error.into()),
        };
        stats.packets += 1;

        let Some(tcp) = parse_ethernet_ipv4_tcp(packet.data, config.port) else {
            continue;
        };
        stats.tcp_payloads += 1;

        let now = start.elapsed().as_secs_f64();
        for frame in reassembler.feed(&tcp) {
            stats.app_frames += 1;
            if let Some(update) = decode_frame(&frame, now) {
                match &update {
                    DecodedUpdate::Player(_) => stats.player_updates += 1,
                    DecodedUpdate::Entity(_) => stats.entity_updates += 1,
                }
                tx.send(UiEvent::Update(update))?;
            }
        }

        if stats.packets % 50 == 0 {
            tx.send(UiEvent::Stats(clone_stats(&stats)))?;
        }
    }
}

#[cfg(not(feature = "live-pcap"))]
fn run_live_capture(_config: Config, _tx: Sender<UiEvent>) -> Result<(), Box<dyn Error>> {
    Err("live pcap support is disabled; build without --no-default-features".into())
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

fn push_limited<T>(items: &mut VecDeque<T>, item: T, max_len: usize) {
    items.push_back(item);
    while items.len() > max_len {
        items.pop_front();
    }
}

fn draw_trail(
    rect: Rect,
    painter: &egui::Painter,
    points: &VecDeque<Point>,
    center: (f32, f32),
    zoom: f32,
    stroke: Stroke,
) {
    if points.len() < 2 {
        return;
    }
    let positions = points
        .iter()
        .map(|point| to_screen(rect, point.x, point.z, center, zoom))
        .collect::<Vec<_>>();
    painter.add(egui::Shape::line(positions, stroke));
}

fn to_screen(rect: Rect, x: f32, z: f32, center: (f32, f32), zoom: f32) -> Pos2 {
    Pos2::new(
        rect.center().x + (x - center.0) * zoom,
        rect.center().y - (z - center.1) * zoom,
    )
}

fn rotate(point: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(point.x * c - point.y * s, point.x * s + point.y * c)
}

fn active_count(player: Option<Point>, entities: &BTreeMap<u8, EntityMarker>, window: f64) -> usize {
    let now = player.map(|point| point.t).unwrap_or(0.0);
    entities
        .values()
        .filter(|entity| (now - entity.point.t).max(0.0) <= window)
        .count()
}

fn clone_stats(stats: &Stats) -> Stats {
    Stats {
        packets: stats.packets,
        tcp_payloads: stats.tcp_payloads,
        app_frames: stats.app_frames,
        player_updates: stats.player_updates,
        entity_updates: stats.entity_updates,
        bad_frames: stats.bad_frames,
    }
}
