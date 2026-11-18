//! Derived race state, recomputed every telemetry tick (60 Hz).

pub mod battle;
pub mod fuel;
pub mod incidents;
pub mod laps;
pub mod pit;
pub mod radar;
pub mod ratings;
pub mod sectors;
pub mod trackmap;
pub mod tyres;
pub mod weather;

use crate::config::{Config, Corner};
use crate::league::{ClassKey, Resolver};
use crate::model::{flags, surface, SessionDef, SessionInfo, Telemetry, MAX_CARS};
use std::collections::VecDeque;
use std::path::PathBuf;

/// Session states (irsdk_SessionState).
pub const STATE_RACING: i32 = 4;
pub const STATE_CHECKERED: i32 = 5;

/// Per-car data that only changes with the session info string or config.
#[derive(Debug, Clone, PartialEq)]
pub struct Static {
    pub idx: usize,
    pub class: ClassKey,
    /// Secondary class (League builder), if the driver has one.
    pub class2: Option<ClassKey>,
    pub name: String,
    pub abbrev: String,
    pub team: String,
    pub number: String,
    pub car: String,
    pub car_id: i64,
    pub user_id: i64,
    pub irating: i32,
    pub lic: String,
    pub lic_color: u32,
    pub club: String,
    /// Flair as sent by iRacing (shown where the country could not be recognised).
    pub flair: String,
    pub country: String,
    pub make: String,
    /// League team (Team builder): colour (None: none set), short name and logo files.
    pub team_color: Option<u32>,
    pub team_short: String,
    pub team_logo: String,
    pub team_logo_small: String,
    pub hidden: bool,
    /// iRacing car class (iRating is calculated per iRacing class even when league classes are shown).
    pub ir_class: i64,
    pub lic_class: char,
    /// Expected score against the drivers of the same iRacing class (for the iRating estimate).
    pub ir_expected: f32,
}

#[derive(Debug, Clone)]
pub struct LapRec {
    pub time: f32,
    pub wet: Option<bool>,
    pub at: f64,
    pub pit: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CarHist {
    pub seen: bool,
    pub last_progress: f64,
    pub was_on_pit: bool,
    pub pit_stops: i32,
    pub last_pit_lap: i32,
    pub stint_start_lap: i32,
    pub start_class_pos: i32,
    pub last_lap_seen: f32,
    pub pit_this_lap: bool,
    pub laps: VecDeque<LapRec>,
    /// Left the pits and has not crossed the line yet.
    pub out_lap: bool,
    /// Took the chequered flag.
    pub finished: bool,
    pub prev_completed: i32,
    pub last_cross_at: f64,
    pub laps_led_overall: i32,
    pub laps_led_class: i32,
    /// Off track or stopped on track (i.e. causing a local yellow) until this session time.
    pub yellow_until: f64,
    speed_at: f64,
    speed_pct: f32,
    tick_at: f64,
    /// Estimated speed from track position, m/s.
    pub speed: f32,
    slow_since: Option<f64>,
    /// Off track since (cleared when back on).
    off_since: Option<f64>,
    /// Black flag seen; entered the pits while black-flagged; seconds stopped in the box.
    black: bool,
    pen_pit: bool,
    pen_stopped: f64,
    /// Last penalty served automatically detected ("DT" / "S&G 30s") and until when to show it.
    pub served: Option<(String, f64)>,
    pub pitted: bool,
    pub exit_completed: i32,
    /// Slow-down penalty (iRacing's furled black flag): since when, and the time given up so
    /// far compared with the car's normal pace. iRacing does not say how long it is.
    pub slowdown_since: Option<f64>,
    pub slowdown_lost: f32,
    slowdown_at: (f64, f32),
    /// Fuel put in at pit stops: the amount at the last stop (or so far at this one), the
    /// session total, and whether in litres (else % of a full tank). Lowest reading on this
    /// pit visit and what has gone in since.
    pub fuel_last: Option<f32>,
    pub fuel_total: f32,
    pub fuel_litres: bool,
    pub fuel_filling: bool,
    fuel_low: Option<f32>,
    fuel_now: f32,
    /// Time stationary in the pit box on this pit visit: serving a penalty (stop-and-go: the
    /// car is black-flagged while it waits) and being serviced (fuel, tyres, repairs).
    pit_now: PitStop,
    /// The last stop's times (once the car has left the pit lane).
    pub pit_last: Option<PitStop>,
}

/// Seconds a car stood in its pit box on one stop, split into the part spent serving a
/// penalty and the part being serviced.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PitStop {
    pub service: f32,
    pub penalty: f32,
}

impl PitStop {
    fn any(&self) -> bool {
        self.service + self.penalty >= 0.5
    }
}

/// Where a car is in its stint, for the "stint lap" column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stint {
    Pit,
    OutLap,
    /// Lap number within the stint (race lap number before the first stop).
    Lap(i32),
}

impl CarHist {
    /// Time in the pit box: at this stop while the car is standing in it or still in the pit
    /// lane after it (true), else at the last stop (false).
    pub fn pit_time(&self) -> Option<(PitStop, bool)> {
        if self.pit_now.any() {
            Some((self.pit_now, true))
        } else {
            self.pit_last.map(|p| (p, false))
        }
    }

    /// Counts the time stood in the pit box; `penalty`: the car is black-flagged (a stop-and-go
    /// being served), else it is a service stop.
    fn track_pit_time(&mut self, in_box: bool, penalty: bool, dt: f64) {
        if in_box {
            let t = if penalty { &mut self.pit_now.penalty } else { &mut self.pit_now.service };
            *t += dt as f32;
        }
    }

    /// Left the pit lane (or the car went to its garage): the stop, if it stopped, is done.
    fn end_pit_stop(&mut self) {
        if self.pit_now.any() {
            self.pit_last = Some(self.pit_now);
        }
        self.pit_now = PitStop::default();
    }

    /// Fuel put in at the last stop (or so far at this one) and over the session.
    pub fn fuel_added(&self) -> Option<(f32, f32)> {
        if self.fuel_filling {
            Some((self.fuel_now, self.fuel_total + self.fuel_now))
        } else {
            self.fuel_last.map(|l| (l, self.fuel_total))
        }
    }

    /// Follows the fuel level through pit visits: what goes in while on pit road is added.
    fn track_fuel(&mut self, reading: Option<(f32, bool)>, in_pits: bool) {
        let Some((f, litres)) = reading.filter(|_| in_pits) else {
            // Left the pit lane (or the car went to its garage): the stop is done.
            if self.fuel_low.take().is_some() && self.fuel_filling {
                self.fuel_last = Some(self.fuel_now);
                self.fuel_total += self.fuel_now;
            }
            self.fuel_filling = false;
            return;
        };
        self.fuel_litres = litres;
        let low = self.fuel_low.map_or(f, |l| l.min(f));
        self.fuel_low = Some(low);
        // More than reading noise (LMU tells fuel in steps of 1/255 of a tank).
        let step = if litres { 0.3 } else { 0.5 };
        if f - low > step {
            self.fuel_filling = true;
            self.fuel_now = f - low;
        }
    }

    /// Median of the last few non-pit laps; a robust pace estimate.
    pub fn pace(&self) -> Option<f32> {
        let mut v: Vec<f32> = self.laps.iter().rev().filter(|l| !l.pit && l.time > 0.0).take(5).map(|l| l.time).collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(|a, b| a.total_cmp(b));
        Some(v[v.len() / 2])
    }
    pub fn last(&self) -> Option<&LapRec> {
        self.laps.back()
    }
}

/// One entry of the computed running order.
#[derive(Debug, Clone, Default)]
pub struct Row {
    pub idx: usize,
    pub pos: i32,
    pub class_pos: i32,
    /// Gap to class leader; `None` with `gap_laps > 0` means laps down.
    pub gap: Option<f32>,
    pub gap_laps: i32,
    pub interval: Option<f32>,
    pub interval_laps: i32,
    /// The same against the overall leader and the car ahead overall, for one overall list.
    pub gap_overall: Option<f32>,
    pub gap_overall_laps: i32,
    pub interval_overall: Option<f32>,
    pub interval_overall_laps: i32,
}

#[derive(Debug, Clone)]
pub struct ClassSummary {
    pub key: ClassKey,
    pub count: usize,
    pub sof: i32,
    pub best_lap: Option<(usize, f32)>,
}

