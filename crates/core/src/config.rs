//! Persistent user configuration. Overlay `settings` are free-form JSON owned by
//! the UI (each overlay declares its own schema and defaults), so new overlay
//! options never need a backend change. Settings that drive server-side
//! calculations (fuel, tyres, leagues, ...) are typed here.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::watch;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub version: u32,
    pub server: ServerConfig,
    /// "auto" follows the simulator's display units.
    pub units: String,
    pub theme: serde_json::Value,
    pub overlays: Vec<OverlayInstance>,
    /// Saved layouts, and the one in use ("" = none).
    pub layouts: Vec<Layout>,
    pub active_layout: String,
    pub hotkeys: Hotkeys,
    pub vr: VrConfig,
    pub fuel: FuelConfig,
    pub tyres: TyreConfig,
    pub battle: BattleConfig,
    pub leagues: Vec<League>,
    pub active_league: Option<String>,
    /// "auto": use the league whose iRacing league ID matches the session (none otherwise);
    /// "always": always use `active_league`, whatever the session.
    pub league_mode: String,
    pub garage61: Garage61Config,
    /// Colour themes the user saved (name + theme values), shared by overlays and the settings window.
    pub saved_themes: Vec<serde_json::Value>,
    /// User-edited corner names per iRacing TrackID; overrides downloaded corner data.
    pub track_corners: std::collections::BTreeMap<String, Vec<Corner>>,
    pub iracing_api: IracingApiConfig,
    /// Only show iRating / SR estimates in official (ranked) sessions.
    pub ratings_official_only: bool,
    /// Desktop app: look for a new version on start-up.
    pub auto_check_updates: bool,
    /// Penalties handed out by race control (shown next to the driver's name).
    pub penalties: Vec<Penalty>,
    /// Edit mode: dragging any overlay moves all of them.
    pub move_all_together: bool,
    /// Short class names (GT3, TCR, LMP2...) from Lovely Car Classes / Car Data.
    pub short_class_names: bool,
    /// Track map outline: "iracing" (iRacing's official map when available) or "recorded".
    pub track_outline: String,
    /// How often live data is sent to overlays: "fast" (nothing above 30/s, least CPU),
    /// "balanced" (timing tables a few times a second, driving aids every tick) or "full"
    /// (everything every tick, as before 0.5.6).
    pub update_rate: String,
    /// Closing the settings window: "ask", "quit" (overlays close too) or "keep" (overlays
    /// keep running, the app stays in the tray).
    pub close_action: String,
    /// "Remind me later" / "skip this version" for the update pop-up.
    pub update_postpone: Option<UpdatePostpone>,
}

/// A penalty given by hand (iRacing does not say which penalty a black flag means).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Penalty {
    pub id: String,
    pub customer_id: i64,
    pub name: String,
    /// "dt" drive-through, "sg" stop & go, "time" added time, "other".
    pub kind: String,
    pub seconds: f32,
    pub note: String,
}

