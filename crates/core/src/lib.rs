//! Apex Overlays core: reads telemetry (live iRacing or the built-in mock),
//! derives race information at the simulator's 60 Hz, and serves it to
//! overlays over a local WebSocket.

pub mod cardata;
pub mod config;
pub mod corners;
pub mod countries;
pub mod engine;
pub mod garage61;
pub mod hub;
pub mod iracing_api;
pub mod league;
pub mod lookup;
pub mod model;
pub mod monitor;
pub mod net;
pub mod safety;
pub mod server;
pub mod source;
pub mod svgmap;
pub mod topics;
pub mod vr;

use config::ConfigStore;
use engine::Engine;
use hub::{Command, Hub};
use model::SessionInfo;
use source::{Poll, Source};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex};
use topics::Topic;

pub struct Options {
    pub data_dir: PathBuf,
    /// Use the built-in simulator instead of iRacing.
    pub mock: bool,
    /// Serve UI files from this directory first (development / custom overlays).
    pub ui_dir: Option<PathBuf>,
    /// Override the configured port.
    pub port: Option<u16>,
}

pub struct App {
    pub hub: Arc<Hub>,
    pub config: ConfigStore,
    pub addr: SocketAddr,
    pub data_dir: PathBuf,
}

impl App {
    /// URL of the control panel / settings page.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.addr.port())
    }

    /// URL of a single overlay (also usable as an OBS browser source).
    pub fn overlay_url(&self, id: &str) -> String {
        format!("http://127.0.0.1:{}/o/{id}", self.addr.port())
    }
}

