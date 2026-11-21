//! Track outlines for Le Mans Ultimate, built from where the cars actually are.
//!
//! iRacing does not say where cars are on track, so its outline is reckoned from the player's
//! heading over one clean lap (engine::trackmap). LMU publishes every car's world position
//! with its distance into the lap, so here each scoring update adds every moving car's
//! position to the stretch of lap it is on, and the averages draw the track: exact, from all
//! cars at once (usually complete within a minute of a busy session, even while you sit in the
//! garage or spectate), with no drift to close. Cars in the pit lane draw the pit lane.

use super::data::VehicleScoring;
use super::Snapshot;
use crate::engine::trackmap::{fill_gaps, Frame, TrackMap, BINS, MAP_VERSION};

/// Share of the lap that must have positions before the outline is used (the few stretches
/// left are filled in between their neighbours).
const COMPLETE: f32 = 0.97;
/// Pit lane stretches further apart than this (in lap bins) are not joined up.
const PIT_GAP: usize = 4;
/// A car's two positions this many stretches apart or fewer (one scoring update) are joined
/// up: a fast car moves further than one stretch between updates, and alone on track it would
/// otherwise leave gaps that take many laps to fill.
const JOIN: usize = 12;

#[derive(Clone, Copy, Default)]
struct Sum {
    x: f64,
    z: f64,
    n: u32,
}

impl Sum {
    fn add(&mut self, x: f64, z: f64) {
        self.x += x;
        self.z += z;
        self.n += 1;
    }
    /// Average position as (east, north).
    fn mean(&self) -> Option<[f64; 2]> {
        (self.n > 0).then(|| [self.x / self.n as f64, self.z / self.n as f64])
    }
}

pub struct MapBuilder {
    track_id: i64,
    name: String,
    track: Vec<Sum>,
    pit: Vec<Sum>,
    last_et: f64,
    done: bool,
    ready: Option<TrackMap>,
    /// Each car's last position (stretch, east, north, in the pit lane), to join up to the next.
    prev: std::collections::HashMap<i32, (usize, f64, f64, bool)>,
}

impl Default for MapBuilder {
    fn default() -> Self {
        Self {
            track_id: 0,
            name: String::new(),
            track: vec![Sum::default(); BINS],
            pit: vec![Sum::default(); BINS],
            last_et: -1.0,
            done: false,
            ready: None,
            prev: Default::default(),
        }
    }
}

impl MapBuilder {
    /// Start again (a new track, or the player reset the map).
    pub fn reset(&mut self) {
        let (id, name) = (self.track_id, std::mem::take(&mut self.name));
        *self = Self { track_id: id, name, ..Self::default() };
    }

    /// Adds the cars' positions from a scoring update (each update only once).
    pub fn feed(&mut self, snap: &Snapshot, track_id: i64, name: &str) {
        if track_id != self.track_id {
            *self = Self { track_id, name: name.to_string(), ..Self::default() };
        }
        let len = snap.info.mLapDist;
        let et = snap.info.mCurrentET;
        if self.done || len < 100.0 || et == self.last_et {
            return;
        }
        self.last_et = et;
        for v in &snap.vehicles {
            let Some((bin, x, z, pit)) = sample(v, len) else {
                self.prev.remove(&v.mID);
                continue;
            };
            let bins = if pit { &mut self.pit } else { &mut self.track };
            bins[bin].add(x, z);
            // The stretches passed since this car's last position, filled in along the way.
            if let Some(&(b0, x0, z0, p0)) = self.prev.get(&v.mID) {
                let gap = (bin + BINS - b0) % BINS;
                if p0 == pit && (2..=JOIN).contains(&gap) {
                    for k in 1..gap {
                        let f = k as f64 / gap as f64;
                        bins[(b0 + k) % BINS].add(x0 + (x - x0) * f, z0 + (z - z0) * f);
                    }
                }
            }
            self.prev.insert(v.mID, (bin, x, z, pit));
        }
        if self.progress() >= COMPLETE {
            self.ready = Some(self.build());
            self.done = true;
        }
    }

    /// Share of the lap with positions so far.
    pub fn progress(&self) -> f32 {
        if self.done {
            return 1.0;
        }
        self.track.iter().filter(|s| s.n > 0).count() as f32 / BINS as f32
    }

    /// The finished outline, once.
    pub fn take(&mut self) -> Option<TrackMap> {
        self.ready.take()
    }

