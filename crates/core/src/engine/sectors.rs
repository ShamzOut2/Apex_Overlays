//! Live sector times of the focus car, compared with its best time in each sector
//! (track map sector colours).

use crate::model::Telemetry;
use serde::Serialize;

/// The latest completed time in one sector.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SectorTime {
    /// Lap (as counted by the tracker) the sector was completed on.
    pub lap: u32,
    pub time: f64,
    /// Against the best time before this one (negative = new best). None for the first valid time.
    pub delta: Option<f64>,
}

/// A finished lap of the focus car: its sector times and how it went.
#[derive(Debug, Clone, PartialEq)]
pub struct SectorLap {
    /// The car's lap number (the lap that was driven).
    pub lap: i32,
    /// Sector times (None for a sector not timed cleanly).
    pub sectors: Vec<Option<f64>>,
    /// Session time the line was crossed.
    pub end_at: f64,
    /// Driven through the pit lane at some point.
    pub pit: bool,
    /// The game took the lap away (track limits; Le Mans Ultimate).
    pub invalid: bool,
}

#[derive(Debug, Default)]
pub struct SectorTracker {
    car: Option<usize>,
    /// Sector start fractions, ascending, beginning with 0.
    pub starts: Vec<f32>,
    cur: Option<usize>,
    entered_at: f64,
    valid: bool,
    prev_pct: f32,
    prev_t: f64,
    /// Best time in each sector this session (focus car).
    pub best: Vec<Option<f64>>,
    pub last: Vec<Option<SectorTime>>,
    pub lap: u32,
    pub lap_end_at: Option<f64>,
    /// The lap in progress: its sectors so far, pit lane, invalidated, and the car's lap number.
    pub current: Vec<Option<f64>>,
    cur_pit: bool,
    cur_invalid: bool,
    prev_lap: i32,
    /// Finished laps, newest last (the last 30).
    pub laps: std::collections::VecDeque<SectorLap>,
}

impl SectorTracker {
    fn reset(&mut self, starts: Vec<f32>, car: Option<usize>) {
        *self = Self { best: vec![None; starts.len()], last: vec![None; starts.len()], current: vec![None; starts.len()], starts, car, ..Default::default() };
    }

    /// Sector starts from the session info; three equal sectors when the track has none.
    fn starts_from(sectors: &[f32]) -> Vec<f32> {
        let mut s: Vec<f32> = sectors.iter().copied().filter(|p| (0.0..1.0).contains(p)).collect();
        s.sort_by(f32::total_cmp);
        s.dedup();
        if s.len() < 2 {
            return vec![0.0, 1.0 / 3.0, 2.0 / 3.0];
        }
        if s[0] > 0.0 {
            s.insert(0, 0.0);
        }
        s
    }

    fn sector_of(&self, pct: f32) -> usize {
        self.starts.iter().rposition(|&s| s <= pct).unwrap_or(0)
    }