pub struct Engine {
    pub tel: Telemetry,
    pub info: SessionInfo,
    pub connected: bool,
    pub source: &'static str,
    pub cfg: Config,
    pub resolver: Resolver,
    pub statics: Vec<Option<Static>>,
    /// Bumped whenever `statics` is rebuilt, so the `drivers` topic is only re-sent then.
    pub statics_rev: u64,
    pub hist: Vec<CarHist>,
    pub order: Vec<Row>,
    pub classes: Vec<ClassSummary>,
    pub fuel: fuel::FuelTracker,
    /// Virtual energy used per lap (Le Mans Ultimate), in % of the allowance.
    pub energy: fuel::FuelTracker,
    /// Fuel use per car/track combination from earlier sessions.
    pub fuel_history: fuel::History,
    fuel_combo: String,
    pub weather: weather::WeatherTracker,
    pub battle: battle::BattleTracker,
    pub map: trackmap::Recorder,
    /// Your best laps (this session, all time) for the delta bar.
    pub laps: laps::LapRecorder,
    pub sectors: sectors::SectorTracker,
    pub incidents: incidents::IncidentTracker,
    pub pit: pit::PitTracker,
    /// The player's pit lane state this tick.
    pub pit_out: pit::PitOut,
    /// Corners of the current track (custom > downloaded > source-provided).
    pub corners: Vec<Corner>,
    pub corners_source: &'static str,
    /// Le Mans Ultimate's weather forecast, while LMU is read.
    pub lmu_forecast: Option<crate::source::lmu::extras::Forecast>,
    remote_track: Option<(i64, crate::corners::TrackData, &'static str)>,
    /// Lovely Car Data per `CarPath` (lower case) and Lovely Car Classes shorthands.
    pub car_data: std::collections::HashMap<String, crate::cardata::CarData>,
    class_names: std::collections::HashMap<String, String>,
    /// Bumped when car data arrives (the car topic is only re-sent then).
    pub car_revision: u64,
    /// Starting grid (car indices, pole first) from qualifying, kept on disk per subsession.
    pub grid: Vec<usize>,
    grid_dir: Option<PathBuf>,
    grid_saved: (i64, Vec<usize>),
    /// Live iRating change estimate per car (races only).
    ir: Vec<Option<f32>>,
    /// Countries from the iRacing Data API for drivers the session info has none for.
    remote_countries: std::collections::BTreeMap<i64, String>,
    /// Session time the leader took the chequered flag.
    checkered_at: Option<f64>,
    last_session: (i64, i32),
    last_state: i32,
}

impl Engine {
    pub fn new(cfg: Config, data_dir: Option<PathBuf>) -> Self {
        let mut e = Self {
            tel: Telemetry::default(),
            info: SessionInfo::default(),
            connected: false,
            source: "none",
            resolver: Resolver::default(),
            cfg,
            statics: vec![None; MAX_CARS],
            statics_rev: 0,
            hist: vec![CarHist::default(); MAX_CARS],
            order: vec![],
            classes: vec![],
            fuel: Default::default(),
            energy: Default::default(),
            fuel_history: fuel::History::load(data_dir.as_ref().map(|d| d.join("fuel-history.json"))),
            fuel_combo: String::new(),
            weather: Default::default(),
            battle: Default::default(),
            map: trackmap::Recorder::new(data_dir.as_ref().map(|d| d.join("trackmaps"))),
            laps: laps::LapRecorder::new(data_dir.as_ref().map(|d| d.join("laps"))),
            incidents: Default::default(),
            pit: pit::PitTracker::new(data_dir.as_ref().map(|d| d.join("pit-lines.json"))),
            pit_out: Default::default(),
            grid: vec![],
            grid_dir: data_dir.map(|d| d.join("grids")),
            grid_saved: (0, vec![]),
            ir: vec![None; MAX_CARS],
            sectors: Default::default(),
            corners: vec![],
            corners_source: "none",
            lmu_forecast: None,
            remote_track: None,
            remote_countries: Default::default(),
            car_data: Default::default(),
            class_names: Default::default(),
            car_revision: 0,
            checkered_at: None,
            last_session: (-1, -1),
            last_state: 0,
        };
        e.set_config(e.cfg.clone());
        e
    }

    pub fn set_config(&mut self, cfg: Config) {
        self.cfg = cfg;
        self.resolver = Resolver::new(self.pick_league());
        self.rebuild_statics();
        self.refresh_corners();
    }

    /// The league to apply to this session. Automatic: the league built for the session's
    /// iRacing league ID, or none (official races, other leagues). Otherwise the one picked.
    pub fn pick_league(&self) -> Option<crate::config::League> {
        let cfg = &self.cfg;
        if cfg.league_mode == "always" {
            return cfg.active_league.as_ref().and_then(|id| cfg.leagues.iter().find(|l| &l.id == id)).cloned();
        }
        let id = self.info.league_id;
        (id > 0).then(|| cfg.leagues.iter().find(|l| l.iracing_league_id == id)).flatten().cloned()
    }

    /// Corner data fetched or generated outside the engine for `track_id`.
    pub fn set_remote_corners(&mut self, track_id: i64, corners: Vec<Corner>, source: &'static str) {
        self.set_track_data(track_id, crate::corners::TrackData { corners, ..Default::default() }, source);
    }

    /// Track data (corners, straights, pit entry/exit) fetched outside the engine for `track_id`.
    pub fn set_track_data(&mut self, track_id: i64, data: crate::corners::TrackData, source: &'static str) {
        self.remote_track = Some((track_id, data, source));
        self.refresh_corners();
    }

    /// Downloaded data for the current track (straights, pit entry/exit), if any.
    pub fn track_data(&self) -> Option<&crate::corners::TrackData> {
        self.remote_track.as_ref().filter(|r| r.0 == self.info.track_id).map(|r| &r.1)
    }

    pub fn set_car_data(&mut self, d: std::collections::HashMap<String, crate::cardata::CarData>) {
        self.car_data = d;
        self.car_revision += 1;
        self.rebuild_statics();
    }

    pub fn set_class_names(&mut self, c: std::collections::HashMap<String, String>) {
        self.class_names = c;
        self.rebuild_statics();
    }

    /// Short name for an iRacing car class: Lovely Car Classes by iRacing's class name, else the
    /// class all of its cars share in Lovely Car Data ("Touring Car" → "TCR").
    fn short_class(&self, d: &crate::model::Driver) -> Option<String> {
        // iRacing's own names that are simply wrong for the series ("Touring Car" is TCR).
        const FIXED: &[(&str, &str)] = &[("Touring Car", "TCR")];
        if let Some((_, n)) = FIXED.iter().find(|(k, _)| k.eq_ignore_ascii_case(d.class_name.trim())) {
            return Some((*n).to_string());
        }
        if !self.cfg.short_class_names {
            return None;
        }
        if let Some(s) = self.class_names.get(&d.class_name) {
            return Some(s.clone());
        }
        let mut found: Option<&str> = None;
        for o in self.info.drivers.iter().filter(|o| o.class_id == d.class_id && !o.is_pace_car && !o.is_spectator) {
            match self.car_data.get(&o.car_path.to_lowercase()).map(|c| c.car_class.as_str()).filter(|c| !c.is_empty()) {
                Some(c) if found.is_none_or(|f| f == c) => found = Some(c),
                Some(_) => return None,
                None => {}
            }
        }
        found.map(String::from)
    }

    pub fn set_remote_countries(&mut self, c: std::collections::BTreeMap<i64, String>) {
        self.remote_countries = c;
        self.rebuild_statics();
    }

    fn refresh_corners(&mut self) {
        let id = self.info.track_id;
        if let Some(c) = self.cfg.track_corners.get(&id.to_string()).filter(|c| !c.is_empty()) {
            self.corners = c.clone();
            self.corners_source = "custom";
        } else if let Some((_, d, src)) = self.remote_track.as_ref().filter(|r| r.0 == id && !r.1.corners.is_empty()) {
            self.corners = d.corners.clone();
            self.corners_source = src;
        } else {
            self.corners.clear();
            self.corners_source = "none";
        }
        self.map.revision += 1;
    }

    /// Corners per lap, estimated from the track length when no corner data exists.
    pub fn corners_per_lap(&self) -> f32 {
        if !self.corners.is_empty() {
            self.corners.len() as f32
        } else {
            (self.info.track_length_km * 3.0).max(6.0)
        }
    }

    /// Compass heading (degrees, 0-360) of the focus car. The player's own yaw while driving;
    /// otherwise the direction of the track where the watched car is.
    pub fn focus_heading(&self) -> Option<f32> {
        let t = &self.tel;
        let idx = t.focus_idx()?;
        let is_player = usize::try_from(t.player_idx).ok() == Some(idx);
        let rad = if is_player && t.driving() { Some(t.yaw_north) } else { self.map.heading_at(t.cars[idx].pct).or_else(|| is_player.then_some(t.yaw_north)) };
        rad.map(|r| r.to_degrees().rem_euclid(360.0))
    }

    pub fn stint(&self, idx: usize) -> Stint {
        let (c, h) = (&self.tel.cars[idx], &self.hist[idx]);
        if c.on_pit_road {
            Stint::Pit
        } else if h.out_lap {
            Stint::OutLap
        } else if h.pitted {
            Stint::Lap((c.lap_completed - h.exit_completed).max(1))
        } else {
            Stint::Lap(c.lap.max(1))
        }
    }

    /// Incidents of a car this session (live for the player, from the session string otherwise).
    pub fn incidents(&self, idx: usize) -> i32 {
        if idx as i32 == self.tel.player_idx {
            self.tel.incidents
        } else {
            self.info.driver(idx).map(|d| d.incidents).unwrap_or(0)
        }
    }

    /// Other drivers' incident counts are hidden in many sessions; assume visible once any is non-zero.
    pub fn incidents_visible(&self, idx: usize) -> bool {
        idx as i32 == self.tel.player_idx || self.info.drivers.iter().any(|d| d.incidents > 0 && d.car_idx as i32 != self.tel.player_idx)
    }

    /// Fuel history key for the player's car at this track ("" while unknown).
    pub fn fuel_key(&self) -> String {
        let car = self.info.driver(self.info.player_car_idx.max(0) as usize).map(|d| d.car_id).unwrap_or(0);
        if car > 0 && self.info.track_id > 0 {
            fuel::History::key(car, self.info.track_id)
        } else {
            String::new()
        }
    }

    pub fn ir_delta(&self, idx: usize) -> Option<f32> {
        self.ir.get(idx).copied().flatten()
    }

    /// iRating change of every car if the race finished now. The field is every registered
    /// driver of the class (also those who have not loaded in yet), in running order; before
    /// the start, in grid order. Drivers who never took part count as non-starters.
    fn compute_ir(&mut self) {
        self.ir.iter_mut().for_each(|v| *v = None);
        if !self.is_race() {
            return;
        }
        let racing = self.tel.session_state >= STATE_RACING;
        let mut ordered: Vec<usize> = self.order.iter().map(|r| r.idx).collect();
        let registered = (0..MAX_CARS).filter(|&i| self.statics[i].is_some());
        for i in self.grid.iter().copied().chain(registered.collect::<Vec<_>>()) {
            if self.statics.get(i).is_some_and(|s| s.is_some()) && !ordered.contains(&i) {
                ordered.push(i);
            }
        }
        let mut classes: std::collections::BTreeMap<i64, Vec<usize>> = Default::default();
        for i in ordered {
            classes.entry(self.statics[i].as_ref().unwrap().ir_class).or_default().push(i);
        }
        for idxs in classes.values() {
            let expected: Vec<f32> = idxs.iter().map(|&i| self.statics[i].as_ref().unwrap().ir_expected).collect();
            let started: Vec<bool> = idxs.iter().map(|&i| !racing || self.hist[i].seen || self.tel.cars[i].in_world()).collect();
            for (k, c) in ratings::ir_changes(&expected, &started).into_iter().enumerate() {
                self.ir[idxs[k]] = Some(c);
            }
        }
    }