    fn build(&self) -> TrackMap {
        let pts = fill_gaps(&self.track.iter().map(Sum::mean).collect::<Vec<_>>());
        let frame = Frame::of(&pts);
        // The pit lane: neighbouring stretches with positions, joined up (around the lap's end too).
        let mut pit = vec![];
        let filled: Vec<usize> = (0..BINS).filter(|&i| self.pit[i].n > 0).collect();
        for (k, &i) in filled.iter().enumerate() {
            let j = filled[(k + 1) % filled.len()];
            let gap = (j + BINS - i) % BINS;
            if filled.len() > 1 && (1..=PIT_GAP).contains(&gap) {
                if let (Some(a), Some(b)) = (self.pit[i].mean(), self.pit[j].mean()) {
                    pit.push([frame.apply(a), frame.apply(b)]);
                }
            }
        }
        TrackMap {
            track_id: self.track_id,
            name: self.name.clone(),
            points: pts.iter().map(|p| frame.apply(*p)).collect(),
            source: "lmu".into(),
            version: MAP_VERSION,
            pit,
            turns: vec![],
        }
    }
}

/// A car's lap stretch and position (east = x, north = z), and whether it is in the pit lane;
/// `None` for cars parked, in the garage, off the track or crawling (they would bend the line).
fn sample(v: &VehicleScoring, len: f64) -> Option<(usize, f64, f64, bool)> {
    if v.mInGarageStall != 0 || v.mControl < 0 || v.mPitState == 3 {
        return None;
    }
    let vel = v.mLocalVel;
    let speed = (vel.x * vel.x + vel.y * vel.y + vel.z * vel.z).sqrt();
    let dist = v.mLapDist;
    if speed < 5.0 || !(0.0..len).contains(&dist) {
        return None;
    }
    let pit = v.mInPits != 0;
    // Off the track (beyond its edge, with a little room for kerbs): not part of the outline.
    let (lateral, edge) = (v.mPathLateral, v.mTrackEdge);
    if !pit && edge > 0.0 && lateral.abs() > edge.abs() + 1.0 {
        return None;
    }
    let bin = ((dist / len) * BINS as f64) as usize;
    let pos = v.mPos;
    Some((bin.min(BINS - 1), pos.x, pos.z, pit))
}

#[cfg(test)]
mod tests {
    use super::super::data::{ScoringInfo, Vect3};
    use super::*;

    /// A 4 km oval-ish track (a circle of radius ~637 m, driven clockwise seen from above:
    /// north, then east, then south) with a pit lane inside the first stretch.
    fn circle(len: f64, at: f64) -> (f64, f64) {
        let r = len / std::f64::consts::TAU;
        let a = at / len * std::f64::consts::TAU;
        // Start at the south, heading west... clockwise: x = -r sin a, z = -r cos a.
        (-r * a.sin(), -r * a.cos())
    }

    fn snap(et: f64, cars: &[(f64, bool)]) -> Snapshot {
        let mut info = ScoringInfo::zeroed();
        info.mLapDist = 4000.0;
        info.mCurrentET = et;
        let vehicles = cars
            .iter()
            .enumerate()
            .map(|(i, &(dist, pit))| {
                let mut v = VehicleScoring::zeroed();
                v.mID = i as i32;
                v.mLapDist = dist;
                v.mInPits = pit as u8;
                v.mTrackEdge = 6.0;
                v.mLocalVel = Vect3 { x: 0.0, y: 0.0, z: -50.0 };
                let (x, z) = circle(4000.0, dist);
                // The pit lane runs 20 m inside the track.
                let k = if pit { 0.97 } else { 1.0 };
                v.mPos = Vect3 { x: x * k, y: 0.0, z: z * k };
                v
            })
            .collect();
        Snapshot { info, vehicles, telemetry: vec![] }
    }

