//! Local HTTP + WebSocket server. Serves the overlay UI (also usable as OBS
//! browser sources) and streams topics over `/ws`.

use crate::config::{Config, ConfigStore};
use crate::hub::{Command, HostEvent, Hub};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;

#[derive(RustEmbed)]
#[folder = "../../ui/dist"]
struct Assets;

pub const KEEP_TOKEN: &str = "__keep__";

#[derive(Clone)]
pub struct AppState {
    pub hub: Arc<Hub>,
    pub config: ConfigStore,
    pub ui_dir: Option<PathBuf>,
    /// User data directory; `<data>/logos` holds optional manufacturer logo images.
    pub data_dir: PathBuf,
}

/// The Le Mans Ultimate track maps built on this PC, for the shared map library
/// (data/trackmaps in the repository): `{"maps": [...]}`.
async fn lmu_track_maps(s: State<AppState>) -> axum::Json<serde_json::Value> {
    let dir = s.data_dir.join("trackmaps");
    let maps: Vec<crate::engine::trackmap::TrackMap> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|f| serde_json::from_str::<crate::engine::trackmap::TrackMap>(&std::fs::read_to_string(f.path()).ok()?).ok())
        .filter(|m| m.source == "lmu" && !crate::source::lmu::is_iracing_track(m.track_id))
        .collect();
    axum::Json(serde_json::json!({ "maps": maps }))
}

#[derive(serde::Deserialize)]
struct MarketQuery {
    #[serde(default)]
    refresh: bool,
}

/// The Layout Market's list, read from the repository (see market.rs).
async fn market_index(q: axum::extract::Query<MarketQuery>) -> axum::response::Response {
    use axum::response::IntoResponse;
    let refresh = q.refresh;
    match tokio::task::spawn_blocking(move || crate::market::index(refresh)).await {
        Ok(Ok(body)) => ([(axum::http::header::CONTENT_TYPE, "application/json")], body).into_response(),
        Ok(Err(e)) => (axum::http::StatusCode::BAD_GATEWAY, e).into_response(),
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn market_layout(Path(id): Path<String>) -> axum::response::Response {
    use axum::response::IntoResponse;
    match tokio::task::spawn_blocking(move || crate::market::layout(&id)).await {
        Ok(Ok(Some(body))) => ([(axum::http::header::CONTENT_TYPE, "application/json")], body).into_response(),
        Ok(Ok(None)) => axum::http::StatusCode::NOT_FOUND.into_response(),
        Ok(Err(e)) => (axum::http::StatusCode::BAD_GATEWAY, e).into_response(),
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|s: State<AppState>| async move { asset(&s, "control.html") }))
        .route("/o/{id}", get(|s: State<AppState>, _: Path<String>| async move { asset(&s, "overlay.html") }))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/status", get(|s: State<AppState>| async move { s.hub.latest("status").map(|m| m.to_string()).unwrap_or_default() }))
        .route("/ws", get(ws))
        .route("/logos/{file}", get(logo))
        .route("/make-logo/{name}", get(make_logo))
        .route("/api/session-info", get(session_info))
        .route("/api/lmu-track-maps", get(lmu_track_maps))
        .route("/api/market", get(market_index))
        .route("/api/market/{id}", get(market_layout))
        .route("/api/log", get(log_file))
        .route("/api/team-logo", axum::routing::post(upload_team_logo).layer(axum::extract::DefaultBodyLimit::max(TEAM_LOGO_MAX)))
        .route("/team-logos/{file}", get(team_logo))
        .fallback(|s: State<AppState>, uri: Uri| async move { asset(&s, uri.path().trim_start_matches('/')) })
        .with_state(state)
}

/// The end of the log file (the last ~300 KB) and where it is, for Settings → General → Logs.
async fn log_file(s: State<AppState>) -> Response {
    let dir = crate::logging::dir(&s.data_dir);
    let body = serde_json::json!({
        "folder": dir.display().to_string(),
        "file": crate::logging::FILE_NAME,
        "previous": dir.join(crate::logging::PREVIOUS_NAME).exists(),
        "text": crate::logging::tail(&s.data_dir, 300_000),
    });
    ([(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")], body.to_string()).into_response()
}

fn asset(s: &AppState, path: &str) -> Response {
    let path = if path.is_empty() { "control.html" } else { path };
    if path.contains("..") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let mime = mime_for(path);
    if let Some(dir) = &s.ui_dir {
        if let Ok(bytes) = std::fs::read(dir.join(path)) {
            return ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, "no-cache")], bytes).into_response();
        }
    }
    match Assets::get(path) {
        Some(f) => ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, "no-cache")], f.data.into_owned()).into_response(),
        None if path.ends_with(".html") => (StatusCode::NOT_FOUND, "UI not built: run `npm run build` in ui/").into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// The raw session string from iRacing, for troubleshooting (e.g. which flair fields it sends).