    /// The starting grid: qualifying results carried into the race, else the last qualifying
    /// session of the event, else the grid saved for this subsession (e.g. after a restart).
    fn refresh_grid(&mut self) {
        let sub = self.info.subsession_id;
        let path = self.grid_dir.as_ref().filter(|_| sub > 0).map(|d| d.join(format!("{sub}.json")));
        let order = self.qualify_order();
        if !order.is_empty() {
            if let Some(p) = path.filter(|_| self.grid_saved != (sub, order.clone())) {
                if let Some(d) = p.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = std::fs::write(p, serde_json::to_string(&order).unwrap_or_default());
                self.grid_saved = (sub, order.clone());
            }
            self.grid = order;
        } else if let Some(saved) = path.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str::<Vec<usize>>(&s).ok()) {
            self.grid = saved;
        } else {
            self.grid.clear();
        }
    }

    pub fn sr_delta(&self, idx: usize) -> Option<f32> {
        let s = self.static_of(idx)?;
        if !self.incidents_visible(idx) {
            return None;
        }
        let c = &self.tel.cars[idx];
        let per_lap = self.corners_per_lap();
        let passed = if self.corners.is_empty() { c.pct.max(0.0) * per_lap } else { self.corners.iter().filter(|k| k.end <= c.pct).count() as f32 };
        let corners = c.lap_completed.max(0) as f32 * per_lap + passed;
        Some(ratings::sr_change(s.lic_class, corners, self.incidents(idx)))
    }

    /// League points for the current running position, and the projected championship total.
    /// Best lap this session, from telemetry or (for cars no longer in the world) the results.
    pub fn best_lap(&self, idx: usize) -> Option<f32> {
        let b = self.tel.cars[idx].best_lap;
        if b > 0.0 {
            return Some(b);
        }
        self.result(idx).map(|r| r.fastest_time).filter(|&t| t > 0.0)
    }

    pub fn last_lap(&self, idx: usize) -> Option<f32> {
        let l = self.tel.cars[idx].last_lap;
        if l > 0.0 {
            return Some(l);
        }
        self.result(idx).map(|r| r.last_time).filter(|&t| t > 0.0)
    }

    fn result(&self, idx: usize) -> Option<&crate::model::ResultPos> {
        self.session_def().and_then(|d| d.results.iter().find(|r| r.car_idx == idx))
    }

    /// Qualifying order (car indices) for pole bonuses: carried-over results, else the last qualifying session.
    fn qualify_order(&self) -> Vec<usize> {
        if !self.info.qualify.is_empty() {
            return self.info.qualify.iter().map(|r| r.car_idx).collect();
        }
        let q = self.info.sessions.iter().rfind(|s| s.kind.to_lowercase().contains("qualify") && !s.results.is_empty());
        let mut res: Vec<&crate::model::ResultPos> = q.map(|s| s.results.iter().collect()).unwrap_or_default();
        res.sort_by_key(|r| r.position);
        res.iter().map(|r| r.car_idx).collect()
    }

    /// Cars in the same points group as `idx` (its class, or everyone for overall scoring).
    fn points_group(&self, idx: usize, overall: bool) -> Vec<usize> {
        let class = self.static_of(idx).map(|s| s.class.id.clone());
        self.order.iter().map(|r| r.idx).filter(|&i| overall || self.static_of(i).map(|s| &s.class.id) == class.as_ref()).collect()
    }

