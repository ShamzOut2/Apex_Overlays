//! Per-lap fuel consumption tracking and race fuel calculator.

use crate::config::FuelConfig;
use crate::model::Telemetry;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct FuelLap {
    pub used: f32,
    pub time: f32,
    pub valid: bool,
}

#[derive(Debug, Default)]
pub struct FuelTracker {
    lap_start_fuel: Option<f32>,
    lap_start_completed: i32,
    pit: bool,
    caution: bool,
    refuel: bool,
    prev_fuel: f32,
    prev_lap_time: f32,
    pub laps: VecDeque<FuelLap>,
    /// Laps counted so far (also past the 50 kept in `laps`), to notice new ones.
    pub counted: u64,
}

impl FuelTracker {
    pub fn new_session(&mut self) {
        // Keep consumption history across sessions (practice data is useful for the race),
        // but start lap tracking afresh.
        self.lap_start_fuel = None;
    }

    pub fn update(&mut self, t: &Telemetry, caution: bool, cfg: &FuelConfig) {
        self.update_level(t.fuel_level, t, caution, cfg);
    }

    /// The same for another level than the fuel's (Le Mans Ultimate's virtual energy, in %).
    pub fn update_level(&mut self, fuel: f32, t: &Telemetry, caution: bool, cfg: &FuelConfig) {
        if fuel > self.prev_fuel + 0.05 {
            self.refuel = true;
        }
        self.pit |= t.on_pit_road;
        self.caution |= caution;
        if t.lap_completed != self.lap_start_completed {
            if let Some(start) = self.lap_start_fuel {
                if t.lap_completed == self.lap_start_completed + 1 {
                    let used = start - fuel;
                    let valid =
                        used > 0.0 && !self.refuel && t.is_on_track && (cfg.include_pit_laps || !self.pit) && (cfg.include_caution_laps || !self.caution);
                    self.laps.push_back(FuelLap { used, time: self.prev_lap_time, valid });
                    self.counted += 1;
                    if self.laps.len() > 50 {
                        self.laps.pop_front();
                    }
                }
            }
            self.lap_start_fuel = t.is_on_track.then_some(fuel);
            self.lap_start_completed = t.lap_completed;
            self.pit = t.on_pit_road;
            self.caution = caution;
            self.refuel = false;
        }
        self.prev_fuel = fuel;
        self.prev_lap_time = t.lap_current_time;
    }

    /// Forget the laps (another car or track: their consumption does not apply).
    pub fn clear(&mut self) {
        self.laps.clear();
        self.lap_start_fuel = None;
    }

    pub fn valid(&self, n: usize) -> impl Iterator<Item = &FuelLap> {
        self.laps.iter().rev().filter(|l| l.valid).take(n.max(1))
    }

    pub fn average(&self, n: usize) -> Option<f32> {
        let (sum, cnt) = self.valid(n).fold((0.0, 0), |(s, c), l| (s + l.used, c + 1));
        (cnt > 0).then(|| sum / cnt as f32)
    }

    pub fn max(&self, n: usize) -> Option<f32> {
        self.valid(n).map(|l| l.used).reduce(f32::max)
    }

    pub fn lap_time(&self, n: usize) -> Option<f32> {
        let v: Vec<f32> = self.valid(n).map(|l| l.time).filter(|&t| t > 0.0).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f32>() / v.len() as f32)
    }
}

/// What a car used per lap at a track in earlier sessions. Small on purpose: a running
/// average rather than every lap.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ComboRecord {
    pub car: String,
    pub track: String,
    /// Running average over (about) the last 20 valid laps.
    pub per_lap: f32,
    pub max: f32,
    pub lap_time: f32,
    pub laps: u32,
    /// Unix time of the last lap counted.
    pub updated: i64,
}

/// Fuel use per car and track combination, kept on disk between sessions (a few KB).
#[derive(Debug, Default)]
pub struct History {
    path: Option<PathBuf>,
    pub combos: BTreeMap<String, ComboRecord>,
}