async fn session_info(State(s): State<AppState>) -> Response {
    let y = s.hub.session_yaml.lock().unwrap().clone();
    if y.is_empty() {
        return (StatusCode::NOT_FOUND, "No session yet - join a session in iRacing first").into_response();
    }
    ([(header::CONTENT_TYPE, "text/yaml; charset=utf-8"), (header::CONTENT_DISPOSITION, "attachment; filename=\"apex-session-info.yaml\"")], y).into_response()
}

/// Team logos (Team builder) are files in `<data dir>/team-logos`, named after their content
/// so the same image is stored once and can be cached for good.
const TEAM_LOGO_MAX: usize = 8 * 1024 * 1024;

/// The kind of image in `bytes` (png, jpg, webp or svg), from its first bytes.
pub fn image_kind(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]).to_lowercase();
        (head.trim_start().starts_with('<') && head.contains("<svg")).then_some("svg")
    }
}

/// Stores a team logo; answers `{"file": "<name>"}` for the league's team.
async fn upload_team_logo(State(s): State<AppState>, body: axum::body::Bytes) -> Response {
    use sha2::{Digest, Sha256};
    let Some(ext) = image_kind(&body) else {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Not a PNG, JPEG, WebP or SVG image").into_response();
    };
    let hash = Sha256::digest(&body);
    let file = format!("{}.{ext}", hash.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
    let dir = s.data_dir.join("team-logos");
    let path = dir.join(&file);
    if !path.exists() {
        if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &body)) {
            log::error!("could not save team logo {}: {e}", path.display());
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Could not save the logo: {e}")).into_response();
        }
    }
    ([(header::CONTENT_TYPE, "application/json")], serde_json::json!({ "file": file }).to_string()).into_response()
}