    pub fn points(&self, idx: usize) -> (Option<f32>, Option<f32>) {
        let Some(league) = self.resolver.league() else { return (None, None) };
        let (Some(r), Some(s)) = (self.row(idx), self.static_of(idx)) else { return (None, None) };
        if !self.is_race() {
            return (None, None);
        }
        let p = &league.points;
        let overall = p.basis == "overall";
        let pos = if overall { r.pos } else { r.class_pos };
        let fastest = if overall {
            self.classes.iter().filter_map(|c| c.best_lap).min_by(|a, b| a.1.total_cmp(&b.1)).map(|b| b.0)
        } else {
            self.classes.iter().find(|c| c.key.id == s.class.id).and_then(|c| c.best_lap).map(|b| b.0)
        };
        let group = self.points_group(idx, overall);
        let pole = self.qualify_order().into_iter().find(|i| group.contains(i));
        let led = |i: usize| if overall { self.hist[i].laps_led_overall } else { self.hist[i].laps_led_class };
        let most = group.iter().copied().max_by_key(|&i| led(i)).filter(|&i| led(i) > 0);
        let pts = ratings::points(p, pos, fastest == Some(idx)).map(|base| {
            base + if pole == Some(idx) { p.pole_bonus } else { 0.0 }
                + if led(idx) > 0 { p.lap_led_bonus } else { 0.0 }
                + if most == Some(idx) { p.most_laps_led_bonus } else { 0.0 }
                + p.finish_bonus
        });
        let season = league.season_points.iter().find(|e| e.customer_id == s.user_id).map(|e| e.points);
        let total = match (season, pts) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, b) if !league.season_points.is_empty() => b,
            _ => None,
        };
        (pts, total)
    }

    pub fn set_session(&mut self, info: SessionInfo) {
        let track_changed = info.track_id != self.info.track_id;
        let league_changed = info.league_id != self.info.league_id;
        self.info = info;
        if league_changed {
            self.resolver = Resolver::new(self.pick_league());
        }
        if track_changed {
            self.map.load(self.info.track_id, &self.info.track_display_name);
        }
        self.rebuild_statics();
        self.refresh_grid();
        if track_changed {
            self.refresh_corners();
        }
    }

    fn rebuild_statics(&mut self) {
        let mut statics = vec![None; MAX_CARS];
        for d in &self.info.drivers {
            if d.car_idx >= MAX_CARS || d.is_spectator || d.is_pace_car || d.car_idx as i32 == self.info.pace_car_idx {
                continue;
            }
            let (name, team, hidden) = self.resolver.display(d);
            let lt = self.resolver.team_of(&team);
            let (team_color, team_short, team_logo, team_logo_small) = match lt {
                Some(t) => (crate::league::parse_color(&t.color), t.short.trim().to_string(), t.logo.clone(), t.logo_small.clone()),
                None => (None, String::new(), String::new(), String::new()),
            };
            let mut class = self.resolver.class_of(d);
            if class.id.starts_with("ir") {
                if let Some(short) = self.short_class(d) {
                    class.name = short;
                }
            }
            statics[d.car_idx] = Some(Static {
                idx: d.car_idx,
                class,
                class2: self.resolver.secondary_of(d),
                abbrev: if d.abbrev.is_empty() { name.clone() } else { d.abbrev.clone() },
                name,
                team,
                number: self.resolver.car_number(d),
                car: if d.car_short.is_empty() { d.car_name.clone() } else { d.car_short.clone() },
                car_id: d.car_id,
                user_id: d.user_id,
                irating: d.irating,
                lic: d.lic_string.clone(),
                lic_color: d.lic_color,
                club: d.club.clone(),
                flair: d.flair.clone(),
                country: Some(override_country(&self.resolver, d.user_id))
                    .filter(|c| !c.is_empty())
                    .or_else(|| (!d.country.is_empty()).then(|| d.country.clone()))
                    .or_else(|| self.remote_countries.get(&d.user_id).cloned())
                    .unwrap_or_default(),
                make: d.make.clone(),
                team_color,
                team_short,
                team_logo,
                team_logo_small,
                hidden,
                ir_class: d.class_id,
                lic_class: d.licence().0,
                ir_expected: 0.0,
            });
        }
        // Expected scores per iRacing class for the iRating estimate (iRatings only change with the session string).
        let mut groups: std::collections::BTreeMap<i64, Vec<usize>> = Default::default();
        for s in statics.iter().flatten() {
            groups.entry(s.ir_class).or_default().push(s.idx);
        }
        for idxs in groups.values() {
            let irs: Vec<i32> = idxs.iter().map(|&i| statics[i].as_ref().unwrap().irating).collect();
            for (k, e) in ratings::expected_scores(&irs).into_iter().enumerate() {
                statics[idxs[k]].as_mut().unwrap().ir_expected = e;
            }
        }
        // iRacing re-sends the session info every few seconds in a race (each results change):
        // the drivers' details are only sent out again to the overlays when they changed.
        if statics != self.statics {
            self.statics = statics;
            self.statics_rev += 1;
        }
    }

    /// Forget everything about the current session (simulator closed).
    pub fn reset(&mut self) {
        self.connected = false;
        self.tel = Telemetry::default();
        self.set_session(SessionInfo::default());
        self.hist = vec![CarHist::default(); MAX_CARS];
        self.order.clear();
        self.classes.clear();
        self.battle = Default::default();
        self.last_session = (-1, -1);
    }

    pub fn session_def(&self) -> Option<&SessionDef> {
        self.info.session(self.tel.session_num)
    }

    pub fn is_race(&self) -> bool {
        self.session_def().is_some_and(|s| s.is_race())
    }

    /// Driving alone in Le Mans Ultimate's private qualifying: the other cars are on track in
    /// sessions of their own, so they are not around you (relative, radar).
    pub fn alone_on_track(&self) -> bool {
        self.source == "lmu" && self.cfg.lmu_private_qualifying && self.session_def().is_some_and(|s| crate::config::session_category(&s.kind) == "qualifying")
    }

    pub fn under_caution(&self) -> bool {
        self.tel.session_flags & (flags::CAUTION | flags::CAUTION_WAVING) != 0
    }

    pub fn static_of(&self, idx: usize) -> Option<&Static> {
        self.statics.get(idx).and_then(|s| s.as_ref())
    }

    pub fn progress(&self, idx: usize) -> f64 {
        let c = &self.tel.cars[idx];
        if c.in_world() {
            c.progress()
        } else {
            self.hist[idx].last_progress
        }
    }

    /// Reference lap time used to convert track distance into time.
    pub fn ref_lap(&self, idx: usize) -> f32 {
        if let Some(p) = self.hist.get(idx).and_then(|h| h.pace()) {
            return p;
        }
        let est = self.info.driver(idx).map(|d| d.class_est_lap).unwrap_or(0.0);
        if est > 0.0 {
            est
        } else if self.info.est_lap_time > 0.0 {
            self.info.est_lap_time
        } else {
            100.0
        }
    }

    /// How far round its lap a car is, in time rather than distance (0..1): the sim's estimated
    /// time to the car's position over its class's estimated lap time. Unlike the raw estimated
    /// time this has no jump at the line (the raw value falls from the class lap time to zero,
    /// which is not the lap time gaps are scaled with), and it accounts for slow and fast parts
    /// of the track. Falls back to the distance fraction when the sim gives no estimate.
    pub fn lap_frac(&self, idx: usize) -> f32 {
        let c = &self.tel.cars[idx];
        let est = self.info.driver(idx).map(|d| d.class_est_lap).filter(|e| *e > 0.0).unwrap_or(self.info.est_lap_time);
        if est > 0.0 && c.est_time > 0.0 && c.est_time < est * 1.2 {
            (c.est_time / est).clamp(0.0, 0.9999)
        } else {
            c.pct.clamp(0.0, 0.9999)
        }
    }

    /// Time from `behind` to `ahead` in race terms. Returns (seconds, whole laps).
    pub fn time_between(&self, behind: usize, ahead: usize) -> (Option<f32>, i32) {
        let dp = self.progress(ahead) - self.progress(behind);
        if dp >= 1.0 {
            return (None, dp.floor() as i32);
        }
        if dp < 0.0 {
            return (Some(0.0), 0);
        }
        let l = self.ref_lap(behind);
        let df = (self.lap_frac(ahead) - self.lap_frac(behind)).rem_euclid(1.0);
        // Near the line the time and distance fractions can disagree on which side a car is.
        let df = if (df - dp as f32).abs() > 0.5 { dp as f32 } else { df };
        (Some((df * l).clamp(0.0, l)), 0)
    }

    /// On-track gap from the focus car to `idx` (positive = `idx` is ahead on track).
    pub fn relative_gap(&self, focus: usize, idx: usize) -> f32 {
        let mut df = self.lap_frac(idx) - self.lap_frac(focus);
        if df > 0.5 {
            df -= 1.0;
        } else if df < -0.5 {
            df += 1.0;
        }
        let l = self.ref_lap(focus);
        (df * l).clamp(-l / 2.0, l / 2.0)
    }

    /// Advance all derived state after the source has written a new telemetry row.
    pub fn tick(&mut self) {
        let key = (self.info.subsession_id, self.tel.session_num);
        if key != self.last_session {
            self.last_session = key;
            self.hist = vec![CarHist::default(); MAX_CARS];
            self.battle = Default::default();
            self.sectors = Default::default();
            self.fuel.new_session();
            self.energy.new_session();
            self.checkered_at = None;
        }
        self.update_hist();
        self.update_slowdowns();
        self.compute_order();
        self.compute_ir();
        if self.is_race() && self.tel.session_state >= STATE_CHECKERED && self.checkered_at.is_none() {
            // The leader just crossed the line under the chequered flag.
            let now = self.tel.session_time;
            self.checkered_at = Some(now);
            if let Some(leader) = self.order.first().map(|r| r.idx) {
                if now - self.hist[leader].last_cross_at < 2.0 {
                    self.hist[leader].finished = true;
                }
            }
        }
        if self.tel.session_state == STATE_RACING && self.last_state != STATE_RACING && self.is_race() {
            for r in &self.order {
                self.hist[r.idx].start_class_pos = r.class_pos;
            }
        }
        self.last_state = self.tel.session_state;
        let caution = self.under_caution();
        // A different car or track: earlier laps' consumption does not apply.
        let combo = self.fuel_key();
        if combo != self.fuel_combo {
            self.fuel.clear();
            self.energy.clear();
            self.fuel_combo = combo.clone();
        }
        let counted = self.fuel.counted;
        self.fuel.update(&self.tel, caution, &self.cfg.fuel);
        if let Some(ve) = self.tel.virtual_energy {
            self.energy.update_level(ve, &self.tel, caution, &self.cfg.fuel);
        }
        if self.fuel.counted != counted && !combo.is_empty() {
            if let Some(lap) = self.fuel.laps.back().copied().filter(|l| l.valid) {
                let me = self.info.driver(self.info.player_car_idx.max(0) as usize);
                let car = me.map(|d| d.car_name.clone()).unwrap_or_default();
                let track = format!("{} {}", self.info.track_display_name, self.info.track_config).trim().to_string();
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                self.fuel_history.add(&combo, &car, &track, &lap, now);
            }
        }
        self.weather.update(&self.tel);
        // Le Mans Ultimate publishes where the cars are: its source builds the outline from that
        // (see source::lmu::map) instead of reckoning it from the player's heading.
        if self.source != "lmu" {
            self.map.update(&self.tel, &self.info);
        }
        let me = usize::try_from(self.tel.player_idx).ok().and_then(|i| self.static_of(i)).map(|s| s.name.clone()).unwrap_or_default();
        self.laps.update(&self.tel, &self.info, &me);
        self.sectors.update(&self.tel, &self.info.sectors);
        self.incidents.update(&self.tel, &self.info);
        self.pit.update(&self.tel, self.info.track_id);
        let (entry, exit) = self.pit_lines();
        self.pit_out = self.pit.player(&self.tel, entry, exit, self.info.track_length_km * 1000.0, self.info.pit_speed_limit_kph);
        battle::update(self);
    }

    /// Pit entry and exit lines (lap fractions): learned from cars crossing them, else from
    /// the track data.
    pub fn pit_lines(&self) -> (Option<f32>, Option<f32>) {
        let (le, lx) = self.pit.learned();
        let td = self.track_data();
        (le.or(td.and_then(|d| d.pit_entry)), lx.or(td.and_then(|d| d.pit_exit)))
    }

    /// Who is causing a yellow (off track, or stopped/crawling on track) and how black flags are served.
    fn track_incidents(h: &mut CarHist, c: &crate::model::CarTel, now: f64, track_km: f32, running: bool) {
        let tick_dt = (now - h.tick_at).clamp(0.0, 0.5);
        h.tick_at = now;
        let dt = now - h.speed_at;
        if !(0.0..=0.2).contains(&dt) {
            let mut dp = c.pct - h.speed_pct;
            if dp < -0.5 {
                dp += 1.0;
            }
            if dt > 0.0 && dt < 2.0 && h.seen {
                h.speed = (dp.max(0.0) * track_km.max(1.0) * 1000.0) / dt as f32;
            }
            h.speed_at = now;
            h.speed_pct = c.pct;
        }
        let off = c.surface == surface::OFF_TRACK;
        if off {
            h.off_since.get_or_insert(now);
        } else {
            h.off_since = None;
        }
        // Clipping a kerb or cutting a chicane (Monza's Roggia, say) reads as off track for a
        // moment without being an incident or a danger: it takes a while off, or being slow
        // off the track, to cause a yellow.
        let off_yellow = h.off_since.is_some_and(|t| now - t > OFF_YELLOW) || (off && h.speed < OFF_SLOW);
        let crawling = c.surface == surface::ON_TRACK && !c.on_pit_road && h.speed < 8.0 && running;
        if crawling {
            h.slow_since.get_or_insert(now);
        } else {
            h.slow_since = None;
        }
        if off_yellow || h.slow_since.is_some_and(|t| now - t > 1.5) {
            h.yellow_until = now + YELLOW_LINGER;
        }
        let black = c.session_flags & flags::BLACK != 0;
        if black {
            h.pen_pit |= c.on_pit_road;
            if c.on_pit_road && (c.surface == surface::IN_PIT_STALL || h.speed < 0.5) {
                h.pen_stopped += tick_dt;
            }
        } else if h.black {
            // Black flag cleared: if it was served in the pits, show how.
            if h.pen_pit {
                let label = if h.pen_stopped > 1.0 { format!("S&G {}s", h.pen_stopped.round()) } else { "DT".into() };
                h.served = Some((label, now + 15.0));
            }
            h.pen_pit = false;
            h.pen_stopped = 0.0;
        }
        h.black = black;
        // Standing in the pit box (the game says the car is in its stall, and it is not moving).
        let in_box = c.on_pit_road && c.surface == surface::IN_PIT_STALL && h.speed < 0.5;
        h.track_pit_time(in_box, black, tick_dt);
    }

    /// Slow-down penalties: how long each has been out and the time given up so far (time
    /// taken minus what the stretch of track covered takes at the car's normal pace; the lap
    /// fraction is in time, so slow corners are not counted as time given up).
    fn update_slowdowns(&mut self) {
        let now = self.tel.session_time;
        let paces: Vec<f32> = (0..MAX_CARS).map(|i| self.ref_lap(i)).collect();
        let fracs: Vec<f32> = (0..MAX_CARS).map(|i| self.lap_frac(i)).collect();
        for (i, h) in self.hist.iter_mut().enumerate() {
            let c = &self.tel.cars[i];
            if c.session_flags & flags::FURLED == 0 || !c.in_world() {
                h.slowdown_since = None;
                h.slowdown_lost = 0.0;
                continue;
            }
            if h.slowdown_since.is_none() {
                h.slowdown_since = Some(now);
                h.slowdown_lost = 0.0;
            } else {
                let (at, frac) = h.slowdown_at;
                let dt = (now - at) as f32;
                let mut dp = fracs[i] - frac;
                if dp < -0.5 {
                    dp += 1.0;
                }
                // Time in the pits is not slowing down on track.
                if (0.0..0.5).contains(&dt) && (0.0..0.2).contains(&dp) && !c.on_pit_road {
                    h.slowdown_lost = (h.slowdown_lost + dt - dp * paces[i]).max(0.0);
                }
            }
            h.slowdown_at = (now, fracs[i]);
        }
    }

    /// Flags to show next to a driver's name.
    pub fn car_flags(&self, idx: usize) -> Vec<&'static str> {
        let c = &self.tel.cars[idx];
        let f = c.session_flags;
        let mut out = vec![];
        if f & flags::DISQUALIFY != 0 {
            out.push("disqualify");
        } else if f & flags::BLACK != 0 {
            out.push("black");
        }
        if f & flags::REPAIR != 0 {
            out.push("repair");
        }
        if f & flags::FURLED != 0 {
            // iRacing's slow-down penalty (for cutting the track).
            out.push("slow");
        }
        if self.tel.session_time < self.hist[idx].yellow_until {
            out.push("yellow");
        }
        if f & flags::BLUE != 0 {
            out.push("blue");
        }
        out
    }

    /// Penalty label: race control's penalty, how a black flag is being served, or one just served.
    pub fn penalty(&self, idx: usize) -> Option<(String, bool)> {
        let s = self.static_of(idx)?;
        if let Some(p) = self.cfg.penalties.iter().find(|p| p.customer_id == s.user_id && s.user_id != 0) {
            return Some((p.label(), false));
        }
        let h = &self.hist[idx];
        if h.black && h.pen_pit {
            return Some((if h.pen_stopped > 0.5 { format!("S&G {}s", h.pen_stopped.floor()) } else { "DT".into() }, false));
        }
        h.served.as_ref().filter(|(_, until)| self.tel.session_time < *until).map(|(l, _)| (l.clone(), true))
    }

    fn update_hist(&mut self) {
        let now = self.tel.session_time;
        let racing = self.is_race() && self.tel.session_state >= STATE_RACING;
        // Running order from the previous tick decides who led the lap that just ended.
        let overall_leader = self.order.first().map(|r| r.idx);
        let class_leaders: Vec<usize> = self.order.iter().filter(|r| r.class_pos == 1).map(|r| r.idx).collect();
        let checkered_at = self.checkered_at;
        // The player's own fuel in litres where the game only tells it for the player (iRacing).
        let player = usize::try_from(self.tel.player_idx).ok();
        let my_fuel = (self.tel.fuel_level > 0.0).then_some(self.tel.fuel_level);
        for i in 0..MAX_CARS {
            let c = &self.tel.cars[i];
            let wet = self.info.tire_is_wet(c.tire_compound);
            let h = &mut self.hist[i];
            let fuel = match c.fuel {
                Some(f) => Some((f * 100.0, false)),
                None if player == Some(i) => my_fuel.map(|l| (l, true)),
                None => None,
            };
            h.track_fuel(fuel, c.on_pit_road && c.in_world());
            if !(c.on_pit_road && c.in_world()) {
                h.end_pit_stop();
            }
            if !c.in_world() {
                continue;
            }
            if h.seen && c.lap_completed > h.prev_completed {
                h.last_cross_at = now;
                if racing {
                    h.laps_led_overall += (overall_leader == Some(i)) as i32;
                    h.laps_led_class += class_leaders.contains(&i) as i32;
                }
                if checkered_at.is_some_and(|t| now >= t - 0.05) {
                    h.finished = true;
                }
            }
            h.prev_completed = c.lap_completed;
            Self::track_incidents(h, c, now, self.info.track_length_km, racing || self.tel.session_state == STATE_RACING);
            h.seen = true;
            h.last_progress = c.progress();
            if c.on_pit_road {
                h.pit_this_lap = true;
            }
            if c.on_pit_road && !h.was_on_pit {
                h.pit_stops += 1;
                h.last_pit_lap = c.lap;
            }
            if !c.on_pit_road && h.was_on_pit {
                h.stint_start_lap = c.lap;
                h.out_lap = true;
                h.pitted = true;
                h.exit_completed = c.lap_completed;
            }
            if h.out_lap && c.lap_completed > h.exit_completed {
                h.out_lap = false;
            }
            h.was_on_pit = c.on_pit_road;
            if c.last_lap > 0.0 && (c.last_lap - h.last_lap_seen).abs() > 1e-4 {
                h.last_lap_seen = c.last_lap;
                h.laps.push_back(LapRec { time: c.last_lap, wet, at: now, pit: h.pit_this_lap });
                h.pit_this_lap = c.on_pit_road;
                if h.laps.len() > 30 {
                    h.laps.pop_front();
                }
            }
        }
    }

    fn compute_order(&mut self) {
        let t = &self.tel;
        let race = self.is_race();
        let results = self.session_def().map(|s| s.results.as_slice()).unwrap_or(&[]);
        let result_pos = |idx: usize| results.iter().find(|r| r.car_idx == idx).map(|r| r.position).unwrap_or(0);
        let best = |idx: usize| {
            let b = t.cars[idx].best_lap;
            if b > 0.0 {
                return b;
            }
            results.iter().find(|r| r.car_idx == idx).map(|r| r.fastest_time).filter(|&v| v > 0.0).unwrap_or(f32::MAX)
        };
        // Before the start the standings show the grid, including drivers still loading in.
        let pre_start = race && t.session_state < STATE_RACING && !self.grid.is_empty();
        let grid_pos = |i: usize| self.grid.iter().position(|&g| g == i).map(|p| p as i32 + 1).unwrap_or(i32::MAX);
        // Every driver in the session is listed, also those in the garage or still loading in
        // (spectators and the pace car are not drivers).
        let mut idxs: Vec<usize> = (0..MAX_CARS).filter(|&i| self.statics[i].as_ref().is_some_and(|s| !s.hidden)).collect();
        let num = |i: usize| self.statics[i].as_ref().and_then(|s| s.number.parse::<i32>().ok()).unwrap_or(9999);
        let pos_or_max = |p: i32| if p > 0 { p } else { i32::MAX };
        if race && t.session_state == STATE_RACING {
            idxs.sort_by(|&a, &b| self.progress(b).total_cmp(&self.progress(a)));
        } else if pre_start {
            idxs.sort_by_key(|&i| (grid_pos(i), pos_or_max(t.cars[i].position), num(i)));
        } else if race {
            idxs.sort_by_key(|&i| {
                let p = if t.cars[i].position > 0 { t.cars[i].position } else { result_pos(i) };
                (pos_or_max(p), num(i))
            });
        } else {
            idxs.sort_by(|&a, &b| best(a).total_cmp(&best(b)).then(num(a).cmp(&num(b))));
        }

        let mut rows: Vec<Row> = Vec::with_capacity(idxs.len());
        let mut class_ids: Vec<String> = vec![];
        let mut class_count: Vec<i32> = vec![];
        let mut leader_of: Vec<usize> = vec![];
        let mut prev_of: Vec<usize> = vec![];
        for (n, &idx) in idxs.iter().enumerate() {
            let cid = &self.statics[idx].as_ref().unwrap().class.id;
            let ci = match class_ids.iter().position(|c| c == cid) {
                Some(ci) => ci,
                None => {
                    class_ids.push(cid.clone());
                    class_count.push(0);
                    leader_of.push(idx);
                    prev_of.push(idx);
                    class_ids.len() - 1
                }
            };
            class_count[ci] += 1;
            let mut row = Row { idx, pos: n as i32 + 1, class_pos: class_count[ci], ..Default::default() };
            if class_count[ci] > 1 {
                if race {
                    (row.gap, row.gap_laps) = self.time_between(idx, leader_of[ci]);
                    (row.interval, row.interval_laps) = self.time_between(idx, prev_of[ci]);
                } else if best(idx) < f32::MAX {
                    row.gap = Some(best(idx) - best(leader_of[ci]));
                    row.interval = Some(best(idx) - best(prev_of[ci]));
                }
            }
            if n > 0 {
                let (leader, prev) = (idxs[0], idxs[n - 1]);
                if race {
                    (row.gap_overall, row.gap_overall_laps) = self.time_between(idx, leader);
                    (row.interval_overall, row.interval_overall_laps) = self.time_between(idx, prev);
                } else if best(idx) < f32::MAX {
                    row.gap_overall = Some(best(idx) - best(leader));
                    row.interval_overall = Some(best(idx) - best(prev));
                }
            }
            prev_of[ci] = idx;
            rows.push(row);
        }
        self.order = rows;
        self.compute_classes();
    }

    fn compute_classes(&mut self) {
        let mut out: Vec<ClassSummary> = vec![];
        let mut irs: Vec<Vec<i32>> = vec![];
        for r in &self.order {
            let s = self.statics[r.idx].as_ref().unwrap();
            let i = match out.iter().position(|c| c.key.id == s.class.id) {
                Some(i) => i,
                None => {
                    out.push(ClassSummary { key: s.class.clone(), count: 0, sof: 0, best_lap: None });
                    irs.push(vec![]);
                    out.len() - 1
                }
            };
            out[i].count += 1;
            irs[i].push(s.irating);
            let b = self.best_lap(r.idx).unwrap_or(0.0);
            if b > 0.0 && out[i].best_lap.is_none_or(|(_, t)| b < t) {
                out[i].best_lap = Some((r.idx, b));
            }
        }
        for (c, ir) in out.iter_mut().zip(&irs) {
            c.sof = sof(ir);
        }
        out.sort_by(|a, b| a.key.order.total_cmp(&b.key.order));
        self.classes = out;
    }

    pub fn row(&self, idx: usize) -> Option<&Row> {
        self.order.iter().find(|r| r.idx == idx)
    }

    pub fn on_track(&self, idx: usize) -> bool {
        self.tel.cars[idx].surface == surface::ON_TRACK
    }
}