impl Penalty {
    pub fn label(&self) -> String {
        let s = self.seconds.round() as i32;
        match self.kind.as_str() {
            "dt" => "DT".into(),
            "sg" if s > 0 => format!("S&G {s}s"),
            "sg" => "S&G".into(),
            "time" => format!("+{s}s"),
            _ => {
                if self.note.is_empty() {
                    "PEN".into()
                } else {
                    self.note.clone()
                }
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let o = |id: &str, kind: &str, x, y, w, h| OverlayInstance {
            id: id.into(),
            kind: kind.into(),
            name: String::new(),
            enabled: true,
            x,
            y,
            width: w,
            height: h,
            settings: serde_json::json!({}),
        };
        Self {
            version: 1,
            server: ServerConfig::default(),
            units: "auto".into(),
            theme: serde_json::json!({}),
            overlays: vec![
                o("relative", "relative", 40, 380, 460, 300),
                o("standings", "standings", 40, 40, 560, 320),
                o("trackmap", "trackmap", 1500, 40, 340, 340),
                o("inputs", "inputs", 760, 860, 420, 120),
                OverlayInstance { enabled: false, ..o("inputGraph", "inputGraph", 760, 740, 360, 110) },
                o("fuel", "fuel", 1540, 420, 300, 230),
                o("battle", "battle", 1200, 860, 360, 150),
                o("session", "session", 510, 10, 900, 52),
                o("tyres", "tyres", 1540, 680, 300, 150),
                o("flags", "flags", 860, 70, 220, 110),
                o("spotter", "spotter", 850, 520, 220, 200),
                o("pit", "pit", 845, 300, 230, 200),
                o("safety", "safety", 760, 160, 400, 230),
                o("delta", "delta", 760, 400, 420, 96),
                OverlayInstance { enabled: false, ..o("incidents", "incidents", 40, 700, 340, 230) },
                OverlayInstance { enabled: false, ..o("weather", "weather", 1240, 680, 300, 170) },
                OverlayInstance { enabled: false, ..o("garage61", "garage61", 1540, 850, 300, 170) },
            ],
            layouts: vec![],
            active_layout: String::new(),
            hotkeys: Hotkeys::default(),
            vr: VrConfig::default(),
            fuel: FuelConfig::default(),
            tyres: TyreConfig::default(),
            battle: BattleConfig::default(),
            leagues: vec![],
            active_league: None,
            league_mode: "auto".into(),
            garage61: Garage61Config::default(),
            saved_themes: vec![],
            track_corners: Default::default(),
            iracing_api: IracingApiConfig::default(),
            ratings_official_only: true,
            auto_check_updates: true,
            penalties: vec![],
            move_all_together: false,
            short_class_names: true,
            update_postpone: None,
            track_outline: "iracing".into(),
            update_rate: "balanced".into(),
            close_action: "ask".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerConfig {
    pub port: u16,
    /// Listen on all interfaces so a second (streaming) PC can load the overlays.
    pub lan_access: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 8182, lan_access: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct OverlayInstance {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub settings: serde_json::Value,
}

impl Default for OverlayInstance {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: String::new(),
            name: String::new(),
            enabled: true,
            x: 100,
            y: 100,
            width: 400,
            height: 200,
            settings: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Hotkeys {
    pub toggle_edit: String,
    pub toggle_visibility: String,
    /// Switch to the next saved layout ("" = no hotkey).
    pub next_layout: String,
    /// Saved layout id → hotkey that switches to it.
    pub layouts: std::collections::BTreeMap<String, String>,
    /// Overlay id → hotkey that shows / hides that overlay.
    pub overlays: std::collections::BTreeMap<String, String>,
    /// Recentre VR (the seated position the overlays fixed in the car are placed from).
    pub vr_recenter: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            toggle_edit: "CmdOrCtrl+Shift+E".into(),
            toggle_visibility: "CmdOrCtrl+Shift+H".into(),
            next_layout: String::new(),
            layouts: Default::default(),
            overlays: Default::default(),
            vr_recenter: String::new(),
        }
    }
}

/// VR: the overlays shown in the headset as SteamVR overlays (desktop app on Windows).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VrConfig {
    pub enabled: bool,
    /// Overlays placed "as on my screen" sit on an imaginary screen this far in front of the
    /// seated position (m), this wide (m), raised or lowered by `height` (m).
    pub distance: f64,
    pub width: f64,
    pub height: f64,
}

impl Default for VrConfig {
    fn default() -> Self {
        Self { enabled: false, distance: 0.9, width: 1.2, height: -0.05 }
    }
}

/// A saved arrangement of overlays (which ones, where, and their settings).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub overlays: Vec<OverlayInstance>,
    /// Session types this layout is switched to automatically when one starts:
    /// "practice", "qualifying", "warmup", "race".
    pub sessions: Vec<String>,
}

/// The kind of session for automatic layouts, from iRacing's SessionType ("Lone Qualify",
/// "Open Practice", "Race", "Warmup", "Offline Testing"...).
pub fn session_category(kind: &str) -> &'static str {
    let k = kind.to_lowercase();
    if k.contains("race") || k.contains("heat") || k.contains("feature") {
        "race"
    } else if k.contains("qual") {
        "qualifying"
    } else if k.contains("warm") {
        "warmup"
    } else if k.is_empty() {
        ""
    } else {
        "practice"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FuelConfig {
    /// Number of recent valid laps averaged for consumption.
    pub avg_laps: usize,
    /// Extra laps of fuel to carry as safety margin.
    pub margin_laps: f32,
    pub include_caution_laps: bool,
    pub include_pit_laps: bool,
}

impl Default for FuelConfig {
    fn default() -> Self {
        Self { avg_laps: 5, margin_laps: 1.0, include_caution_laps: false, include_pit_laps: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TyreConfig {
    /// TrackWetness level (1 dry .. 7 extremely wet) at which wets are advised.
    pub wet_threshold: i32,
    /// TrackWetness level at or below which slicks are advised again.
    pub dry_threshold: i32,
    /// Use what compound the rest of the class is running.
    pub use_field: bool,
    /// Compare recent lap times of cars on wets vs slicks.
    pub use_crossover: bool,
    /// Seconds a compound must be faster before the crossover counts.
    pub crossover_margin: f32,
}

impl Default for TyreConfig {
    fn default() -> Self {
        Self { wet_threshold: 4, dry_threshold: 2, use_field: true, use_crossover: true, crossover_margin: 0.5 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BattleConfig {
    /// Gaps above this (seconds) are not considered a battle.
    pub max_gap: f32,
}

impl Default for BattleConfig {
    fn default() -> Self {
        Self { max_gap: 3.0 }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct League {
    pub id: String,
    pub name: String,
    pub classes: Vec<LeagueClass>,
    pub driver_overrides: Vec<DriverOverride>,
    /// What to do with cars matching no class: "iracing" keeps the sim class, "other" groups them.
    pub fallback: String,
    /// iRacing league this was imported from (0 = built by hand).
    pub iracing_league_id: i64,
    pub season_id: i64,
    pub season_name: String,
    pub points: PointsConfig,
    /// Championship totals before the current race, from the league's season standings.
    pub season_points: Vec<SeasonEntry>,
    pub imported_at: String,
    /// Leave out the class tags some leagues put in names ("(P) Max Power" shows as
    /// "Max Power").
    pub strip_name_class: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PointsConfig {
    /// Points for P1, P2, ... (anything beyond the table scores `default_points`).
    pub table: Vec<f32>,
    pub default_points: f32,
    /// "class" scores by position in class, "overall" by overall position.
    pub basis: String,
    pub fastest_lap_bonus: f32,
    /// Bonus for the pole sitter (qualifying P1, by class or overall like the points basis).
    pub pole_bonus: f32,
    /// Bonus for leading at least one lap.
    pub lap_led_bonus: f32,
    /// Bonus for leading the most laps.
    pub most_laps_led_bonus: f32,
    /// Points every classified finisher receives (participation).
    pub finish_bonus: f32,
    pub system_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SeasonEntry {
    pub customer_id: i64,
    pub name: String,
    pub points: f32,
    pub position: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Corner {
    pub name: String,
    pub number: i32,
    /// Lap fractions (0..1).
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdatePostpone {
    pub version: String,
    /// Unix time (seconds) until which the pop-up stays away; `None` = skip this version.
    pub until: Option<i64>,
}

impl UpdatePostpone {
    /// The start-up pop-up should stay away for `version` at `now` (unix seconds).
    pub fn holds(&self, version: &str, now: i64) -> bool {
        self.version == version && self.until.is_none_or(|u| now < u)
    }
}

/// Credentials for the iRacing Data API (OAuth "password limited" grant). Secrets are stored
/// masked (base64 SHA-256, as iRacing's token endpoint expects), never in plain text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct IracingApiConfig {
    pub client_id: String,
    pub client_secret_masked: String,
    pub username: String,
    pub password_masked: String,
}

impl IracingApiConfig {
    pub fn configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret_masked.is_empty() && !self.username.is_empty() && !self.password_masked.is_empty()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LeagueClass {
    pub id: String,
    pub name: String,
    /// CSS hex colour, e.g. "#ff4d4d".
    pub color: String,
    pub rules: ClassRules,
}

/// A car joins the class if ANY populated rule matches. Text rules are
/// case-insensitive globs (`*` and `?`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClassRules {
    pub car_ids: Vec<i64>,
    pub car_names: Vec<String>,
    pub car_numbers: Vec<String>,
    pub customer_ids: Vec<i64>,
    pub team_names: Vec<String>,
    pub driver_names: Vec<String>,
    pub iracing_class_ids: Vec<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DriverOverride {
    pub customer_id: i64,
    /// iRacing name as pulled from a session or the league roster (for reference).
    pub name: String,
    /// League car number; replaces the number shown in overlays when set.
    pub car_number: String,
    /// League class this driver races in (a `LeagueClass::id`); takes priority over class rules.
    pub class_id: String,
    pub display_name: String,
    pub team_name: String,
    /// ISO country code shown as the driver's flag, overriding their iRacing flair.
    pub country: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Garage61Config {
    pub enabled: bool,
    /// Personal access token from garage61.net → Developer → My applications.
    pub token: String,
    pub include_team: bool,
    pub refresh_minutes: u32,
    /// Laps searched: "all", "season" (current iRacing season), "seasons2" (current and
    /// previous), or a number of days ("90", "30", "7").
    pub period: String,
    /// Lap to compare with live: "personal" (your best), "team" (fastest in your teams) or "off".
    pub reference: String,
    /// Only laps driven in similar conditions: track temperature within `temp_range` °C and
    /// the same dry / wet track.
    pub match_conditions: bool,
    pub temp_range: f32,
}

impl Default for Garage61Config {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            include_team: true,
            refresh_minutes: 5,
            period: "all".into(),
            reference: "personal".into(),
            match_conditions: false,
            temp_range: 6.0,
        }
    }
}

pub fn default_data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("Apex Overlays");
    // Earlier builds used "ApexOverlays": move existing settings across once.
    let old = base.join("ApexOverlays");
    if !dir.exists() && old.is_dir() {
        let _ = std::fs::rename(&old, &dir);
    }
    dir
}

/// Shared, observable config with atomic persistence.
#[derive(Clone)]
pub struct ConfigStore {
    inner: Arc<RwLock<Config>>,
    path: Option<Arc<PathBuf>>,
    tx: Arc<watch::Sender<u64>>,
}

impl Config {
    /// Switch to a saved layout. The arrangement in use is first kept in the layout it came
    /// from, so moving or changing overlays is never lost by switching.
    pub fn apply_layout(&mut self, id: &str) -> bool {
        if !self.layouts.iter().any(|l| l.id == id) {
            return false;
        }
        let current = self.overlays.clone();
        if let Some(active) = self.layouts.iter_mut().find(|l| l.id == self.active_layout) {
            active.overlays = current.clone();
        }
        if let Some(next) = self.layouts.iter().find(|l| l.id == id) {
            let mut overlays = next.overlays.clone();
            // Overlays the layout does not have come along switched off, with the same id, so
            // their hotkeys work in every layout and they can be switched on there too.
            for o in current.into_iter().filter(|o| !next.overlays.iter().any(|n| n.id == o.id)) {
                overlays.push(OverlayInstance { enabled: false, ..o });
            }
            self.overlays = overlays;
        }
        self.active_layout = id.to_string();
        true
    }

    /// Overlay kinds added in later versions, set up (switched off) in existing configs so
    /// they appear in the overlay list and the hotkeys straight away.
    pub fn add_new_kinds(&mut self) {
        let defaults = Config::default().overlays;
        for kind in ["safety", "delta", "inputGraph"] {
            if self.overlays.iter().any(|o| o.kind == kind) {
                continue;
            }
            let Some(mut o) = defaults.iter().find(|o| o.kind == kind).cloned() else { continue };
            let mut id = kind.to_string();
            let mut n = 2;
            while self.overlays.iter().any(|x| x.id == id) {
                id = format!("{kind}-{n}");
                n += 1;
            }
            o.id = id;
            o.enabled = false;
            self.overlays.push(o);
        }
    }

    /// The saved layout set to switch to for this kind of session.
    pub fn layout_for_session(&self, category: &str) -> Option<String> {
        if category.is_empty() {
            return None;
        }
        self.layouts.iter().find(|l| l.sessions.iter().any(|s| s == category)).map(|l| l.id.clone())
    }

    /// The saved layout after the one in use (wrapping round).
    pub fn next_layout(&self) -> Option<String> {
        let i = self.layouts.iter().position(|l| l.id == self.active_layout).map_or(0, |i| i + 1);
        self.layouts.get(i % self.layouts.len().max(1)).map(|l| l.id.clone())
    }

    /// Show or hide one overlay.
    pub fn toggle_overlay(&mut self, id: &str) {
        if let Some(o) = self.overlays.iter_mut().find(|o| o.id == id) {
            o.enabled = !o.enabled;
        }
    }
}

impl ConfigStore {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("config.json");
        let cfg = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| match serde_json::from_str::<Config>(&s) {
                Ok(c) => Some(c),
                Err(e) => {
                    log::warn!("config.json is invalid ({e}); keeping a backup and using defaults");
                    let _ = std::fs::copy(&path, dir.join("config.invalid.json"));
                    None
                }
            })
            .unwrap_or_default();
        let mut cfg = cfg;
        cfg.add_new_kinds();
        let s = Self::new(cfg, Some(path));
        s.save();
        s
    }

    pub fn new(cfg: Config, path: Option<PathBuf>) -> Self {
        Self { inner: Arc::new(RwLock::new(cfg)), path: path.map(Arc::new), tx: Arc::new(watch::channel(0).0) }
    }

    pub fn get(&self) -> Config {
        self.inner.read().unwrap().clone()
    }

    pub fn read<R>(&self, f: impl FnOnce(&Config) -> R) -> R {
        f(&self.inner.read().unwrap())
    }

    pub fn update(&self, f: impl FnOnce(&mut Config)) {
        f(&mut self.inner.write().unwrap());
        self.save();
        self.tx.send_modify(|v| *v += 1);
    }

    pub fn replace(&self, cfg: Config) {
        self.update(|c| *c = cfg);
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.tx.subscribe()
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let json = serde_json::to_string_pretty(&*self.inner.read().unwrap()).unwrap_or_default();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, path.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn update_postpone() {
        let later = UpdatePostpone { version: "0.6.0".into(), until: Some(1_000) };
        assert!(later.holds("0.6.0", 999));
        assert!(!later.holds("0.6.0", 1_000), "reminder time reached");
        assert!(!later.holds("0.7.0", 0), "a newer version always pops up");
        let skip = UpdatePostpone { version: "0.6.0".into(), until: None };
        assert!(skip.holds("0.6.0", i64::MAX));
        assert!(!skip.holds("0.6.1", 0));
    }

    use super::*;

    #[test]
    fn partial_config_fills_defaults() {
        let c: Config = serde_json::from_str(r#"{"units":"imperial","fuel":{"marginLaps":2}}"#).unwrap();
        assert_eq!(c.units, "imperial");
        assert_eq!(c.fuel.margin_laps, 2.0);
        assert_eq!(c.fuel.avg_laps, 5);
        assert_eq!(c.server.port, 8182);
    }

    #[test]
    fn layouts_swap_and_remember_changes() {
        let ov = |id: &str, x: i32| OverlayInstance { id: id.into(), kind: "relative".into(), enabled: true, x, ..Default::default() };
        let mut c = Config {
            overlays: vec![ov("a", 0)],
            layouts: vec![
                Layout { id: "race".into(), name: "Race".into(), overlays: vec![ov("a", 0)], ..Default::default() },
                Layout { id: "quali".into(), name: "Quali".into(), overlays: vec![ov("b", 5)], ..Default::default() },
            ],
            active_layout: "race".into(),
            ..Default::default()
        };
        // Move an overlay, swap away and back: the move is kept.
        c.overlays[0].x = 100;
        assert!(c.apply_layout("quali"));
        assert_eq!(c.overlays[0].id, "b");
        assert_eq!(c.next_layout().as_deref(), Some("race"));
        c.apply_layout("race");
        assert_eq!(c.overlays[0].x, 100);
        assert!(!c.apply_layout("nope"));
        c.toggle_overlay("a");
        assert!(!c.overlays[0].enabled);
        assert_eq!(Config::default().next_layout(), None);
        // Automatic layouts per session type.
        c.layouts[1].sessions = vec!["qualifying".into()];
        assert_eq!(c.layout_for_session(session_category("Lone Qualify")).as_deref(), Some("quali"));
        assert_eq!(c.layout_for_session(session_category("Race")), None);
        assert_eq!(session_category("Offline Testing"), "practice");
        assert_eq!(session_category("Warmup"), "warmup");
        assert_eq!(session_category("Heat 1"), "race");
        // An overlay only in one layout comes along, switched off, so its hotkey still works.
        c.apply_layout("quali");
        assert_eq!(c.overlays.iter().map(|o| (o.id.as_str(), o.enabled)).collect::<Vec<_>>(), vec![("b", true), ("a", false)]);
    }

    #[test]
    fn new_overlay_kinds_are_added_switched_off() {
        let mut c = Config { overlays: vec![OverlayInstance { id: "safety".into(), kind: "relative".into(), ..Default::default() }], ..Default::default() };
        c.add_new_kinds();
        let added: Vec<_> = c.overlays.iter().skip(1).map(|o| (o.id.as_str(), o.kind.as_str(), o.enabled)).collect();
        assert_eq!(added, vec![("safety-2", "safety", false), ("delta", "delta", false), ("inputGraph", "inputGraph", false)]);
        c.add_new_kinds();
        assert_eq!(c.overlays.len(), 4);
    }
}