/// Combinations kept; the least recently driven are dropped beyond this.
const MAX_COMBOS: usize = 300;
/// Laps the running average remembers.
const WINDOW: f32 = 20.0;

impl History {
    pub fn load(path: Option<PathBuf>) -> Self {
        let combos = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Self { path, combos }
    }

    pub fn key(car_id: i64, track_id: i64) -> String {
        format!("{car_id}:{track_id}")
    }

    pub fn get(&self, key: &str) -> Option<&ComboRecord> {
        self.combos.get(key).filter(|c| c.laps > 0 && c.per_lap > 0.0)
    }

    /// Count one valid lap and save.
    pub fn add(&mut self, key: &str, car: &str, track: &str, lap: &FuelLap, now: i64) {
        let c = self.combos.entry(key.to_string()).or_default();
        c.car = car.to_string();
        c.track = track.to_string();
        let n = (c.laps as f32).min(WINDOW - 1.0);
        c.per_lap = (c.per_lap * n + lap.used) / (n + 1.0);
        // Highest recent lap, slowly forgetting old peaks.
        c.max = if c.laps == 0 { lap.used } else { (c.max * 0.99).max(lap.used) };
        if lap.time > 0.0 {
            c.lap_time = if c.lap_time > 0.0 { (c.lap_time * n + lap.time) / (n + 1.0) } else { lap.time };
        }
        c.laps += 1;
        c.updated = now;
        if self.combos.len() > MAX_COMBOS {
            if let Some(oldest) = self.combos.iter().min_by_key(|(_, c)| c.updated).map(|(k, _)| k.clone()) {
                self.combos.remove(&oldest);
            }
        }
        self.save();
    }

    fn save(&self) {
        let Some(p) = &self.path else { return };
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, serde_json::to_string(&self.combos).unwrap_or_default());
    }
}

/// Inputs for the race-length estimate.
pub struct RaceClock {
    /// Total race laps (laps-limited race).
    pub laps_total: Option<i32>,
    /// Seconds left in a timed session.
    pub time_remaining: Option<f64>,
    pub leader_progress: f64,
    pub leader_lap: f32,
    pub player_progress: f64,
    pub player_lap: f32,
}

/// Fractional laps the player still has to drive, including the current one.
pub fn laps_remaining(c: &RaceClock) -> Option<f64> {
    if c.leader_lap <= 0.0 || c.player_lap <= 0.0 {
        return None;
    }
    let ll = c.leader_lap as f64;
    let mut t_end: Option<f64> = None;
    if let Some(total) = c.laps_total {
        t_end = Some(((total as f64) - c.leader_progress).max(0.0) * ll);
    }
    if let Some(rem) = c.time_remaining.filter(|r| *r >= 0.0 && *r < 1e6) {
        // When the clock hits zero the leader still completes the lap they are on.
        let at_zero = c.leader_progress + rem / ll;
        let finish = ((at_zero - 1e-3).ceil() - c.leader_progress).max(0.0) * ll;
        t_end = Some(t_end.map_or(finish, |t| t.min(finish)));
    }
    let t_end = t_end?;
    let p_end = c.player_progress + t_end / c.player_lap as f64;
    Some(((p_end - 1e-3).ceil() - c.player_progress).max(0.0))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    pub fuel_to_finish: f32,
    pub fuel_to_add: f32,
    pub stops: i32,
    pub laps_per_tank: f32,
    /// Consumption per lap that would make the current fuel last to the flag.
    pub target_per_lap: f32,
    /// Laps from now until a single stop can fill to the end (pit window opens).
    pub window_opens_in: f32,
    /// Laps from now until the tank runs dry (pit window closes).
    pub window_closes_in: f32,
}

