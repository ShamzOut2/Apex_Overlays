//! The player's incidents: each rise of the incident count, what it most likely was, and
//! the cars nearby when it was contact. iRacing only gives the count, so the type is read
//! from the points (road: 1x off track, 2x loss of control or hitting a wall, 4x contact
//! with a car) together with what the car was doing at the time.

use crate::model::{surface, SessionInfo, Telemetry};
use serde::Serialize;
use std::collections::VecDeque;

/// Incidents kept per subsession, and in an endurance race (over 5 hours: a time limit that
/// long, or enough laps at your lap time).
const KEEP: usize = 100;
pub const KEEP_LONG: usize = 300;
const LONG_RACE: f64 = 5.0 * 3600.0;
/// The newest incidents sent to the overlay (it shows up to 30).
pub const SENT: usize = 30;
/// A car this close (metres along the track) to a 4x is taken as the other car involved.
const CONTACT_RANGE: f32 = 12.0;
/// Acceleration (m/s²) above which a 2x is taken as hitting a wall or object, not a spin.
const IMPACT: f32 = 35.0;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Involved {
    pub idx: usize,
    pub name: String,
    pub number: String,
    /// Metres ahead of you along the track (negative = behind) when it happened.
    pub dist: f32,
    /// Their own incident count went up by 4 or more just after.
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Incident {
    pub points: i32,
    /// Main cause: "offTrack", "lossOfControl", "objectContact", "carContact" or "heavyContact".
    pub kind: &'static str,
    pub label: String,
    /// Your incident count after it.
    pub total: i32,
    pub lap: i32,
    pub session_num: i32,
    pub session_name: String,
    pub session_time: f64,
    pub involved: Vec<Involved>,
    /// Incident counts of the cars involved at the time (for confirming them later).
    #[serde(skip)]
    base: Vec<i32>,
}

#[derive(Debug, Default)]
pub struct IncidentTracker {
    subsession: i64,
    last: Option<i32>,
    pub list: Vec<Incident>,
    /// (session time, |acceleration|) over the last couple of seconds.
    accel: VecDeque<(f64, f32)>,
    off_at: f64,
    /// An endurance race: more incidents are kept (set once, for the rest of the event).
    pub long_race: bool,
}

/// The event has a race over 5 hours: by its time limit, or its laps at your lap time.
fn long_race(t: &Telemetry, info: &SessionInfo) -> bool {
    let lap = [t.lap_best_time, t.lap_last_time].into_iter().find(|&l| l > 0.0).unwrap_or(0.0) as f64;
    info.sessions
        .iter()
        .filter(|s| s.is_race())
        .any(|s| s.time.is_some_and(|x| x > LONG_RACE) || s.laps.is_some_and(|l| lap > 0.0 && l as f64 * lap > LONG_RACE))
}

/// Cars within `range` metres of the player along the track, in the same pit lane state,
/// nearest first: (car index, metres ahead, negative = behind).
pub fn near_player(t: &Telemetry, track_m: f32, range: f32) -> Vec<(usize, f32)> {
    let Some(me) = usize::try_from(t.player_idx).ok().filter(|&i| i < t.cars.len()) else { return vec![] };
    if track_m <= 0.0 {
        return vec![];
    }
    let p0 = t.lap_dist_pct;
    let mut near: Vec<(usize, f32)> = t
        .cars
        .iter()
        .enumerate()
        .filter(|&(i, c)| i != me && c.pct >= 0.0 && c.in_world() && c.on_pit_road == t.on_pit_road)
        .map(|(i, c)| (i, ((c.pct - p0 + 1.5).rem_euclid(1.0) - 0.5) * track_m))
        .filter(|(_, d)| d.abs() <= range)
        .collect();
    near.sort_by(|a, b| a.1.abs().total_cmp(&b.1.abs()));
    near
}

/// The parts an incident of `points` is made of, biggest first.
fn parts(points: i32) -> Vec<i32> {
    let mut left = points;
    let mut out = vec![];
    for p in [4, 2, 1] {
        while left >= p {
            out.push(p);
            left -= p;
        }
    }
    out
}

impl IncidentTracker {
    pub fn update(&mut self, t: &Telemetry, info: &SessionInfo) {
        if info.subsession_id != self.subsession {
            *self = Self { subsession: info.subsession_id, ..Default::default() };
        }
        // A replay shows past telemetry; the count only means something live.
        if t.is_replay_playing || t.player_idx < 0 {
            return;
        }
        if !self.long_race && long_race(t, info) {
            self.long_race = true;
        }
        let now = t.session_time;
        self.accel.push_back((now, t.lat_accel.hypot(t.long_accel)));
        while self.accel.front().is_some_and(|&(at, _)| now - at > 2.0 || at > now) {
            self.accel.pop_front();
        }
        if t.player().is_some_and(|c| c.surface == surface::OFF_TRACK) {
            self.off_at = now;
        }
        match self.last {
            Some(p) if t.incidents > p => self.record(t, info, t.incidents - p),
            _ => {}
        }
        self.last = Some(t.incidents);
        self.confirm(info, now);
    }

    fn record(&mut self, t: &Telemetry, info: &SessionInfo, points: i32) {
        let peak = self.accel.iter().map(|a| a.1).fold(0.0f32, f32::max);
        let near = near_player(t, info.track_length_km * 1000.0, CONTACT_RANGE);
        let mut kind = "";
        let mut labels: Vec<String> = vec![];
        let mut involved = vec![];
        for p in parts(points) {
            let (k, l) = match p {
                4 if !near.is_empty() => {
                    involved = near
                        .iter()
                        .take(2)
                        .filter_map(|&(idx, dist)| {
                            let d = info.driver(idx)?;
                            Some(Involved { idx, name: d.name.clone(), number: d.car_number.clone(), dist: (dist * 10.0).round() / 10.0, confirmed: false })
                        })
                        .collect();
                    ("carContact", "Contact with another car".to_string())
                }
                4 => ("heavyContact", "Heavy contact".to_string()),
                2 if peak > IMPACT => ("objectContact", "Hit a wall or object".to_string()),
                2 => ("lossOfControl", "Loss of control".to_string()),
                _ => ("offTrack", "Off track".to_string()),
            };
            if kind.is_empty() {
                kind = k;
            }
            if !labels.contains(&l) {
                labels.push(l);
            }
        }
        let base = involved.iter().map(|i: &Involved| info.driver(i.idx).map(|d| d.incidents).unwrap_or(0)).collect();
        let session_name = info.sessions.iter().find(|s| s.num == t.session_num).map(|s| s.name.clone()).unwrap_or_default();
        self.list.push(Incident {
            points,
            kind,
            label: labels.join(" + "),
            total: t.incidents,
            lap: t.lap,
            session_num: t.session_num,
            session_name,
            session_time: t.session_time,
            involved,
            base,
        });
        let keep = if self.long_race { KEEP_LONG } else { KEEP };
        while self.list.len() > keep {
            self.list.remove(0);
        }
    }

    /// The other car in a contact usually gets the same 4x: once its own count shows that,
    /// it is marked as confirmed.
    fn confirm(&mut self, info: &SessionInfo, now: f64) {
        for inc in self.list.iter_mut().rev().take_while(|i| now - i.session_time < 120.0) {
            for (inv, base) in inc.involved.iter_mut().zip(&inc.base) {
                if !inv.confirmed && info.driver(inv.idx).is_some_and(|d| d.incidents >= base + 4) {
                    inv.confirmed = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::{engine, place};

    #[test]
    fn splits_points() {
        assert_eq!(parts(1), vec![1]);
        assert_eq!(parts(4), vec![4]);
        assert_eq!(parts(5), vec![4, 1]);
        assert_eq!(parts(7), vec![4, 2, 1]);
    }

    #[test]
    fn keeps_more_in_an_endurance_race() {
        let mut e = engine(1, true);
        let inc = |tr: &mut IncidentTracker, e: &mut crate::engine::Engine, n: usize| {
            for _ in 0..n {
                e.tel.session_time += 10.0;
                e.tel.incidents += 1;
                tr.update(&e.tel, &e.info);
            }
        };
        // A 50-minute race: 100 kept.
        e.info.sessions = vec![crate::model::SessionDef { kind: "Race".into(), time: Some(3000.0), ..Default::default() }];
        let mut tr = IncidentTracker::default();
        inc(&mut tr, &mut e, 150);
        assert_eq!((tr.list.len(), tr.long_race), (100, false));
        // A 6-hour race: 300.
        e.info.sessions[0].time = Some(6.0 * 3600.0);
        let mut tr = IncidentTracker::default();
        inc(&mut tr, &mut e, 350);
        assert_eq!((tr.list.len(), tr.long_race), (KEEP_LONG, true));
        // 200 laps at 1:45 (5h50) with no time limit: long once a lap is timed.
        e.info.sessions[0] = crate::model::SessionDef { kind: "Race".into(), laps: Some(200), ..Default::default() };
        let mut tr = IncidentTracker::default();
        e.tel.lap_best_time = -1.0;
        e.tel.lap_last_time = -1.0;
        inc(&mut tr, &mut e, 1);
        assert!(!tr.long_race, "lap time not known yet");
        e.tel.lap_last_time = 105.0;
        inc(&mut tr, &mut e, 1);
        assert!(tr.long_race);
    }

    #[test]
    fn records_and_classifies() {
        let mut e = engine(3, true);
        e.info.track_length_km = 5.0;
        for i in 0..3 {
            place(&mut e, i, 2, 0.3 + 0.1 * i as f32);
        }
        e.tel.lap_dist_pct = 0.3;
        let mut tr = IncidentTracker::default();
        let step = |tr: &mut IncidentTracker, e: &mut crate::engine::Engine, inc: i32| {
            e.tel.session_time += 1.0 / 60.0;
            e.tel.incidents = inc;
            tr.update(&e.tel, &e.info);
        };
        step(&mut tr, &mut e, 0);
        // Off track.
        e.tel.cars[0].surface = surface::OFF_TRACK;
        step(&mut tr, &mut e, 1);
        assert_eq!((tr.list[0].kind, tr.list[0].points, tr.list[0].total), ("offTrack", 1, 1));
        e.tel.cars[0].surface = surface::ON_TRACK;
        // A spin, then a big hit.
        step(&mut tr, &mut e, 3);
        assert_eq!(tr.list[1].kind, "lossOfControl");
        e.tel.lat_accel = 60.0;
        step(&mut tr, &mut e, 5);
        assert_eq!(tr.list[2].kind, "objectContact");
        e.tel.lat_accel = 0.0;
        // Contact with car 1, 2 m ahead; it then gets its own 4x.
        place(&mut e, 1, 2, 0.3004);
        step(&mut tr, &mut e, 9);
        let c = &tr.list[3];
        assert_eq!((c.kind, c.label.as_str()), ("carContact", "Contact with another car"));
        assert_eq!((c.involved[0].idx, c.involved[0].dist, c.involved[0].confirmed), (1, 2.0, false));
        e.info.drivers.iter_mut().find(|d| d.car_idx == 1).unwrap().incidents += 4;
        step(&mut tr, &mut e, 9);
        assert!(tr.list[3].involved[0].confirmed);
        // Replays don't count.
        e.tel.is_replay_playing = true;
        step(&mut tr, &mut e, 13);
        assert_eq!(tr.list.len(), 4);
    }
}
