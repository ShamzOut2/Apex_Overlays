//! Pit lane helper: where the pit entry and exit lines are, and what the player is doing
//! on the way in, in the lane, in the box and on the way out.
//!
//! The lines are learned from any car crossing them (iRacing's on-pit-road flag switching
//! while the car drives through) and kept per track on disk; until then Lovely Track Data
//! (or the mock's track) gives an estimate.

use crate::model::{surface, Telemetry, MAX_CARS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Samples kept per line (the median is used).
const SAMPLES: usize = 15;
/// The "pit exit" sign shows as soon as the car leaves its box, or this close to the line
/// without stopping (a drive-through): a box can be close to the exit.
const EXIT_SHOW_M: f32 = 60.0;
/// irsdk_EngineWarnings: pit speed limiter on.
const LIMITER: u32 = 0x10;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Lines {
    entry: Vec<f32>,
    exit: Vec<f32>,
}

fn median(v: &[f32]) -> Option<f32> {
    let mut s = v.to_vec();
    s.sort_by(f32::total_cmp);
    s.get(s.len() / 2).copied()
}

/// Circular midpoint of two lap fractions a tick apart.
fn between(a: f32, b: f32) -> f32 {
    let d = (b - a + 1.5).rem_euclid(1.0) - 0.5;
    (a + d / 2.0).rem_euclid(1.0)
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PitOut {
    /// "" (nothing to show), "entry" (heading for the pit lane), "lane" (in the pit lane),
    /// "box" (stopped in the pit box), "exit" (heading for the exit line), "out" (just left).
    pub phase: &'static str,
    /// Metres to the pit entry / exit line (the speed limit lines), when known.
    pub to_entry: Option<f32>,
    pub to_exit: Option<f32>,
    /// m/s
    pub speed: f32,
    /// Pit lane speed limit, m/s (0 = unknown).
    pub limit: f32,
    pub limiter: bool,
    /// In the box: "wait" while being serviced, "go" when done.
    pub service: &'static str,
    /// Repair time left (s): mandatory and optional.
    pub repair: f32,
    pub opt_repair: f32,
    /// Seconds since the car crossed the pit exit line.
    pub since_exit: Option<f32>,
}

#[derive(Debug, Default)]
pub struct PitTracker {
    file: Option<PathBuf>,
    all: BTreeMap<i64, Lines>,
    track: i64,
    lines: Lines,
    prev: Vec<(bool, f32, i32)>,
    prev_t: f64,
    /// The player's visit to the pit lane: stopped in the box yet, box arrival, service seen.
    left_box: bool,
    box_since: Option<f64>,
    service_seen: bool,
    exit_at: Option<f64>,
    was_on_pit: bool,
    /// Last time the player was on the pit entry road.
    approach_at: Option<f64>,
    /// The player's lap fraction and the time, last tick (to tell driving from being moved).
    last: Option<(f32, f64)>,
}

impl PitTracker {
    pub fn new(file: Option<PathBuf>) -> Self {
        let all = file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Self { file, all, ..Default::default() }
    }

    /// Learned entry and exit lines (lap fractions).
    pub fn learned(&self) -> (Option<f32>, Option<f32>) {
        (median(&self.lines.entry), median(&self.lines.exit))
    }

    fn save(&mut self) {
        self.all.insert(self.track, self.lines.clone());
        if let Some(f) = &self.file {
            let _ = std::fs::write(f, serde_json::to_string(&self.all).unwrap_or_default());
        }
    }

    /// Learn the lines from every car crossing them, then follow the player.
    pub fn update(&mut self, t: &Telemetry, track_id: i64) {
        if track_id != self.track {
            self.track = track_id;
            self.lines = self.all.get(&track_id).cloned().unwrap_or_default();
            self.prev.clear();
        }
        let dt = t.session_time - self.prev_t;
        self.prev_t = t.session_time;
        // Only consecutive ticks: a jump (replay, reconnect) is no crossing.
        let fresh = (0.0..0.5).contains(&dt) && self.prev.len() == MAX_CARS && !t.is_replay_playing;
        let mut changed = false;
        for (i, c) in t.cars.iter().enumerate().take(MAX_CARS) {
            if fresh {
                let (was_pit, was_pct, was_surface) = self.prev[i];
                let step = (c.pct - was_pct + 1.5).rem_euclid(1.0) - 0.5;
                // Driving through (not towed or reset): a small forward step on track surfaces.
                let driving = c.pct >= 0.0 && was_pct >= 0.0 && (0.0..0.005).contains(&step);
                let rolling = |s: i32| s == surface::ON_TRACK || s == surface::APPROACHING_PITS;
                if driving && !was_pit && c.on_pit_road && rolling(was_surface) {
                    self.lines.entry.push(between(was_pct, c.pct));
                    changed = true;
                } else if driving && was_pit && !c.on_pit_road && rolling(c.surface) {
                    self.lines.exit.push(between(was_pct, c.pct));
                    changed = true;
                }
            }
        }
        if changed {
            for v in [&mut self.lines.entry, &mut self.lines.exit] {
                if v.len() > SAMPLES {
                    v.drain(..v.len() - SAMPLES);
                }
            }
            self.save();
        }
        self.prev = t.cars.iter().take(MAX_CARS).map(|c| (c.on_pit_road, c.pct, c.surface)).collect();
    }

    /// The player's pit state. `entry`/`exit` are the lines (lap fractions) to use.
    pub fn player(&mut self, t: &Telemetry, entry: Option<f32>, exit: Option<f32>, track_m: f32, limit_kph: f32) -> PitOut {
        let now = t.session_time;
        let on_pit = t.on_pit_road;
        let me = t.player();
        let in_box = t.in_pit_stall || me.is_some_and(|c| c.surface == surface::IN_PIT_STALL);
        if on_pit && !self.was_on_pit {
            // A new visit.
            self.left_box = false;
            self.box_since = None;
            self.service_seen = false;
        }
        // Driven across the line: a small step forward since the last tick, at speed. Being
        // put on the grid from the pit box (or reset, towed) moves the car without driving and
        // is no pit exit, so no "turn off the pit limiter" then.
        let pct = t.lap_dist_pct;
        let drove = t.speed > 2.0
            && self.last.is_some_and(|(was, at)| {
                let step = (pct - was + 1.5).rem_euclid(1.0) - 0.5;
                (0.0..0.5).contains(&(now - at)) && was >= 0.0 && pct >= 0.0 && (0.0..0.005).contains(&step)
            });
        self.last = Some((pct, now));
        if !on_pit && self.was_on_pit {
            self.exit_at = (t.driving() && drove).then_some(now);
        }
        self.was_on_pit = on_pit;
        if in_box {
            self.box_since.get_or_insert(now);
            self.service_seen |= t.pitstop_active || t.pit_sv_status == 1;
        } else if self.box_since.is_some() {
            self.left_box = true;
            self.box_since = None;
        }
        let ahead = |line: Option<f32>| {
            let d = (line? - t.lap_dist_pct).rem_euclid(1.0);
            (track_m > 0.0 && d < 0.3).then_some(((d * track_m) * 10.0).round() / 10.0)
        };
        let mut out = PitOut {
            to_entry: ahead(entry),
            to_exit: ahead(exit),
            speed: t.speed,
            limit: limit_kph / 3.6,
            limiter: t.engine_warnings & LIMITER != 0,
            repair: t.pit_repair_left.max(0.0),
            opt_repair: t.pit_opt_repair_left.max(0.0),
            since_exit: self.exit_at.map(|at| (now - at) as f32).filter(|s| (0.0..60.0).contains(s)),
            ..Default::default()
        };
        if !t.driving() {
            return out;
        }
        let surf = me.map(|c| c.surface).unwrap_or(surface::NOT_IN_WORLD);
        if surf == surface::APPROACHING_PITS {
            self.approach_at = Some(now);
        }
        // Held briefly so a tick between the entry road and pit road does not blank it.
        let approaching = self.approach_at.is_some_and(|at| (0.0..1.5).contains(&(now - at)));
        out.phase = if in_box {
            let waited = self.box_since.is_some_and(|s| now - s > 1.0);
            let busy = t.pitstop_active || t.pit_sv_status == 1 || t.pit_repair_left > 0.0;
            out.service = if busy || !(self.service_seen || waited) { "wait" } else { "go" };
            "box"
        } else if on_pit {
            if self.left_box || out.to_exit.is_some_and(|d| d < EXIT_SHOW_M) {
                "exit"
            } else {
                "lane"
            }
        } else if approaching && out.since_exit.is_none_or(|s| s > 10.0) {
            // Back in for another visit (a drive-through on the next lap, say).
            "entry"
        } else if out.since_exit.is_some() {
            "out"
        } else {
            ""
        };
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CarTel;

    fn tel(t: f64, pct: f32, pit: bool, surf: i32) -> Telemetry {
        let mut tel = Telemetry { session_time: t, player_idx: 0, lap_dist_pct: pct, on_pit_road: pit, is_on_track: true, speed: 30.0, ..Default::default() };
        tel.cars[0] = CarTel { pct, on_pit_road: pit, surface: surf, ..Default::default() };
        tel
    }

    #[test]
    fn learns_lines_and_follows_a_stop() {
        let dir = std::env::temp_dir().join(format!("apex-pit-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("pit-lines.json");
        let mut p = PitTracker::new(Some(file.clone()));
        let (entry, exit) = (Some(0.95), Some(0.05));
        let mut t = 0.0;
        let mut run = |p: &mut PitTracker, pct: f32, pit: bool, surf: i32, f: &dyn Fn(&mut Telemetry)| {
            t += 1.0 / 60.0;
            let mut x = tel(t, pct, pit, surf);
            f(&mut x);
            p.update(&x, 7);
            p.player(&x, p.learned().0.or(entry), p.learned().1.or(exit), 4000.0, 60.0)
        };
        let none = |_: &mut Telemetry| {};
        assert_eq!(run(&mut p, 0.90, false, surface::ON_TRACK, &none).phase, "");
        let o = run(&mut p, 0.93, false, surface::APPROACHING_PITS, &none);
        assert_eq!(o.phase, "entry");
        assert!((o.to_entry.unwrap() - 80.0).abs() < 0.5, "{o:?}");
        run(&mut p, 0.9601, false, surface::APPROACHING_PITS, &none);
        let o = run(&mut p, 0.9603, true, surface::ON_TRACK, &|x| x.engine_warnings = LIMITER);
        assert_eq!((o.phase, o.limiter), ("lane", true));
        assert!((p.learned().0.unwrap() - 0.9602).abs() < 1e-4);
        // In the box: wait while serviced, then go.
        let o = run(&mut p, 0.99, true, surface::IN_PIT_STALL, &|x| x.pitstop_active = true);
        assert_eq!((o.phase, o.service), ("box", "wait"));
        let o = run(&mut p, 0.99, true, surface::IN_PIT_STALL, &none);
        assert_eq!(o.service, "go");
        // Leaving the box: the exit sign, then out.
        assert_eq!(run(&mut p, 0.995, true, surface::ON_TRACK, &none).phase, "exit");
        run(&mut p, 0.0400, true, surface::ON_TRACK, &none);
        let o = run(&mut p, 0.0402, false, surface::ON_TRACK, &none);
        assert_eq!(o.phase, "out");
        assert!((p.learned().1.unwrap() - 0.0401).abs() < 1e-4);
        // Back in the box, then put on the grid (moved, not driven out): no pit exit banner.
        run(&mut p, 0.99, true, surface::IN_PIT_STALL, &none);
        let o = run(&mut p, 0.80, false, surface::ON_TRACK, &|x| {
            x.speed = 0.0;
            x.engine_warnings = LIMITER;
        });
        assert_ne!(o.phase, "out", "gridding is no pit exit");
        assert_eq!(o.since_exit, None);
        // Saved for next time.
        let again = PitTracker::new(Some(file));
        assert_eq!(again.all.get(&7).map(|l| l.entry.len()), Some(1));
        let _ = std::fs::remove_dir_all(dir);
    }
}