/// Start the telemetry thread, the Garage 61 worker and the HTTP server.
pub fn start(opts: Options) -> std::io::Result<App> {
    std::fs::create_dir_all(&opts.data_dir)?;
    let config = ConfigStore::load(&opts.data_dir);
    let hub = Hub::new();
    let (port, lan) = config.read(|c| (opts.port.unwrap_or(c.server.port), c.server.lan_access));
    let ip = if lan { [0, 0, 0, 0] } else { [127, 0, 0, 1] };
    let listener = std::net::TcpListener::bind(SocketAddr::from((ip, port)))?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;

    let state = server::AppState { hub: hub.clone(), config: config.clone(), ui_dir: opts.ui_dir.clone(), data_dir: opts.data_dir.clone() };
    std::thread::Builder::new().name("http".into()).spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio runtime");
        rt.block_on(async move {
            // Mirror config changes to clients (overlays apply settings live).
            let (hub2, cfg2) = (state.hub.clone(), state.config.clone());
            tokio::spawn(async move {
                let mut rx = cfg2.subscribe();
                loop {
                    hub2.publish("config", &cfg2.read(server::redacted).to_string());
                    if rx.changed().await.is_err() {
                        break;
                    }
                }
            });
            let listener = tokio::net::TcpListener::from_std(listener).expect("listener");
            if let Err(e) = axum::serve(listener, server::router(state)).await {
                log::error!("server stopped: {e}");
            }
        });
    })?;

    monitor::spawn(hub.clone());
    let combo = Arc::new(Mutex::new(garage61::Combo::default()));
    garage61::spawn(hub.clone(), config.clone(), combo.clone(), opts.data_dir.clone());
    let track = Arc::new(Mutex::new((0i64, String::new())));
    if !opts.mock {
        corners::spawn(hub.clone(), opts.data_dir.clone(), track.clone());
        svgmap::spawn(hub.clone(), config.clone(), opts.data_dir.clone(), track.clone());
    }

    let (cmd_tx, cmd_rx) = mpsc::channel();
    hub.set_command_sender(cmd_tx);
    let wanted = Arc::new(Mutex::new(Vec::new()));
    countries::spawn(hub.clone(), config.clone(), opts.data_dir.clone(), wanted.clone());
    let wanted_cars = Arc::new(Mutex::new(Vec::new()));
    if !opts.mock {
        cardata::spawn(hub.clone(), opts.data_dir.clone(), wanted_cars.clone());
    }
    let source: Box<dyn Source> = if opts.mock { Box::new(source::mock::Mock::new()) } else { Box::new(source::iracing::IRacing::new()) };
    *hub.source.lock().unwrap() = source.name();
    hub.publish_status();
    hub.publish("update", &serde_json::json!({ "status": "unsupported", "current": env!("CARGO_PKG_VERSION") }).to_string());
    let (h, c, d) = (hub.clone(), config.clone(), opts.data_dir.clone());
    std::thread::Builder::new().name("telemetry".into()).spawn(move || run(source, h, c, combo, track, wanted, wanted_cars, cmd_rx, d))?;

    Ok(App { hub, config, addr, data_dir: opts.data_dir })
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut source: Box<dyn Source>,
    hub: Arc<Hub>,
    config: ConfigStore,
    combo: Arc<Mutex<garage61::Combo>>,
    track: Arc<Mutex<(i64, String)>>,
    wanted_countries: Arc<Mutex<Vec<i64>>>,
    wanted_cars: Arc<Mutex<Vec<String>>>,
    cmds: mpsc::Receiver<Command>,
    data_dir: PathBuf,
) {
    let mut e = Engine::new(config.get(), Some(data_dir));
    e.source = source.name();
    let mut cfg_rx = config.subscribe();
    let mut keys = Keys::default();
    let mut last_session_layout: (i64, i32, &'static str) = (0, -1, "");
    loop {
        if cfg_rx.has_changed().unwrap_or(false) {
            cfg_rx.mark_unchanged();
            e.set_config(config.get());
        }
        while let Ok(cmd) = cmds.try_recv() {
            match cmd {
                Command::SetTrackData { track_id, data, source } => e.set_track_data(track_id, data, source),
                Command::SetCountries(c) => e.set_remote_countries(c),
                Command::SetCarData(d) => e.set_car_data(d),
                Command::SetOfficialMap(m) => e.map.set_official(m),
                Command::SetLibraryMap(m) => e.map.set_library(m),
                Command::SetClassNames(c) => e.set_class_names(c),
                Command::Replay { session_num, session_time } => {
                    let me = e.info.driver(e.info.player_car_idx.max(0) as usize);
                    let car = me.map(|d| d.car_number_raw).filter(|n| *n >= 0);
                    source.replay(session_num, session_time, car, (e.tel.cam_group, e.tel.cam_camera));
                }
                Command::ResetTrackMap => {
                    e.map.clear();
                    if let Some(m) = source.track_map(e.info.track_id) {
                        e.map.set(m);
                    }
                }
            }
        }
        match source.poll(&mut e.tel) {
            Poll::Idle => continue,
            Poll::Disconnected => {
                if e.connected {
                    e.reset();
                    hub.connected.store(false, Ordering::Relaxed);
                    hub.publish_status();
                    publish(&e, &hub, &mut keys, true);
                    *combo.lock().unwrap() = Default::default();
                }
            }
            Poll::Data { session_yaml } => {
                if !e.connected {
                    e.connected = true;
                    hub.connected.store(true, Ordering::Relaxed);
                    hub.publish_status();
                }
                if let Some(y) = session_yaml {
                    e.set_session(SessionInfo::parse(&y));
                    *hub.session_yaml.lock().unwrap() = y;
                    if e.map.map.is_none() {
                        if let Some(m) = source.track_map(e.info.track_id) {
                            e.map.set(m);
                        }
                    }
                    if let Some(c) = source.corners() {
                        let (pit_entry, pit_exit) = source.pit_lane().unzip();
                        let data = corners::TrackData { corners: c, straights: vec![], pit_entry, pit_exit };
                        e.set_track_data(e.info.track_id, data, "mock");
                    }
                    *track.lock().unwrap() = (e.info.track_id, e.info.track_name.clone());
                    let mut cars: Vec<String> =
                        e.info.drivers.iter().filter(|d| !d.is_pace_car && !d.is_spectator).map(|d| d.car_path.to_lowercase()).collect();
                    cars.sort();
                    cars.dedup();
                    *wanted_cars.lock().unwrap() = cars;
                    *wanted_countries.lock().unwrap() =
                        e.info.drivers.iter().filter(|d| d.country.is_empty() && !d.is_ai && !d.is_pace_car && !d.is_spectator).map(|d| d.user_id).collect();
                    let me = e.info.driver(e.info.player_car_idx.max(0) as usize);
                    let mut c = combo.lock().unwrap();
                    *c = garage61::Combo {
                        car_id: me.map(|d| d.car_id).unwrap_or(0),
                        car_name: me.map(|d| d.car_name.clone()).unwrap_or_default(),
                        track_id: e.info.track_id,
                        track_name: e.info.track_display_name.clone(),
                        track_config: e.info.track_config.clone(),
                        ..c.clone()
                    };
                }
                // Conditions for Garage 61's "similar conditions" lap search.
                let (temp, wet) = (e.tel.track_temp.round() as i32, e.tel.track_wetness >= 3);
                if e.tel.tick % 60 == 0 {
                    let mut c = combo.lock().unwrap();
                    if (c.track_temp, c.wet) != (temp, wet) {
                        (c.track_temp, c.wet) = (temp, wet);
                    }
                }
                // Session layouts: when a new event or a different kind of session starts, switch
                // to the layout set for it (once — the user can still pick another by hand).
                let category = e.session_def().map(|s| crate::config::session_category(&s.kind)).unwrap_or("");
                let session_key = (e.info.subsession_id, e.tel.session_num, category);
                if session_key != last_session_layout && !category.is_empty() {
                    if last_session_layout.0 != session_key.0 || last_session_layout.2 != category {
                        config.update(|c| {
                            if let Some(id) = c.layout_for_session(category).filter(|id| *id != c.active_layout) {
                                log::info!("{category} session: switching to layout {id}");
                                c.apply_layout(&id);
                            }
                        });
                    }
                    last_session_layout = session_key;
                }
                let t0 = std::time::Instant::now();
                e.tick();
                publish(&e, &hub, &mut keys, false);
                hub.stats.ticks.fetch_add(1, Ordering::Relaxed);
                hub.stats.tick_ns.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
            }
        }
    }
}

/// Last state of the topics that are only re-sent when they change.
#[derive(Default)]
struct Keys {
    map: Option<(u64, i32, i64)>,
    car: Option<(u64, i64, i32, i32)>,
    delta: Option<u64>,
    /// Ticks published, for topics sent at a lower rate.
    tick: u64,
}

fn publish(e: &Engine, hub: &Hub, keys: &mut Keys, force: bool) {
    keys.tick += 1;
    let rate = e.cfg.update_rate.as_str();
    for (i, t) in Topic::ALL.into_iter().enumerate() {
        if !force && !hub.has_subscribers(t.name()) {
            continue;
        }
        // Spread the slower topics over different ticks.
        let every = match rate {
            "full" => 1,
            "fast" => t.every_fast(),
            _ => t.every(),
        };
        if !force && !(keys.tick + i as u64).is_multiple_of(every) {
            continue;
        }
        if t == Topic::TrackMap {
            // The outline is large and rarely changes: rebuild only when it does.
            let key = (e.map.revision, (e.map.progress() * 100.0) as i32, e.info.track_id);
            if keys.map == Some(key) && !force {
                continue;
            }
            keys.map = Some(key);
        }
        if t == Topic::Delta {
            if keys.delta == Some(e.laps.revision) && !force {
                continue;
            }
            keys.delta = Some(e.laps.revision);
        }
        if t == Topic::Car {
            let key = (e.car_revision, e.info.subsession_id, e.info.player_car_idx, e.tel.session_num);
            if keys.car == Some(key) && !force {
                continue;
            }
            keys.car = Some(key);
        }
        hub.publish(t.name(), &t.build(e));
    }
}