    pub fn update(&mut self, tel: &Telemetry, sectors: &[f32]) {
        let car = tel.focus_idx();
        let starts = Self::starts_from(sectors);
        if car != self.car || starts != self.starts {
            self.reset(starts, car);
        }
        let Some(idx) = car else { return };
        let c = &tel.cars[idx];
        let (pct, now) = (c.pct, tel.session_time);
        let dt = now - self.prev_t;
        // A pause, replay jump or tow breaks the timing of the sector in progress.
        if !c.in_world() || !(0.0..1.0).contains(&dt) {
            self.cur = None;
        }
        let s = self.sector_of(pct);
        match self.cur {
            None => {
                self.cur = Some(s);
                self.valid = false;
            }
            Some(k) if k != s => {
                let n = self.starts.len();
                let next = (k + 1) % n;
                if s == next {
                    // Interpolate the moment the car crossed the boundary.
                    let boundary = if next == 0 { 1.0 } else { self.starts[next] };
                    let p1 = if pct < self.prev_pct { pct + 1.0 } else { pct };
                    let frac = if p1 > self.prev_pct { ((boundary - self.prev_pct) / (p1 - self.prev_pct)).clamp(0.0, 1.0) } else { 1.0 };
                    let at = self.prev_t + dt * frac as f64;
                    if self.valid {
                        let time = at - self.entered_at;
                        let delta = self.best[k].map(|b| time - b);
                        if delta.is_none_or(|d| d < 0.0) {
                            self.best[k] = Some(time);
                        }
                        self.last[k] = Some(SectorTime { lap: self.lap, time, delta });
                        self.current[k] = Some(time);
                    }
                    if next == 0 {
                        self.lap += 1;
                        self.lap_end_at = Some(at);
                        // The lap is done: kept with its sectors (a lap begun part-way is not).
                        if self.current.iter().any(|s| s.is_some()) {
                            let sectors = std::mem::replace(&mut self.current, vec![None; n]);
                            self.laps.push_back(SectorLap { lap: self.prev_lap, sectors, end_at: at, pit: self.cur_pit, invalid: self.cur_invalid });
                            if self.laps.len() > 30 {
                                self.laps.pop_front();
                            }
                        }
                        self.current = vec![None; n];
                        self.cur_pit = c.on_pit_road;
                        self.cur_invalid = false;
                    }
                    self.entered_at = at;
                    self.valid = !c.on_pit_road;
                } else {
                    self.valid = false;
                }
                self.cur = Some(s);
            }
            _ => {}
        }
        if c.on_pit_road {
            self.valid = false;
            self.cur_pit = true;
        }
        // Le Mans Ultimate says when the player's lap no longer counts.
        if tel.lap_invalid && tel.player_idx == idx as i32 {
            self.cur_invalid = true;
        }
        // The lap number during the lap (the game may count the new one as the line is crossed).
        if !(pct < self.prev_pct && self.prev_pct - pct > 0.5) {
            self.prev_lap = c.lap;
        }
        self.prev_pct = pct;
        self.prev_t = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{surface, CarTel};

    fn tel_at(t: f64, pct: f32) -> Telemetry {
        let mut tel = Telemetry { player_idx: 0, session_time: t, ..Default::default() };
        tel.cars[0] = CarTel { pct, surface: surface::ON_TRACK, ..Default::default() };
        tel
    }

    /// Drive from (t0, p0) to (t1, p1) at constant speed, 60 ticks a second (p may pass 1.0).
    fn drive(s: &mut SectorTracker, t0: f64, t1: f64, p0: f32, p1: f32) {
        let n = ((t1 - t0) * 60.0).round() as usize;
        for k in 1..=n {
            let f = k as f64 / n as f64;
            s.update(&tel_at(t0 + (t1 - t0) * f, (p0 + (p1 - p0) * f as f32) % 1.0), &[0.0, 0.5]);
        }
    }

    #[test]
    fn times_sectors_and_compares_with_best() {
        let mut s = SectorTracker::default();
        // Joining mid sector 1: that sector is not timed.
        drive(&mut s, 0.0, 5.0, 0.25, 0.5);
        assert!(s.last[0].is_none());
        drive(&mut s, 5.0, 15.0, 0.5, 1.0);
        let s2 = s.last[1].unwrap();
        assert!((s2.time - 10.0).abs() < 0.05 && s2.delta.is_none(), "{s2:?}");
        assert_eq!(s.lap, 1);
        drive(&mut s, 15.0, 24.0, 0.0, 0.5);
        drive(&mut s, 24.0, 33.5, 0.5, 1.0);
        let s2 = s.last[1].unwrap();
        assert!((s2.delta.unwrap() + 0.5).abs() < 0.05, "{s2:?}");
        assert_eq!((s2.lap, s.lap), (1, 2));
        assert!((s.lap_end_at.unwrap() - 33.5).abs() < 0.05);
        // A slower sector keeps the best.
        drive(&mut s, 33.5, 43.0, 0.0, 0.5);
        assert!((s.last[0].unwrap().delta.unwrap() - 0.5).abs() < 0.05);
    }
}