/// A team logo by its file name (only names the upload makes).
async fn team_logo(State(s): State<AppState>, Path(file): Path<String>) -> Response {
    let ok =
        file.split_once('.').is_some_and(|(h, e)| h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()) && matches!(e, "png" | "jpg" | "webp" | "svg"));
    if !ok {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match std::fs::read(s.data_dir.join("team-logos").join(&file)) {
        // An SVG opened on its own runs no scripts.
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, mime_for(&file)),
                (header::CACHE_CONTROL, "max-age=31536000, immutable"),
                (header::CONTENT_SECURITY_POLICY, "script-src 'none'"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// User-supplied manufacturer logos: `<data dir>/logos/<make>.svg|png|webp`.
async fn logo(State(s): State<AppState>, Path(file): Path<String>) -> Response {
    if file.contains("..") || file.contains('/') || file.contains('\\') {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match std::fs::read(s.data_dir.join("logos").join(&file)) {
        Ok(bytes) => ([(header::CONTENT_TYPE, mime_for(&file)), (header::CACHE_CONTROL, "max-age=300")], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// A manufacturer's logo in one request: the user's own image in `<data dir>/logos`
/// (`<name>.svg|png|webp`) if there is one, otherwise the one bundled with the app.
async fn make_logo(State(s): State<AppState>, Path(name): Path<String>) -> Response {
    let slug: String = name.to_lowercase().split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-");
    if slug.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    for ext in ["svg", "png", "webp"] {
        let file = format!("{slug}.{ext}");
        if let Ok(bytes) = std::fs::read(s.data_dir.join("logos").join(&file)) {
            return ([(header::CONTENT_TYPE, mime_for(&file)), (header::CACHE_CONTROL, "max-age=300")], bytes).into_response();
        }
    }
    asset(&s, &format!("assets/makes/{slug}.png"))
}

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

/// Config as sent to browsers: the Garage 61 token is never echoed back.
pub fn redacted(c: &Config) -> serde_json::Value {
    let mut v = serde_json::to_value(c).unwrap_or_default();
    for p in ["/garage61/token", "/iracingApi/clientSecretMasked", "/iracingApi/passwordMasked"] {
        if let Some(t) = v.pointer_mut(p) {
            if t.as_str().is_some_and(|s| !s.is_empty()) {
                *t = KEEP_TOKEN.into();
            }
        }
    }
    v
}

fn merge_incoming(store: &ConfigStore, mut incoming: Config) {
    let cur = store.get();
    if incoming.garage61.token == KEEP_TOKEN {
        incoming.garage61.token = cur.garage61.token;
    }
    let api = &mut incoming.iracing_api;
    if api.client_secret_masked == KEEP_TOKEN {
        api.client_secret_masked = cur.iracing_api.client_secret_masked;
    }
    if api.password_masked == KEEP_TOKEN {
        api.password_masked = cur.iracing_api.password_masked;
    }
    store.replace(incoming);
}

async fn get_config(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(s.config.read(redacted))
}

async fn put_config(State(s): State<AppState>, Json(c): Json<Config>) -> StatusCode {
    merge_incoming(&s.config, c);
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum ClientMsg {
    Sub {
        topics: Vec<String>,
    },
    Unsub {
        topics: Vec<String>,
    },
    SetEditMode {
        value: bool,
    },
    ToggleEditMode,
    SetVisible {
        value: bool,
    },
    Drag {
        id: String,
    },
    Resize {
        id: String,
        dir: String,
    },
    SetConfig {
        config: Box<Config>,
    },
    PatchOverlay {
        id: String,
        patch: serde_json::Value,
    },
    ResetTrackMap,
    OpenSettings,
    /// Show the folder with the log files (desktop app).
    OpenLogs,
    /// Open one of the app's repository pages in the browser (desktop app).
    OpenUrl {
        url: String,
    },
    /// An error (or warning) in an overlay or settings page, for the log file.
    Log {
        #[serde(default)]
        level: String,
        #[serde(default)]
        source: String,
        #[serde(default)]
        message: String,
    },
    /// Plain-text secrets are masked immediately and never stored.
    SetIracingApi {
        client_id: String,
        client_secret: String,
        username: String,
        password: String,
    },
    ImportLeague {
        league_id: i64,
        season_id: Option<i64>,
    },
    AutoSize {
        id: String,
        /// 0 = the user sets this dimension.
        #[serde(default)]
        width: f64,
        #[serde(default)]
        height: f64,
    },
    CheckUpdate,
    InstallUpdate,
    /// Update pop-up: remind me in `days` days, or skip this version when `days` is absent.
    PostponeUpdate {
        version: String,
        days: Option<u32>,
    },
    ClosePrompt,
    /// Closing the settings window: quit everything or keep the overlays; optionally remembered.
    CloseApp {
        quit: bool,
        #[serde(default)]
        remember: bool,
    },
    /// Jump the iRacing replay to this moment (only works while a replay is playing).
    Replay {
        session_num: i32,
        session_time: f64,
    },
    /// Let an overlay window take mouse clicks (e.g. replay buttons) or go back to click-through.
    SetInteractive {
        id: String,
        value: bool,
    },
}

async fn ws(upgrade: WebSocketUpgrade, State(s): State<AppState>) -> Response {
    upgrade.on_upgrade(move |socket| client(socket, s))
}

async fn client(mut socket: WebSocket, s: AppState) {
    s.hub.clients.fetch_add(1, Ordering::Relaxed);
    let mut rx = s.hub.receiver();
    let mut subs: HashSet<String> = HashSet::new();
    loop {
        tokio::select! {
            msg = socket.recv() => {
                let text = match msg {
                    Some(Ok(Message::Text(t))) => t,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => continue,
                };
                let Ok(m) = serde_json::from_str::<ClientMsg>(&text) else { continue };
                let mut send_now: Vec<Arc<str>> = vec![];
                handle(&s, m, &mut subs, &mut send_now);
                for m in send_now {
                    if socket.send(Message::Text(m.to_string().into())).await.is_err() {
                        break;
                    }
                }
            }
            r = rx.recv() => match r {
                Ok((topic, msg)) => {
                    if subs.contains(&*topic) {
                        if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                            break;
                        }
                        s.hub.stats.bytes_sent.fetch_add(msg.len() as u64, Ordering::Relaxed);
                        s.hub.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    // Slow client: skip ahead and send the newest state of everything it watches.
                    for t in &subs {
                        if let Some(m) = s.hub.latest(t) {
                            let _ = socket.send(Message::Text(m.to_string().into())).await;
                        }
                    }
                }
                Err(RecvError::Closed) => break,
            }
        }
    }
    for t in &subs {
        s.hub.unsubscribe_topic(t);
    }
    s.hub.clients.fetch_sub(1, Ordering::Relaxed);
}

fn handle(s: &AppState, m: ClientMsg, subs: &mut HashSet<String>, send_now: &mut Vec<Arc<str>>) {
    match m {
        ClientMsg::Sub { topics } => {
            for t in topics {
                if subs.insert(t.clone()) {
                    s.hub.subscribe_topic(&t);
                    if let Some(m) = s.hub.latest(&t) {
                        send_now.push(m);
                    }
                }
            }
        }
        ClientMsg::Unsub { topics } => {
            for t in topics {
                if subs.remove(&t) {
                    s.hub.unsubscribe_topic(&t);
                }
            }
        }
        ClientMsg::SetEditMode { value } => s.hub.set_edit_mode(value),
        ClientMsg::ToggleEditMode => s.hub.set_edit_mode(!s.hub.edit_mode.load(Ordering::Relaxed)),
        ClientMsg::SetVisible { value } => s.hub.set_visible(value),
        ClientMsg::Drag { id } => s.hub.host(HostEvent::Drag { id }),
        ClientMsg::Resize { id, dir } => s.hub.host(HostEvent::Resize { id, dir }),
        ClientMsg::SetConfig { config } => merge_incoming(&s.config, *config),
        ClientMsg::PatchOverlay { id, patch } => s.config.update(|c| {
            if let Some(o) = c.overlays.iter_mut().find(|o| o.id == id) {
                let mut v = serde_json::to_value(&*o).unwrap_or_default();
                merge_json(&mut v, &patch);
                if let Ok(n) = serde_json::from_value(v) {
                    *o = n;
                }
            }
        }),
        ClientMsg::ResetTrackMap => s.hub.command(Command::ResetTrackMap),
        ClientMsg::Replay { session_num, session_time } => s.hub.command(Command::Replay { session_num, session_time }),
        ClientMsg::SetInteractive { id, value } => s.hub.host(HostEvent::Interactive { id, value }),
        ClientMsg::OpenSettings => s.hub.host(HostEvent::OpenSettings),
        ClientMsg::OpenLogs => s.hub.host(HostEvent::OpenFolder { path: crate::logging::dir(&s.data_dir) }),
        ClientMsg::OpenUrl { url } => {
            if crate::market::may_open(&url) {
                s.hub.host(HostEvent::OpenUrl { url });
            } else {
                log::warn!("not opening {url}: not one of the app's pages");
            }
        }
        ClientMsg::Log { level, source, message } => {
            // Pages send their own errors: kept short, one line per report.
            let cut = |t: &str, n: usize| t.chars().take(n).collect::<String>();
            let (source, message) = (cut(&source, 60), cut(&message, 4000));
            if level == "warn" {
                log::warn!(target: "ui", "{source}: {message}");
            } else {
                log::error!(target: "ui", "{source}: {message}");
            }
        }
        ClientMsg::SetIracingApi { client_id, client_secret, username, password } => s.config.update(|c| {
            let api = &mut c.iracing_api;
            let username = username.trim().to_string();
            let client_id = client_id.trim().to_string();
            if !client_id.is_empty() {
                api.client_id = client_id;
            }
            if !username.is_empty() {
                api.username = username;
            }
            if !client_secret.is_empty() {
                api.client_secret_masked = crate::iracing_api::mask(&client_secret, &api.client_id);
            }
            if !password.is_empty() {
                api.password_masked = crate::iracing_api::mask(&password, &api.username);
            }
        }),
        ClientMsg::ImportLeague { league_id, season_id } => import_league(s.clone(), league_id, season_id),
        ClientMsg::AutoSize { id, width, height } => s.hub.host(HostEvent::AutoSize { id, width, height }),
        ClientMsg::CheckUpdate | ClientMsg::InstallUpdate if !s.hub.updates_supported.load(Ordering::Relaxed) => {
            s.hub.publish("update", r#"{"status":"unsupported","message":"Updates are installed by the Apex Overlays desktop app"}"#);
        }
        ClientMsg::CheckUpdate => s.hub.host(HostEvent::CheckUpdate),
        ClientMsg::PostponeUpdate { version, days } => {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
            let until = days.map(|d| now + d as i64 * 86_400);
            s.config.update(|c| c.update_postpone = Some(crate::config::UpdatePostpone { version, until }));
            s.hub.host(HostEvent::ClosePrompt);
        }
        ClientMsg::ClosePrompt => s.hub.host(HostEvent::ClosePrompt),
        ClientMsg::CloseApp { quit, remember } => {
            if remember {
                s.config.update(|c| c.close_action = if quit { "quit" } else { "keep" }.into());
            }
            s.hub.host(HostEvent::CloseApp { quit });
        }
        ClientMsg::InstallUpdate => s.hub.host(HostEvent::InstallUpdate),
    }
}

/// Imports a league in the background, reporting progress on the `leagueImport` topic.
fn import_league(s: AppState, league_id: i64, season_id: Option<i64>) {
    let status = move |hub: &Hub, state: &str, msg: &str, seasons: &[crate::iracing_api::SeasonSummary], id: Option<&str>| {
        let v = serde_json::json!({ "status": state, "message": msg, "leagueId": league_id, "seasons": seasons, "id": id });
        hub.publish("leagueImport", &v.to_string());
    };
    status(&s.hub, "working", "Contacting iRacing…", &[], None);
    std::thread::spawn(move || {
        let mut client = crate::iracing_api::Client::new(s.config.read(|c| c.iracing_api.clone()));
        match crate::iracing_api::import(&mut client, league_id, season_id) {
            Ok((mut league, seasons)) => {
                league.imported_at = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
                let id = league.id.clone();
                let msg = format!(
                    "Imported {}: {} classes, {} drivers, {} points positions, {} drivers in the standings",
                    league.name,
                    league.classes.len(),
                    league.driver_overrides.len(),
                    league.points.table.len(),
                    league.season_points.len()
                );
                s.config.update(|c| match c.leagues.iter_mut().find(|l| l.iracing_league_id == league.iracing_league_id && league.iracing_league_id != 0) {
                    // Re-import: refresh data but keep the user's colours, class names and edits.
                    Some(existing) => {
                        for class in &mut league.classes {
                            if let Some(old) = existing.classes.iter().find(|o| o.id == class.id) {
                                class.color = old.color.clone();
                                class.name = old.name.clone();
                            }
                        }
                        // Secondary classes are made here, not in iRacing: kept.
                        league.classes.extend(existing.classes.iter().filter(|c| c.is_secondary()).cloned());
                        // Keep everything the user set on the roster; fill in names/numbers from iRacing.
                        let imported = std::mem::take(&mut league.driver_overrides);
                        league.driver_overrides = existing.driver_overrides.clone();
                        for n in imported {
                            match league.driver_overrides.iter_mut().find(|o| o.customer_id == n.customer_id) {
                                Some(o) => {
                                    if o.name.is_empty() {
                                        o.name = n.name;
                                    }
                                    if o.car_number.is_empty() {
                                        o.car_number = n.car_number;
                                    }
                                    if o.display_name.is_empty() {
                                        o.display_name = n.display_name;
                                    }
                                }
                                None => league.driver_overrides.push(n),
                            }
                        }
                        // Teams are built here (Team builder), not in iRacing.
                        league.teams = existing.teams.clone();
                        if league.points.table.is_empty() {
                            league.points.table = existing.points.table.clone();
                        }
                        league.points.basis = existing.points.basis.clone();
                        league.points.fastest_lap_bonus = existing.points.fastest_lap_bonus;
                        league.points.pole_bonus = existing.points.pole_bonus;
                        league.points.lap_led_bonus = existing.points.lap_led_bonus;
                        league.points.most_laps_led_bonus = existing.points.most_laps_led_bonus;
                        league.points.finish_bonus = existing.points.finish_bonus;
                        league.points.default_points = existing.points.default_points;
                        league.fallback = existing.fallback.clone();
                        league.strip_name_class = existing.strip_name_class;
                        league.id = existing.id.clone();
                        *existing = league;
                    }
                    None => c.leagues.push(league),
                });
                status(&s.hub, "ok", &msg, &seasons, Some(&id));
            }
            Err(e) => status(&s.hub, "error", &e, &[], None),
        }
    });
}

/// RFC 7386-style merge patch.
pub fn merge_json(target: &mut serde_json::Value, patch: &serde_json::Value) {
    match (target, patch) {
        (serde_json::Value::Object(t), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                if v.is_null() {
                    t.remove(k);
                } else {
                    merge_json(t.entry(k.clone()).or_insert(serde_json::Value::Null), v);
                }
            }
        }
        (t, p) => *t = p.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::result_large_err)]
    fn team_logos_are_stored_by_content_and_served() {
        let dir = std::env::temp_dir().join(format!("apex-team-logos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let app = crate::start(crate::Options { data_dir: dir.clone(), mock: true, ui_dir: None, port: Some(0) }).unwrap();
        let base = format!("http://127.0.0.1:{}", app.addr.port());
        let png: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR fake";
        let up = |body: &[u8]| ureq::post(&format!("{base}/api/team-logo")).send_bytes(body);
        let file = up(png).unwrap().into_json::<serde_json::Value>().unwrap()["file"].as_str().unwrap().to_string();
        assert!(file.ends_with(".png") && file.len() == 20, "{file}");
        // The same image again: the same file.
        assert_eq!(up(png).unwrap().into_json::<serde_json::Value>().unwrap()["file"], file.as_str());
        let r = ureq::get(&format!("{base}/team-logos/{file}")).call().unwrap();
        assert_eq!(r.header("content-type"), Some("image/png"));
        let mut got = vec![];
        std::io::Read::read_to_end(&mut r.into_reader(), &mut got).unwrap();
        assert_eq!(got, png);
        // Not an image, or a name the upload does not make: refused.
        assert!(matches!(up(b"hello"), Err(ureq::Error::Status(415, _))));
        assert!(matches!(ureq::get(&format!("{base}/team-logos/config.json")).call(), Err(ureq::Error::Status(400, _))));
        let svg = up(br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"/>"#).unwrap().into_json::<serde_json::Value>().unwrap();
        let svg = svg["file"].as_str().unwrap();
        let r = ureq::get(&format!("{base}/team-logos/{svg}")).call().unwrap();
        assert_eq!(r.header("content-security-policy"), Some("script-src 'none'"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn merge_patch() {
        let mut a = serde_json::json!({"x": 1, "s": {"a": 1, "b": 2}});
        merge_json(&mut a, &serde_json::json!({"s": {"b": null, "c": 3}, "y": true}));
        assert_eq!(a, serde_json::json!({"x": 1, "s": {"a": 1, "c": 3}, "y": true}));
    }

    #[test]
    fn token_is_redacted_and_kept() {
        let store = ConfigStore::new(Config::default(), None);
        store.update(|c| {
            c.garage61.token = "secret".into();
            c.iracing_api.password_masked = "hash".into();
        });
        let red = store.read(redacted);
        assert_eq!(red["garage61"]["token"], KEEP_TOKEN);
        let incoming: Config = serde_json::from_value(red).unwrap();
        merge_incoming(&store, incoming);
        assert_eq!(store.read(|c| c.garage61.token.clone()), "secret");
        assert_eq!(store.read(|c| c.iracing_api.password_masked.clone()), "hash");
    }
}