fn override_country(r: &Resolver, user_id: i64) -> String {
    r.league().and_then(|l| l.driver_overrides.iter().find(|o| o.customer_id == user_id)).map(|o| o.country.trim().to_uppercase()).unwrap_or_default()
}

/// Penalty / flag tracking that needs history (see [`Engine::track_incidents`]).
pub const YELLOW_LINGER: f64 = 5.0;
/// Off track for longer than this (seconds), or slower than `OFF_SLOW` (m/s) while off,
/// counts as causing a yellow.
pub const OFF_YELLOW: f64 = 1.0;
pub const OFF_SLOW: f32 = 20.0;

/// iRacing's strength-of-field formula.
pub fn sof(irs: &[i32]) -> i32 {
    let v: Vec<f64> = irs.iter().filter(|&&i| i > 0).map(|&i| i as f64).collect();
    if v.is_empty() {
        return 0;
    }
    let k = 1600.0 / std::f64::consts::LN_2;
    let sum: f64 = v.iter().map(|ir| (-ir / k).exp()).sum();
    (k * (v.len() as f64 / sum).ln()).round() as i32
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::CarTel;

    pub fn session_yaml(n: usize, race: bool) -> String {
        let mut s = String::from("WeekendInfo:\n TrackID: 1\n TrackDisplayName: Test\nSessionInfo:\n Sessions:\n - SessionNum: 0\n");
        s += &format!("   SessionType: {}\n   SessionLaps: 20\n", if race { "Race" } else { "Practice" });
        s += "DriverInfo:\n DriverCarIdx: 0\n DriverCarEstLapTime: 100.0\n Drivers:\n";
        for i in 0..n {
            let class = if i % 2 == 0 { 1 } else { 2 };
            s += &format!(" - CarIdx: {i}\n   UserName: Driver {i}\n   CarNumber: \"{i}\"\n   CarClassID: {class}\n   CarClassShortName: C{class}\n   CarClassEstLapTime: 100.0\n   IRating: {}\n", 1500 + i * 100);
        }
        s
    }

    pub fn engine(n: usize, race: bool) -> Engine {
        let mut e = Engine::new(Config::default(), None);
        e.set_session(SessionInfo::parse(&session_yaml(n, race)));
        e.tel.player_idx = 0;
        e.tel.session_state = STATE_RACING;
        e
    }

    pub fn place(e: &mut Engine, idx: usize, lap: i32, pct: f32) {
        e.tel.cars[idx] = CarTel { lap, pct, est_time: pct * 100.0, surface: surface::ON_TRACK, ..Default::default() };
    }

    #[test]
    fn lap_times_with_sectors_and_the_best_possible_lap() {
        let mut e = engine(2, true);
        e.info.track_length_km = 3.0;
        place(&mut e, 1, 1, 0.2);
        // You drive 4 laps of about 90 s, 10 updates a second; the third is over track limits
        // (Le Mans Ultimate) and the second has a quick middle sector.
        let mut t = 0.0f64;
        let mut lap_start = 0.0f64;
        let mut lap = 1;
        let mut pct = 0.0f64;
        while lap <= 4 {
            let speed = if lap == 2 && (0.34..0.66).contains(&pct) { 1.0 / 85.0 } else { 1.0 / 90.0 };
            t += 0.1;
            pct += speed * 0.1;
            if pct >= 1.0 {
                pct -= 1.0;
                lap += 1;
                e.tel.cars[0].last_lap = (t - lap_start) as f32;
                // The game's best lap (track-limit laps do not count).
                if lap != 4 && (e.tel.cars[0].best_lap <= 0.0 || e.tel.cars[0].last_lap < e.tel.cars[0].best_lap) {
                    e.tel.cars[0].best_lap = e.tel.cars[0].last_lap;
                }
                lap_start = t;
            }
            e.tel.session_time = t;
            e.tel.cars[0] = CarTel {
                lap,
                lap_completed: lap - 1,
                pct: pct as f32,
                surface: surface::ON_TRACK,
                last_lap: e.tel.cars[0].last_lap,
                best_lap: e.tel.cars[0].best_lap,
                ..Default::default()
            };
            e.tel.lap_invalid = lap == 3 && pct > 0.5;
            e.tel.lap_current_time = (t - lap_start) as f32;
            e.tick();
        }
        let out = crate::topics::laptimes(&e);
        let json = serde_json::to_string(&out).unwrap();
        // Newest first; lap 1's first sector is not timed (the timing starts with the car).
        let laps: Vec<i32> = out.laps.iter().map(|l| l.lap).collect();
        assert_eq!(laps, [4, 3, 2, 1], "{json}");
        assert_eq!(out.laps[3].sectors[0], None);
        let l2 = out.laps.iter().find(|l| l.lap == 2).unwrap();
        assert_eq!(l2.sectors.len(), 3);
        assert!(l2.sectors.iter().all(|s| s.is_some()), "{json}");
        assert!((l2.time.unwrap() - 88.6).abs() < 0.3, "official time: {json}");
        assert!(l2.pb, "the quickest lap is the personal best: {json}");
        assert!(out.laps.iter().find(|l| l.lap == 3).unwrap().invalid, "track limits");
        assert!(!l2.invalid);
        // Best possible lap: the best of each sector, from different laps.
        let theo = out.theoretical.unwrap();
        assert!(theo <= l2.time.unwrap() + 0.2 && theo > 85.0, "{json}");
        assert_eq!(out.current_lap, 5);
        assert!(out.current_time.is_some_and(|c| (0.0..10.0).contains(&c)));
    }

    #[test]
    fn league_teams_give_their_colour_short_name_and_logos() {
        use crate::config::{League, LeagueTeam};
        let mut e = engine(3, true);
        let cfg = Config {
            league_mode: "always".into(),
            active_league: Some("l".into()),
            leagues: vec![League {
                id: "l".into(),
                teams: vec![
                    LeagueTeam {
                        name: "Red Bull Racing".into(),
                        short: "RBR".into(),
                        color: "#1e41ff".into(),
                        logo: "0123456789abcdef.png".into(),
                        ..Default::default()
                    },
                    LeagueTeam { name: "No Colour GP".into(), ..Default::default() },
                ],
                ..Default::default()
            }],
            ..Default::default()
        };
        e.set_config(cfg);
        // iRacing's team names (team events): matched ignoring case.
        let y = session_yaml(3, true)
            .replace("UserName: Driver 1\n", "UserName: Driver 1\n   TeamName: red bull racing \n")
            .replace("UserName: Driver 2\n", "UserName: Driver 2\n   TeamName: No Colour GP\n");
        e.set_session(SessionInfo::parse(&y));
        let s1 = e.statics[1].as_ref().unwrap();
        assert_eq!((s1.team_color, s1.team_short.as_str(), s1.team_logo.as_str()), (Some(0x1e41ff), "RBR", "0123456789abcdef.png"));
        assert_eq!(e.statics[2].as_ref().unwrap().team_color, None, "a team with no colour");
        assert_eq!(e.statics[0].as_ref().unwrap().team_color, None, "no team");
        let json = crate::topics::Topic::Drivers.build(&e);
        assert!(json.contains(r##""teamColor":"#1e41ff","teamShort":"RBR","teamLogo":"/team-logos/0123456789abcdef.png","teamLogoSmall":"""##), "{json}");
        assert!(json.contains(r#""teamColor":"","teamShort":"","teamLogo":"","teamLogoSmall":"""#), "{json}");
    }

    #[test]
    fn drivers_details_only_resent_when_they_change() {
        let mut e = engine(4, true);
        let rev = e.statics_rev;
        // iRacing re-sends the session info with new results every few seconds.
        e.set_session(SessionInfo::parse(&session_yaml(4, true)));
        assert_eq!(e.statics_rev, rev, "same drivers: not sent again");
        e.set_session(SessionInfo::parse(&session_yaml(4, true).replace("UserName: Driver 2", "UserName: Driver Two")));
        assert_eq!(e.statics_rev, rev + 1, "a changed name is sent");
        assert_eq!(e.statics[2].as_ref().unwrap().name, "Driver Two");
    }

    #[test]
    fn fuel_put_in_by_the_player_in_litres() {
        let mut e = engine(2, true);
        place(&mut e, 0, 3, 0.95);
        place(&mut e, 1, 3, 0.50);
        e.tel.fuel_level = 8.0;
        e.tick();
        // In the pit box: 41.5 litres go in, then back out on track.
        e.tel.cars[0].on_pit_road = true;
        for l in [8.0, 7.9, 20.0, 35.0, 49.4] {
            e.tel.fuel_level = l;
            e.tick();
        }
        assert_eq!(e.hist[0].fuel_added().map(|a| a.0), Some(41.5));
        assert!(e.hist[0].fuel_filling);
        e.tel.cars[0].on_pit_road = false;
        e.tick();
        let a = crate::topics::extras(&e, 0).fuel_added.unwrap();
        assert_eq!((a.last, a.total, a.unit, a.filling), (41.5, 41.5, "L", false));
        // iRacing does not tell other cars' fuel.
        assert!(crate::topics::extras(&e, 1).fuel_added.is_none());
    }

    #[test]
    fn race_order_and_class_gaps() {
        let mut e = engine(4, true);
        place(&mut e, 0, 3, 0.50);
        place(&mut e, 1, 3, 0.60);
        place(&mut e, 2, 3, 0.40);
        place(&mut e, 3, 2, 0.90);
        e.tick();
        let order: Vec<usize> = e.order.iter().map(|r| r.idx).collect();
        assert_eq!(order, vec![1, 0, 2, 3]);
        // Overall: one leader (car 1); car 0 leads its class but is 10 s behind overall.
        assert_eq!(e.row(1).unwrap().gap_overall, None);
        let r0 = e.row(0).unwrap();
        assert_eq!(r0.class_pos, 1);
        assert!((r0.gap_overall.unwrap() - 10.0).abs() < 1e-3, "{:?}", r0.gap_overall);
        assert!((r0.interval_overall.unwrap() - 10.0).abs() < 1e-3);
        let r2 = e.row(2).unwrap();
        assert!((r2.gap_overall.unwrap() - 20.0).abs() < 1e-3, "{:?}", r2.gap_overall);
        assert!((r2.interval_overall.unwrap() - 10.0).abs() < 1e-3);
        assert_eq!(r2.class_pos, 2);
        assert!((r2.gap.unwrap() - 10.0).abs() < 1e-3, "{:?}", r2.gap);
        let r3 = e.row(3).unwrap();
        assert_eq!(r3.class_pos, 2);
        assert!((r3.gap.unwrap() - 70.0).abs() < 1e-3, "{:?}", r3.gap);
        place(&mut e, 3, 1, 0.90);
        e.tick();
        assert_eq!(e.row(3).unwrap().gap_laps, 1);
    }

    #[test]
    fn standings_list_drivers_in_the_garage() {
        for race in [false, true] {
            let mut e = engine(3, race);
            place(&mut e, 0, 3, 0.5);
            place(&mut e, 1, 3, 0.6);
            // Car 2 is connected but has never left the garage.
            e.tel.cars[2] = CarTel { surface: surface::NOT_IN_WORLD, pct: -1.0, ..Default::default() };
            e.tick();
            let mut order: Vec<usize> = e.order.iter().map(|r| r.idx).collect();
            order.sort();
            assert_eq!(order, vec![0, 1, 2], "race: {race}");
            assert_eq!(e.order.last().map(|r| r.idx), Some(2), "in the garage, listed last");
        }
    }

    #[test]
    fn relative_wraps_across_line() {
        let mut e = engine(2, true);
        place(&mut e, 0, 3, 0.95);
        place(&mut e, 1, 4, 0.05);
        assert!((e.relative_gap(0, 1) - 10.0).abs() < 1e-3);
        assert!((e.relative_gap(1, 0) + 10.0).abs() < 1e-3);
    }

    #[test]
    fn chequered_flag_laps_led_and_bonus_points() {
        use crate::config::{League, PointsConfig};
        let cfg = Config {
            leagues: vec![League {
                id: "l".into(),
                points: PointsConfig {
                    table: vec![25.0, 18.0, 15.0],
                    basis: "overall".into(),
                    lap_led_bonus: 1.0,
                    most_laps_led_bonus: 2.0,
                    finish_bonus: 0.5,
                    ..Default::default()
                },
                ..Default::default()
            }],
            active_league: Some("l".into()),
            league_mode: "always".into(),
            ..Default::default()
        };
        let mut e = engine(3, true);
        e.set_config(cfg);
        let lap = |e: &mut Engine, t: f64, cars: [(i32, f32); 3]| {
            e.tel.session_time = t;
            for (i, (l, p)) in cars.into_iter().enumerate() {
                place(e, i, l, p);
                e.tel.cars[i].lap_completed = l - 1;
            }
            e.tick();
        };
        lap(&mut e, 0.0, [(1, 0.95), (1, 0.90), (1, 0.85)]);
        // Car 0 leads across the line twice.
        lap(&mut e, 10.0, [(2, 0.01), (1, 0.97), (1, 0.93)]);
        lap(&mut e, 110.0, [(3, 0.01), (2, 0.97), (2, 0.93)]);
        assert_eq!(e.hist[0].laps_led_overall, 2);
        assert_eq!(e.points(0).0, Some(25.0 + 1.0 + 2.0 + 0.5));
        assert_eq!(e.points(1).0, Some(18.5));
        // Chequered flag: the leader finishes, the others when they next cross the line.
        e.tel.session_state = STATE_CHECKERED;
        lap(&mut e, 210.0, [(4, 0.01), (2, 0.99), (2, 0.98)]);
        assert!(e.hist[0].finished);
        assert!(!e.hist[1].finished);
        lap(&mut e, 212.0, [(4, 0.03), (3, 0.01), (2, 0.99)]);
        assert!(e.hist[1].finished);
        assert!(!e.hist[2].finished);
    }

    #[test]
    fn gaps_do_not_jump_at_the_line() {
        let mut e = engine(2, true);
        e.tick();
        // Real pace (110 s) differs from the sim's class estimate (100 s, used by est_time).
        e.hist[0].laps.push_back(LapRec { time: 110.0, wet: None, at: 0.0, pit: false });
        let mut last: Option<(f32, f32)> = None;
        // Car 1 runs 4% ahead of car 0 and crosses the line during this.
        for k in 0..40 {
            let p0 = 0.94 + k as f32 * 0.001;
            let p1 = p0 + 0.04;
            place(&mut e, 0, 3, p0);
            let (lap1, p1) = if p1 >= 1.0 { (4, p1 - 1.0) } else { (3, p1) };
            place(&mut e, 1, lap1, p1);
            e.tick();
            let rel = e.relative_gap(0, 1);
            let race = e.time_between(0, 1).0.unwrap();
            assert!((rel - 4.4).abs() < 0.05, "relative {rel} at {k}");
            if let Some((r0, t0)) = last {
                assert!((rel - r0).abs() < 0.05 && (race - t0).abs() < 0.05, "jump at {k}: {r0}->{rel}, {t0}->{race}");
            }
            last = Some((rel, race));
        }
    }

    #[test]
    fn grid_from_qualifying_before_the_start() {
        use crate::model::ResultPos;
        let mut e = engine(4, true);
        let mut info = e.info.clone();
        info.qualify = [3usize, 1, 0, 2].iter().enumerate().map(|(p, &car_idx)| ResultPos { position: p as i32 + 1, car_idx, ..Default::default() }).collect();
        e.set_session(info);
        // Parade laps; only two cars have loaded in so far.
        e.tel.session_state = 3;
        for car in e.tel.cars.iter_mut().take(4) {
            *car = Default::default();
            car.pct = -1.0;
        }
        place(&mut e, 0, 0, 0.9);
        place(&mut e, 2, 0, 0.95);
        e.tick();
        let order: Vec<usize> = e.order.iter().map(|r| r.idx).collect();
        assert_eq!(order, [3, 1, 0, 2]);
        // iRating estimates use the whole field from the start, not just the cars loaded in.
        let total: f32 = (0..4).filter_map(|i| e.ir_delta(i)).sum();
        assert!(total.abs() < 5.0, "{total}");
        assert!(e.ir_delta(3).unwrap() > 0.0 && e.ir_delta(2).unwrap() < 0.0);
        assert!((0..4).all(|i| e.ir_delta(i).unwrap().abs() < 150.0));
    }

    #[test]
    fn touring_car_is_tcr() {
        let mut y = session_yaml(2, true);
        y = y.replace("CarClassShortName: C", "CarClassShortName: Touring Car C");
        let mut e = engine(2, true);
        e.set_session(SessionInfo::parse(&y.replace("Touring Car C0", "Touring Car").replace("Touring Car C1", "Touring Car")));
        assert_eq!(e.static_of(0).unwrap().class.name, "TCR");
    }

    #[test]
    fn league_follows_the_session() {
        use crate::config::League;
        let league = |id: &str, lid: i64| League { id: id.into(), name: id.into(), iracing_league_id: lid, ..Default::default() };
        let cfg = Config { leagues: vec![league("a", 111), league("b", 0)], active_league: Some("b".into()), ..Default::default() };
        let mut e = engine(2, true);
        e.set_config(cfg.clone());
        // Official race (no league ID): no league, even though one is selected.
        assert!(e.resolver.league().is_none());
        let mut info = e.info.clone();
        info.league_id = 111;
        e.set_session(info);
        assert_eq!(e.resolver.league_name(), Some("a"));
        // Another league's session: none of ours.
        let mut info = e.info.clone();
        info.league_id = 222;
        e.set_session(info);
        assert!(e.resolver.league().is_none());
        // "Always use" keeps the selected league whatever the session.
        e.set_config(Config { league_mode: "always".into(), ..cfg });
        assert_eq!(e.resolver.league_name(), Some("b"));
    }

    #[test]
    fn a_quick_cut_is_not_a_yellow() {
        let mut e = engine(2, true);
        let mut info = e.info.clone();
        info.track_length_km = 5.8;
        e.set_session(info);
        let step = |e: &mut Engine, t: f64, pct: f32, surf: i32| {
            e.tel.session_time = t;
            place(e, 0, 3, 0.9);
            place(e, 1, 3, pct);
            e.tel.cars[1].surface = surf;
            e.tick();
        };
        let mut t = 0.0;
        let mut pct = 0.1;
        // About 58 m/s (0.0005 of a 5.8 km lap every 0.05 s).
        let mut drive = |e: &mut Engine, secs: f64, surf: i32| {
            let end = t + secs;
            while t < end {
                t += 0.05;
                pct += 0.0005;
                step(e, t, pct, surf);
            }
        };
        drive(&mut e, 2.0, surface::ON_TRACK);
        // Over the kerbs for half a second at speed: no yellow.
        drive(&mut e, 0.5, surface::OFF_TRACK);
        drive(&mut e, 0.5, surface::ON_TRACK);
        assert!(e.car_flags(1).is_empty(), "{:?}", e.car_flags(1));
        // Off for two seconds: a yellow.
        drive(&mut e, 2.0, surface::OFF_TRACK);
        assert_eq!(e.car_flags(1), vec!["yellow"]);
    }

    #[test]
    fn yellow_and_black_flag_penalties() {
        let mut e = engine(2, true);
        let step = |e: &mut Engine, t: f64, pct: f32, surf: i32, pit: bool, flags: u32| {
            e.tel.session_time = t;
            place(e, 0, 3, 0.5);
            place(e, 1, 3, pct);
            e.tel.cars[1].surface = surf;
            e.tel.cars[1].on_pit_road = pit;
            e.tel.cars[1].session_flags = flags;
            e.tick();
        };
        // Driving normally: no flags.
        for k in 0..10 {
            step(&mut e, k as f64 * 0.25, 0.1 + k as f32 * 0.002, surface::ON_TRACK, false, 0);
        }
        assert!(e.car_flags(1).is_empty());
        // Off track: causes a yellow that lingers after rejoining.
        step(&mut e, 3.0, 0.13, surface::OFF_TRACK, false, 0);
        step(&mut e, 4.0, 0.14, surface::ON_TRACK, false, 0);
        assert_eq!(e.car_flags(1), vec!["yellow"]);
        // Black flag served as a stop & go of ~3 seconds.
        let mut t = 20.0;
        step(&mut e, t, 0.98, surface::APPROACHING_PITS, true, flags::BLACK);
        assert_eq!(e.car_flags(1), vec!["black"]);
        for _ in 0..30 {
            t += 0.1;
            step(&mut e, t, 0.01, surface::IN_PIT_STALL, true, flags::BLACK);
        }
        assert!(e.penalty(1).unwrap().0.starts_with("S&G"));
        step(&mut e, t + 0.1, 0.02, surface::APPROACHING_PITS, true, 0);
        let (label, served) = e.penalty(1).unwrap();
        assert_eq!(label, "S&G 3s");
        assert!(served);
        // A race-control penalty takes precedence.
        let mut cfg = e.cfg.clone();
        cfg.penalties = vec![crate::config::Penalty { customer_id: e.static_of(1).unwrap().user_id, kind: "sg".into(), seconds: 40.0, ..Default::default() }];
        e.set_config(cfg);
        assert_eq!(e.penalty(1).unwrap().0, "S&G 40s");
    }

    #[test]
    fn pit_time_service_and_penalty() {
        let mut e = engine(2, true);
        let step = |e: &mut Engine, t: f64, pct: f32, surf: i32, pit: bool, flags: u32| {
            e.tel.session_time = t;
            place(e, 0, 3, 0.5);
            place(e, 1, 3, pct);
            e.tel.cars[1].surface = surf;
            e.tel.cars[1].on_pit_road = pit;
            e.tel.cars[1].session_flags = flags;
            e.tick();
        };
        let mut t = 0.0;
        // Drives down the pit lane (not counted), stands in the box for 20 s, drives out.
        let stop = |e: &mut Engine, t: &mut f64, secs: f64, black_for: f64| {
            for k in 0..20 {
                *t += 0.1;
                step(e, *t, 0.95 + k as f32 * 0.002, surface::APPROACHING_PITS, true, if black_for > 0.0 { flags::BLACK } else { 0 });
            }
            let start = *t;
            while *t < start + secs {
                *t += 0.1;
                let f = if *t - start < black_for { flags::BLACK } else { 0 };
                step(e, *t, 0.01, surface::IN_PIT_STALL, true, f);
            }
            // Live while the stop goes on.
            assert!(e.hist[1].pit_time().unwrap().1);
            for k in 0..20 {
                *t += 0.1;
                step(e, *t, 0.012 + k as f32 * 0.002, surface::ON_TRACK, true, 0);
            }
            *t += 0.1;
            step(e, *t, 0.06, surface::ON_TRACK, false, 0);
            e.hist[1].pit_time().unwrap()
        };
        assert!(e.hist[1].pit_time().is_none());
        // A service stop.
        let (p, live) = stop(&mut e, &mut t, 20.0, 0.0);
        assert!(!live);
        assert!((p.service - 20.0).abs() < 0.5 && p.penalty == 0.0, "{p:?}");
        let x = crate::topics::extras(&e, 1).pit_time.unwrap();
        assert!(!x.live && (x.service - 20.0).abs() < 0.5);
        // A stop-and-go: black-flagged the whole time it stands there.
        t += 60.0;
        let (p, _) = stop(&mut e, &mut t, 10.0, 99.0);
        assert!((p.penalty - 10.0).abs() < 0.5 && p.service == 0.0, "{p:?}");
        // The penalty served first, then serviced in the same stop.
        t += 60.0;
        let (p, _) = stop(&mut e, &mut t, 40.0, 10.0);
        assert!((p.penalty - 10.0).abs() < 0.5 && (p.service - 30.0).abs() < 0.5, "{p:?}");
        // The other car never stopped.
        assert!(e.hist[0].pit_time().is_none());
    }

    #[test]
    fn strength_of_field() {
        assert_eq!(sof(&[2000, 2000]), 2000);
        let s = sof(&[1000, 3000]);
        assert!(s > 1000 && s < 3000);
    }
}

#[cfg(test)]
mod perf {
    use super::tests::{engine, place};
    use crate::topics::Topic;

    /// `cargo test --release -p apex-core perf -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn tick_and_topics() {
        let n = 60;
        let mut e = engine(n, true);
        e.info.track_length_km = 5.0;
        let reps = 3000;
        let t0 = std::time::Instant::now();
        for k in 0..reps {
            e.tel.session_time = k as f64 / 60.0;
            for i in 0..n {
                place(&mut e, i, 3 + (k / 600) as i32, ((i as f32 * 0.013 + k as f32 * 0.0004) % 1.0).abs());
            }
            e.tick();
        }
        let tick = t0.elapsed() / reps;
        println!("tick: {tick:?}");
        // A full-size iRacing session string (60 drivers, three sessions of results, the setup):
        // re-sent every few seconds in a race.
        let y = big_session_yaml(n);
        let reps = 50;
        let t0 = std::time::Instant::now();
        for _ in 0..reps {
            e.set_session(crate::model::SessionInfo::parse(&y));
        }
        println!("session info {} KB: {:?}", y.len() / 1024, t0.elapsed() / reps);
        for t in Topic::ALL {
            let t0 = std::time::Instant::now();
            let mut len = 0;
            for _ in 0..300 {
                len = t.build(&e).len();
            }
            println!("{:10} {:?}  {} bytes", t.name(), t0.elapsed() / 300, len);
        }
    }

    fn big_session_yaml(n: usize) -> String {
        let mut s = String::from("---\nWeekendInfo:\n TrackName: spa 2024 up\n TrackID: 525\n TrackLength: 6.93 km\n TrackDisplayName: Circuit de Spa-Francorchamps\n TrackConfigName: Grand Prix\n SeriesID: 228\n SubSessionID: 71234567\n LeagueID: 0\n Official: 1\n EventType: Race\n WeekendOptions:\n  StandingStart: 1\n  IncidentLimit: 25\n");
        s += "SessionInfo:\n Sessions:\n";
        for (num, kind) in ["Practice", "Qualify", "Race"].iter().enumerate() {
            s += &format!(" - SessionNum: {num}\n   SessionLaps: unlimited\n   SessionTime: 1800.0000 sec\n   SessionType: {kind}\n   SessionName: {}\n   SessionTrackRubberState: moderate usage\n   ResultsPositions:\n", kind.to_uppercase());
            for i in 0..n {
                s += &format!("   - Position: {}\n     ClassPosition: {}\n     CarIdx: {i}\n     Lap: 12\n     Time: 1234.5678\n     FastestLap: 7\n     FastestTime: 138.1234\n     LastTime: 139.4321\n     LapsLed: 0\n     LapsComplete: 12\n     JokerLapsComplete: 0\n     LapsDriven: 12.0000\n     Incidents: 2\n     ReasonOutId: 0\n     ReasonOutStr: Running\n", i + 1, i / 2);
            }
            s += "   ResultsFastestLap:\n   - CarIdx: 3\n     FastestLap: 7\n     FastestTime: 137.9000\n   ResultsAverageLapTime: 140.1234\n   ResultsNumCautionFlags: 0\n   ResultsNumCautionLaps: 0\n   ResultsNumLeadChanges: 0\n   ResultsLapsComplete: 12\n   ResultsOfficial: 0\n";
        }
        s += "DriverInfo:\n DriverCarIdx: 0\n DriverCarEstLapTime: 138.5\n Drivers:\n";
        for i in 0..n {
            s += &format!(" - CarIdx: {i}\n   UserName: Firstname Lastname{i}\n   AbbrevName: Lastname{i}, F\n   Initials: FL\n   UserID: {}\n   TeamID: 0\n   TeamName: Firstname Lastname{i}\n   CarNumber: \"{i}\"\n   CarNumberRaw: {i}\n   CarPath: bmwm4gt3\n   CarClassID: 4083\n   CarID: 132\n   CarIsPaceCar: 0\n   CarIsAI: 0\n   CarIsElectric: 0\n   CarScreenName: BMW M4 GT3\n   CarScreenNameShort: BMW M4 GT3\n   CarClassShortName: GT3 Class\n   CarClassRelSpeed: 0\n   CarClassLicenseLevel: 0\n   CarClassMaxFuelPct: 1.000 %\n   CarClassWeightPenalty: 0.000 kg\n   CarClassPowerAdjust: 0.000 %\n   CarClassDryTireSetLimit: 0 %\n   CarClassColor: 0xffda59\n   CarClassEstLapTime: 138.5000\n   IRating: {}\n   LicLevel: 18\n   LicSubLevel: 349\n   LicString: A 3.49\n   LicColor: 0x0153db\n   IsSpectator: 0\n   CarDesignStr: 1,ffffff,000000,ff0000\n   HelmetDesignStr: 1,ffffff,000000,ff0000\n   SuitDesignStr: 1,ffffff,000000,ff0000\n   BodyType: 0\n   FaceType: 0\n   HelmetType: 0\n   CarNumberDesignStr: 0,0,ffffff,777777,000000\n   CarSponsor_1: 0\n   CarSponsor_2: 0\n   ClubName: Benelux\n   ClubID: 7\n   DivisionName: Division 3\n   DivisionID: 2\n   FlairName: Belgium\n   FlairID: 22\n   CurDriverIncidentCount: 2\n   TeamIncidentCount: 2\n", 100000 + i, 1500 + i * 30);
        }
        s += "CarSetup:\n UpdateCount: 3\n";
        for k in 0..120 {
            s += &format!(" Setting{k}: {k}.00 mm\n");
        }
        s
    }
}
