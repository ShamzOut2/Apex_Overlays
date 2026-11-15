//! Safety alert: cars ahead that are causing a yellow (off track, stopped or crawling) or
//! carrying the meatball, and cars of a faster class closing from behind. Everything within
//! `RANGE` seconds is sent; the overlay applies its own (shorter) windows.

use crate::engine::Engine;
use crate::model::{flags, surface};
use crate::topics::{driver, extras, DriverOut, Extras};
use serde::Serialize;

/// Furthest ahead / behind (seconds) a car is reported.
pub const RANGE: f32 = 30.0;
/// Below this (m/s) a car causing a yellow is reported as slow rather than rejoining.
const SLOW: f32 = 15.0;
/// At most this many cars per list.
const MAX: usize = 4;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hazard {
    #[serde(flatten)]
    pub d: DriverOut,
    /// Lap times, flags, penalty, incidents… as in the standings.
    #[serde(flatten)]
    pub x: Extras,
    /// "yellow" (causing one) or "repair" (meatball).
    pub flag: &'static str,
    /// What the car is doing: "stopped", "slow", "offTrack", "rejoining" or "damaged".
    pub reason: &'static str,
    /// Seconds ahead of you on track, and metres.
    pub gap: f32,
    pub distance: f32,
    /// The car's speed, m/s.
    pub speed: f32,
    pub position: i32,
    pub class_position: i32,
    pub tire_wet: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Approach {
    #[serde(flatten)]
    pub d: DriverOut,
    #[serde(flatten)]
    pub x: Extras,
    /// Seconds behind you on track, and metres.
    pub gap: f32,
    pub distance: f32,
    /// How much faster per lap their class (or their recent laps) is, seconds.
    pub faster_by: f32,
    /// Seconds until they reach you at that pace.
    pub catch_in: Option<f32>,
    pub speed: f32,
    pub position: i32,
    pub class_position: i32,
    pub tire_wet: Option<bool>,
}

/// A car ahead serving a slow-down penalty: a chance to pass it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overtake {
    #[serde(flatten)]
    pub d: DriverOut,
    #[serde(flatten)]
    pub x: Extras,
    /// Seconds and metres ahead of you.
    pub gap: f32,
    pub distance: f32,
    /// Their speed and yours, m/s.
    pub speed: f32,
    pub my_speed: f32,
    pub position: i32,
    pub class_position: i32,
    pub tire_wet: Option<bool>,
}

#[derive(Serialize, Default)]
pub struct SafetyOut {
    pub hazards: Vec<Hazard>,
    pub approaching: Vec<Approach>,
    pub overtakes: Vec<Overtake>,
}

fn round(v: f32, digits: i32) -> f32 {
    let m = 10f32.powi(digits);
    (v * m).round() / m
}

/// A car's lap pace: its recent laps, else its class's estimated lap time.
fn pace(e: &Engine, idx: usize) -> Option<f32> {
    e.hist.get(idx).and_then(|h| h.pace()).or_else(|| class_pace(e, idx))
}

/// The estimated lap time of a car's class (iRacing's estimate, else the class's best lap).
fn class_pace(e: &Engine, idx: usize) -> Option<f32> {
    let est = e.info.driver(idx).map(|d| d.class_est_lap).unwrap_or(0.0);
    if est > 0.0 {
        return Some(est);
    }
    let class = &e.static_of(idx)?.class.id;
    e.classes.iter().find(|c| &c.key.id == class).and_then(|c| c.best_lap).map(|b| b.1)
}