    #[test]
    fn builds_the_outline_from_every_car_and_the_pit_lane() {
        let mut b = MapBuilder::default();
        // 20 cars spread round the lap at 50 m/s, 2 of them through the pit lane, 5 updates a second.
        let mut et = 0.0;
        let map = loop {
            let cars: Vec<(f64, bool)> = (0..20).map(|i| (((i as f64) * 200.0 + et * 50.0) % 4000.0, i < 2 && (et * 50.0) % 4000.0 < 300.0)).collect();
            b.feed(&snap(et, &cars), 1_000_000_123, "Test Ring");
            et += 0.2;
            if let Some(m) = b.take() {
                break m;
            }
            assert!(et < 60.0, "not complete after a minute");
        };
        assert!(et < 30.0, "complete in {et:.1} s with 20 cars");
        assert!(b.take().is_none(), "handed over once");
        assert_eq!((map.track_id, map.source.as_str(), map.points.len()), (1_000_000_123, "lmu", BINS));
        // A circle: every point the same distance from the middle of the unit box.
        let r: Vec<f32> = map.points.iter().map(|p| ((p[0] - 0.5).powi(2) + (p[1] - 0.5).powi(2)).sqrt()).collect();
        let (lo, hi) = r.iter().fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(hi - lo < 0.02, "round: {lo}..{hi}");
        // Start at the bottom (south), then going west = left on screen (x falls): north up, east right.
        assert!(map.points[0][1] > 0.95, "start at the south (bottom): {:?}", map.points[0]);
        assert!(map.points[BINS / 8][0] < map.points[0][0], "then west (left)");
        assert!(map.points[BINS / 4][0] < 0.05, "west side at a quarter lap");
        // The pit lane: drawn, inside the track.
        assert!(!map.pit.is_empty(), "pit lane drawn");
        let pr = ((map.pit[0][0][0] - 0.5).powi(2) + (map.pit[0][0][1] - 0.5).powi(2)).sqrt();
        assert!(pr < lo, "pit lane inside the track ({pr} < {lo})");
    }

    #[test]
    fn one_fast_car_alone_draws_the_track_in_one_lap() {
        let mut b = MapBuilder::default();
        // Alone on track at 80 m/s (16 m between updates, more than one 8 m stretch).
        let (speed, mut dist, mut et) = (80.0, 0.0, 0.0);
        let mut laps = 0.0;
        while b.progress() < COMPLETE && laps < 1.05 {
            et += 0.2;
            dist += speed * 0.2;
            laps = dist / 4000.0;
            let mut s = snap(et, &[(dist % 4000.0, false)]);
            s.vehicles[0].mLocalVel.z = -speed;
            b.feed(&s, 7, "Test");
        }
        assert!(b.progress() >= COMPLETE, "one lap was enough ({:.0}% after {laps:.2} laps)", b.progress() * 100.0);
    }

    #[test]
    fn installed_in_the_engine_kept_and_loaded_next_time() {
        use crate::engine::trackmap::Recorder;
        let dir = std::env::temp_dir().join(format!("apex-lmu-map-{}", std::process::id()));
        let mut b = MapBuilder::default();
        let mut et = 0.0;
        let map = loop {
            let cars: Vec<(f64, bool)> = (0..20).map(|i| ((i as f64 * 200.0 + et * 50.0) % 4000.0, false)).collect();
            b.feed(&snap(et, &cars), 1_000_000_321, "Test Ring");
            et += 0.2;
            if let Some(m) = b.take() {
                break m;
            }
        };
        let mut r = Recorder::new(Some(dir.clone()));
        r.load(1_000_000_321, "Test Ring");
        assert!(r.map.is_none());
        r.building = Some(0.5);
        assert!(r.recording() && (r.progress() - 0.5).abs() < 1e-6, "progress shown while building");
        r.install(map.clone());
        assert_eq!(r.progress(), 1.0);
        assert!(!r.recording());
        // Next visit: loaded from disk straight away, as it was.
        let mut again = Recorder::new(Some(dir.clone()));
        again.load(1_000_000_321, "Test Ring");
        assert_eq!(again.map.as_ref(), Some(&map));
        assert!(again.heading_at(0.25).is_some(), "the compass can follow the outline");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn leaves_out_parked_and_off_track_cars_and_starts_again() {
        let mut b = MapBuilder::default();
        let mut s = snap(1.0, &[(1000.0, false), (2000.0, false), (3000.0, false)]);
        s.vehicles[0].mLocalVel = Vect3::default(); // parked
        s.vehicles[1].mPathLateral = 30.0; // in the gravel
        b.feed(&s, 7, "T");
        assert_eq!(b.track.iter().map(|s| s.n).sum::<u32>(), 1);
        // The same scoring update again adds nothing.
        b.feed(&s, 7, "T");
        assert_eq!(b.track.iter().map(|s| s.n).sum::<u32>(), 1);
        b.reset();
        assert_eq!(b.progress(), 0.0);
        // Another track starts afresh.
        s.info.mCurrentET = 2.0;
        b.feed(&s, 8, "U");
        assert_eq!(b.track_id, 8);
    }
}