pub fn plan(level: f32, capacity: f32, per_lap: f32, laps_left: f32, margin_laps: f32) -> Plan {
    if per_lap <= 0.0 {
        return Plan::default();
    }
    let need = (laps_left + margin_laps) * per_lap;
    let short = (need - level).max(0.0);
    let stops = if short <= 0.0 {
        0
    } else if capacity > 0.0 {
        (short / capacity).ceil() as i32
    } else {
        1
    };
    let laps_per_tank = if capacity > 0.0 { capacity / per_lap } else { 0.0 };
    Plan {
        fuel_to_finish: need,
        fuel_to_add: if capacity > 0.0 { short.min(capacity - level.min(capacity)) } else { short },
        stops,
        laps_per_tank,
        target_per_lap: if laps_left > 0.0 { level / (laps_left + margin_laps) } else { 0.0 },
        window_opens_in: (laps_left + margin_laps - laps_per_tank).max(0.0),
        window_closes_in: level / per_lap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_keeps_a_running_average() {
        let dir = std::env::temp_dir().join(format!("apex-fuel-{}", std::process::id()));
        let path = dir.join("fuel-history.json");
        let mut h = History::load(Some(path.clone()));
        let key = History::key(132, 18);
        for (i, used) in [3.0, 3.2, 2.8, 3.0].iter().enumerate() {
            h.add(&key, "BMW M4 GT3", "Road America", &FuelLap { used: *used, time: 140.0, valid: true }, i as i64);
        }
        let c = h.get(&key).unwrap();
        assert!((c.per_lap - 3.0).abs() < 1e-4 && c.laps == 4 && c.max > 3.1 && c.max < 3.2);
        // Saved and loaded again.
        let again = History::load(Some(path));
        assert_eq!(again.get(&key), Some(c));
        // Older laps fade out of the running average.
        let mut h = again;
        for i in 0..60 {
            h.add(&key, "BMW M4 GT3", "Road America", &FuelLap { used: 2.5, time: 140.0, valid: true }, 10 + i);
        }
        assert!((h.get(&key).unwrap().per_lap - 2.5).abs() < 0.05);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn lap_race_remaining() {
        let c = RaceClock { laps_total: Some(20), time_remaining: None, leader_progress: 10.25, leader_lap: 100.0, player_progress: 10.25, player_lap: 100.0 };
        assert!((laps_remaining(&c).unwrap() - 9.75).abs() < 1e-6);
        // Lapped player finishes when crossing the line after the leader.
        let c = RaceClock { player_progress: 9.5, player_lap: 105.0, ..c };
        assert!((laps_remaining(&c).unwrap() - 9.5).abs() < 1e-6);
    }

    #[test]
    fn timed_race_remaining() {
        let c = RaceClock { laps_total: None, time_remaining: Some(250.0), leader_progress: 5.0, leader_lap: 100.0, player_progress: 5.0, player_lap: 100.0 };
        // 2.5 laps at zero → leader finishes lap 8, i.e. 3 more laps.
        assert!((laps_remaining(&c).unwrap() - 3.0).abs() < 1e-6);
    }

    #[test]
    fn fuel_plan() {
        let p = plan(30.0, 100.0, 2.0, 40.0, 1.0);
        assert_eq!(p.fuel_to_finish, 82.0);
        assert_eq!(p.fuel_to_add, 52.0);
        assert_eq!(p.stops, 1);
        assert_eq!(p.window_closes_in, 15.0);
        let p = plan(90.0, 100.0, 2.0, 100.0, 0.0);
        assert_eq!(p.stops, 2);
        assert_eq!(p.fuel_to_add, 10.0);
    }

    #[test]
    fn tracks_consumption() {
        let mut f = FuelTracker::default();
        let cfg = FuelConfig::default();
        let mut t = Telemetry { is_on_track: true, fuel_level: 50.0, ..Default::default() };
        f.update(&t, false, &cfg);
        for lap in 1..=3 {
            t.fuel_level -= 2.5;
            t.lap_current_time = 90.0;
            f.update(&t, false, &cfg);
            t.lap_completed = lap;
            t.lap_current_time = 0.0;
            f.update(&t, false, &cfg);
        }
        // First lap starts mid-way (no lap start reference) so only 2 full laps count.
        assert_eq!(f.laps.len(), 2);
        assert!((f.average(5).unwrap() - 2.5).abs() < 1e-4);
        assert_eq!(f.lap_time(5), Some(90.0));
    }
}
