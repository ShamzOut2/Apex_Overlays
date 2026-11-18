//! Battle box: the cars directly ahead and behind, with gap trend.

use super::Engine;
use std::collections::VecDeque;

const SAMPLE_EVERY: f64 = 0.5;
const WINDOW: f64 = 45.0;

#[derive(Debug, Default, Clone)]
pub struct GapHist {
    pub idx: Option<usize>,
    samples: VecDeque<(f64, f32)>,
}

impl GapHist {
    fn push(&mut self, idx: usize, at: f64, gap: f32) {
        if self.idx != Some(idx) || self.samples.back().is_some_and(|s| at < s.0) {
            self.idx = Some(idx);
            self.samples.clear();
        }
        if self.samples.back().is_some_and(|s| at - s.0 < SAMPLE_EVERY) {
            return;
        }
        self.samples.push_back((at, gap));
        while self.samples.front().is_some_and(|s| at - s.0 > WINDOW) {
            self.samples.pop_front();
        }
    }

    /// Least-squares slope of gap over time (seconds of gap per second).
    pub fn slope(&self) -> Option<f32> {
        if self.samples.len() < 10 {
            return None;
        }
        let n = self.samples.len() as f64;
        let (mx, my) = self.samples.iter().fold((0.0, 0.0), |(x, y), s| (x + s.0 / n, y + s.1 as f64 / n));
        let (mut num, mut den) = (0.0, 0.0);
        for s in &self.samples {
            num += (s.0 - mx) * (s.1 as f64 - my);
            den += (s.0 - mx).powi(2);
        }
        (den > 0.0).then(|| (num / den) as f32)
    }
}

/// Who counts as "the car ahead/behind".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Position battle within your class.
    Class,
    /// Overall running order, any class.
    Overall,
    /// Physically nearest cars on track, any class (includes lapped/lapping cars).
    Track,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Class, Mode::Overall, Mode::Track];
    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Default)]
pub struct BattleTracker {
    /// Gap history per mode: [ahead, behind].
    pub hist: [[GapHist; 2]; 3],
}

impl BattleTracker {
    pub fn get(&self, mode: Mode, ahead: bool) -> &GapHist {
        &self.hist[mode.index()][if ahead { 0 } else { 1 }]
    }
}

/// Returns (ahead, behind) car indices for the focus car.
pub fn neighbours(e: &Engine, focus: usize, mode: Mode) -> (Option<usize>, Option<usize>) {
    let class = e.static_of(focus).map(|s| s.class.id.clone());
    let eligible = |i: usize| i != focus && (mode != Mode::Class || e.static_of(i).map(|s| &s.class.id) == class.as_ref());
    if e.is_race() && mode != Mode::Track {
        let rows: Vec<usize> = e.order.iter().map(|r| r.idx).filter(|&i| i == focus || eligible(i)).collect();
        let Some(p) = rows.iter().position(|&i| i == focus) else { return (None, None) };
        (p.checked_sub(1).map(|i| rows[i]), rows.get(p + 1).copied())
    } else if e.alone_on_track() {
        // LMU private qualifying: the others are in sessions of their own, not around you.
        (None, None)
    } else {
        // On-track neighbours (always used outside races).
        let mut best: [Option<(usize, f32)>; 2] = [None, None];
        for r in &e.order {
            if !eligible(r.idx) || !e.tel.cars[r.idx].in_world() || e.tel.cars[r.idx].on_pit_road {
                continue;
            }
            let g = e.relative_gap(focus, r.idx);
            if g >= 0.0 && best[0].is_none_or(|b| g < b.1) {
                best[0] = Some((r.idx, g));
            } else if g < 0.0 && best[1].is_none_or(|b| -g < b.1) {
                best[1] = Some((r.idx, -g));
            }
        }
        (best[0].map(|b| b.0), best[1].map(|b| b.0))
    }
}

/// Gap in seconds from focus to `other` (always positive) and whole laps between them.
pub fn gap(e: &Engine, focus: usize, other: usize, other_ahead: bool, mode: Mode) -> (Option<f32>, i32) {
    if !e.is_race() || mode == Mode::Track {
        return (Some(e.relative_gap(focus, other).abs()), 0);
    }
    if other_ahead {
        e.time_between(focus, other)
    } else {
        e.time_between(other, focus)
    }
}

pub fn update(e: &mut Engine) {
    let Some(focus) = e.tel.focus_idx() else { return };
    let now = e.tel.session_time;
    for mode in Mode::ALL {
        let (ahead, behind) = neighbours(e, focus, mode);
        let ga = ahead.and_then(|a| gap(e, focus, a, true, mode).0.map(|g| (a, g)));
        let gb = behind.and_then(|b| gap(e, focus, b, false, mode).0.map(|g| (b, g)));
        if let Some((a, g)) = ga {
            e.battle.hist[mode.index()][0].push(a, now, g);
        }
        if let Some((b, g)) = gb {
            e.battle.hist[mode.index()][1].push(b, now, g);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{engine, place};
    use super::*;

    #[test]
    fn finds_class_neighbours_and_trend() {
        let mut e = engine(6, true);
        // Class 1 = even indices.
        place(&mut e, 2, 3, 0.60);
        place(&mut e, 0, 3, 0.55);
        place(&mut e, 4, 3, 0.50);
        place(&mut e, 1, 3, 0.58);
        e.tick();
        assert_eq!(neighbours(&e, 0, Mode::Class), (Some(2), Some(4)));
        // Overall order includes the other class; on track the nearest cars of any class.
        assert_eq!(neighbours(&e, 0, Mode::Overall), (Some(1), Some(4)));
        assert_eq!(neighbours(&e, 0, Mode::Track), (Some(1), Some(4)));
        // Car ahead pulls away 0.1 s every second.
        for s in 0..40 {
            e.tel.session_time = s as f64;
            e.tel.cars[2].est_time = 60.0 + s as f32 * 0.1;
            e.tick();
        }
        let slope = e.battle.get(Mode::Class, true).slope().unwrap();
        assert!((slope - 0.1).abs() < 1e-3, "{slope}");
    }
}