pub fn build(e: &Engine) -> SafetyOut {
    let t = &e.tel;
    let Some(focus) = t.focus_idx() else { return SafetyOut::default() };
    let me = &t.cars[focus];
    if me.on_pit_road || me.surface == surface::IN_PIT_STALL {
        return SafetyOut::default();
    }
    let Some(my) = e.static_of(focus) else { return SafetyOut::default() };
    let track_m = e.info.track_length_km * 1000.0;
    let my_class_pace = class_pace(e, focus);
    let my_pace = pace(e, focus);
    let now = t.session_time;
    let mut out = SafetyOut::default();

    for r in &e.order {
        let i = r.idx;
        let c = &t.cars[i];
        if i == focus || !c.in_world() || c.on_pit_road || c.surface == surface::IN_PIT_STALL || c.surface == surface::APPROACHING_PITS {
            continue;
        }
        let Some(s) = e.static_of(i) else { continue };
        let gap = e.relative_gap(focus, i);
        let mut dp = c.pct - me.pct;
        if dp > 0.5 {
            dp -= 1.0;
        } else if dp < -0.5 {
            dp += 1.0;
        }
        let distance = round((dp * track_m).abs(), 0);
        let h = &e.hist[i];
        if gap > 0.0 && gap <= RANGE && h.slowdown_since.is_some() {
            out.overtakes.push(Overtake {
                d: driver(e, s),
                x: extras(e, i),
                gap: round(gap, 1),
                distance,
                speed: round(h.speed, 1),
                my_speed: round(e.hist[focus].speed, 1),
                position: r.pos,
                class_position: r.class_pos,
                tire_wet: e.info.tire_shown(c),
            });
        }
        if gap > 0.0 && gap <= RANGE {
            let yellow = now < h.yellow_until;
            let repair = c.session_flags & flags::REPAIR != 0;
            if yellow || repair {
                let reason = if !yellow {
                    "damaged"
                } else if c.surface == surface::OFF_TRACK {
                    "offTrack"
                } else if h.speed < 2.0 {
                    "stopped"
                } else if h.speed < SLOW {
                    "slow"
                } else {
                    // Back up to speed while the yellow it caused is still out.
                    "rejoining"
                };
                out.hazards.push(Hazard {
                    d: driver(e, s),
                    x: extras(e, i),
                    flag: if yellow { "yellow" } else { "repair" },
                    reason,
                    gap: round(gap, 1),
                    distance,
                    speed: round(h.speed, 1),
                    position: r.pos,
                    class_position: r.class_pos,
                    tire_wet: e.info.tire_shown(c),
                });
            }
        } else if gap < 0.0 && -gap <= RANGE && s.class.id != my.class.id {
            // A different class that is faster than yours.
            let (Some(theirs), Some(mine)) = (class_pace(e, i), my_class_pace) else { continue };
            if theirs >= mine * 0.995 {
                continue;
            }
            let faster_by = match (pace(e, i), my_pace) {
                (Some(a), Some(b)) if b > a => b - a,
                _ => mine - theirs,
            };
            let catch_in = (faster_by > 0.05).then(|| -gap * theirs / faster_by);
            out.approaching.push(Approach {
                d: driver(e, s),
                x: extras(e, i),
                gap: round(-gap, 1),
                distance,
                faster_by: round(faster_by, 1),
                catch_in: catch_in.map(|v| round(v, 0)),
                speed: round(h.speed, 1),
                position: r.pos,
                class_position: r.class_pos,
                tire_wet: e.info.tire_shown(c),
            });
        }
    }
    out.hazards.sort_by(|a, b| a.gap.total_cmp(&b.gap));
    out.approaching.sort_by(|a, b| a.gap.total_cmp(&b.gap));
    out.overtakes.sort_by(|a, b| a.gap.total_cmp(&b.gap));
    out.overtakes.truncate(MAX);
    out.hazards.truncate(MAX);
    out.approaching.truncate(MAX);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::{engine, place};

    #[test]
    fn hazards_ahead_and_faster_classes_behind() {
        let mut e = engine(4, true);
        // Class 2 (odd car numbers) is ten seconds a lap faster than yours.
        let mut info = e.info.clone();
        for d in info.drivers.iter_mut().filter(|d| d.car_idx % 2 == 1) {
            d.class_est_lap = 90.0;
        }
        e.set_session(info);
        let step = |e: &mut Engine, t: f64| {
            e.tel.session_time = t;
            place(e, 0, 3, 0.50);
            place(e, 1, 3, 0.47);
            place(e, 2, 3, 0.55);
            place(e, 3, 3, 0.52);
            e.tel.cars[1].est_time = 0.47 * 90.0;
            e.tel.cars[3].est_time = 0.52 * 90.0;
            e.tel.cars[2].surface = surface::OFF_TRACK;
            e.tel.cars[3].session_flags = flags::REPAIR;
            e.tick();
        };
        step(&mut e, 1.0);
        step(&mut e, 1.5);
        let out = build(&e);
        let hz: Vec<_> = out.hazards.iter().map(|h| (h.d.idx, h.flag, h.reason)).collect();
        assert_eq!(hz, vec![(3, "repair", "damaged"), (2, "yellow", "offTrack")]);
        assert!(out.hazards[0].gap > 0.0 && out.hazards[0].gap < out.hazards[1].gap);
        assert_eq!(out.approaching.len(), 1);
        let a = &out.approaching[0];
        assert_eq!(a.d.idx, 1);
        assert!((a.gap - 3.0).abs() < 0.2, "{}", a.gap);
        assert!((a.faster_by - 10.0).abs() < 0.01);
        assert!(a.catch_in.is_some_and(|c| c > 20.0 && c < 35.0), "{:?}", a.catch_in);

        // Once passed, the hazard is no longer ahead and drops out.
        e.tel.session_time = 2.0;
        place(&mut e, 0, 3, 0.60);
        e.tick();
        assert!(build(&e).hazards.iter().all(|h| h.d.idx != 2 && h.d.idx != 3));
    }

    #[test]
    fn slow_down_ahead_is_an_overtake_chance() {
        let mut e = engine(2, true);
        // Car 1 is half a lap's worth of ticks at half speed: it gives time up.
        for k in 0..40 {
            e.tel.session_time = k as f64 * 0.1;
            place(&mut e, 0, 3, 0.40 + k as f32 * 0.001);
            place(&mut e, 1, 3, 0.45 + k as f32 * 0.0005);
            e.tel.cars[1].session_flags = flags::FURLED;
            e.tick();
        }
        assert!(e.car_flags(1).contains(&"slow"));
        let out = build(&e);
        assert_eq!(out.overtakes.len(), 1);
        let sd = out.overtakes[0].x.slowdown.unwrap();
        assert!(sd.seconds > 3.5 && sd.lost > 1.0, "{} {}", sd.seconds, sd.lost);
        // Flag lowered: no longer an overtake chance.
        e.tel.cars[1].session_flags = 0;
        e.tick();
        assert!(build(&e).overtakes.is_empty());
    }
}
