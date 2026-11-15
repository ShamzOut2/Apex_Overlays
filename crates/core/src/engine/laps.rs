//! Reference laps recorded from your own driving, for the delta bar: your best lap this
//! session and your all-time best for the car and track (saved to disk, so it carries over
//! between sessions). A lap counts when it was driven from line to line without going off
//! track or onto pit road.

use crate::garage61::{Reference, REF_STEPS};
use crate::model::{surface, SessionInfo, Telemetry};
use std::path::PathBuf;

/// How long after the line to wait for iRacing's official lap time.
const SETTLE: f64 = 1.0;

#[derive(Default)]
pub struct LapRecorder {
    dir: Option<PathBuf>,
    /// Car and track ("<car id>-<track id>") the references belong to.
    combo: String,
    session_key: (i64, i32),
    lap: i32,
    /// Session time when the lap in progress began (the lap counter ticking over).
    lap_start: f64,
    /// (lap distance, seconds into the lap) of the lap in progress.
    samples: Vec<(f32, f32)>,
    valid: bool,
    /// A finished lap waiting for its official time, and when it finished.
    pending: Option<(Vec<(f32, f32)>, f64)>,
    pub session: Option<Reference>,
    pub all_time: Option<Reference>,
    /// Bumped whenever a reference changes, so the topic is only re-sent then.
    pub revision: u64,
}

/// Time at each step of lap distance from a lap's samples, ending at the official lap time.
pub fn resample(samples: &[(f32, f32)], lap_time: f32) -> Option<Vec<f32>> {
    let first = samples.first()?;
    let last = samples.last()?;
    if first.0 > 0.03 || last.0 < 0.97 || lap_time <= 0.0 {
        return None;
    }
    let mut pts: Vec<(f32, f32)> = vec![(0.0, 0.0)];
    pts.extend(samples.iter().copied().filter(|p| p.0 > 0.0 && p.0 < 1.0));
    pts.push((1.0, lap_time));
    let mut out = Vec::with_capacity(REF_STEPS + 1);
    let mut j = 0;
    for k in 0..=REF_STEPS {
        let x = k as f32 / REF_STEPS as f32;
        while j + 1 < pts.len() - 1 && pts[j + 1].0 < x {
            j += 1;
        }
        let (a, b) = (pts[j], pts[(j + 1).min(pts.len() - 1)]);
        let t = if b.0 > a.0 { a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0) } else { a.1 };
        out.push(t.clamp(0.0, lap_time));
    }
    // Keep it rising: a jitter in the samples must not make time run backwards.
    for k in 1..out.len() {
        out[k] = out[k].max(out[k - 1]);
    }
    Some(out)
}

/// A reference lap that makes sense: starts at 0, keeps rising, and reaches each part of the
/// lap in a believable share of the lap time. Laps recorded before 0.9.6 could take the
/// previous lap's time for their first moments (iRacing's lap timer resets a little after the
/// line), which made the whole lap read as that time and the delta count down to the line.
pub fn plausible(times: &[f32], lap_time: f32) -> bool {
    let n = times.len();
    if n < 11 || lap_time <= 0.0 {
        return false;
    }
    let at = |x: f32| times[((n - 1) as f32 * x).round() as usize] / lap_time;
    times.windows(2).all(|w| w[1] >= w[0])
        && at(0.0) < 0.01
        && at(0.1) < 0.4
        && (0.15..0.85).contains(&at(0.5))
        && at(0.9) > 0.6
        && (at(1.0) - 1.0).abs() < 0.01
}

impl LapRecorder {
    pub fn new(dir: Option<PathBuf>) -> Self {
        Self { dir, ..Default::default() }
    }

    fn file(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(format!("{}.json", self.combo)))
    }

    pub fn update(&mut self, t: &Telemetry, info: &SessionInfo, driver: &str) {
        let me = usize::try_from(info.player_car_idx).ok();
        let car_id = me.and_then(|i| info.driver(i)).map(|d| d.car_id).unwrap_or(0);
        if car_id == 0 || info.track_id == 0 {
            return;
        }
        let combo = format!("{car_id}-{}", info.track_id);
        if combo != self.combo {
            self.combo = combo;
            self.all_time = self.file().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|s| serde_json::from_str::<Reference>(&s).ok());
            // A broken reference (see `plausible`) is forgotten, so a good lap can replace it.
            if self.all_time.as_ref().is_some_and(|r| !plausible(&r.times, r.lap_time as f32)) {
                log::info!("lap reference for {} was not a usable lap; removed", self.combo);
                self.all_time = None;
                if let Some(f) = self.file() {
                    let _ = std::fs::remove_file(f);
                }
            }
            self.session = None;
            self.restart(t.lap);
            self.revision += 1;
        }
        let key = (info.subsession_id, t.session_num);
        if key != self.session_key {
            self.session_key = key;
            if self.session.take().is_some() {
                self.revision += 1;
            }
            self.restart(t.lap);
        }
        if self.pending.as_ref().is_some_and(|(_, at)| t.session_time - at >= SETTLE) {
            if let Some((samples, _)) = self.pending.take() {
                self.finish(&samples, t.lap_last_time, driver);
            }
        }
        if !t.driving() {
            self.restart(t.lap);
            return;
        }
        if t.lap != self.lap {
            if self.valid && self.samples.len() > 20 {
                self.pending = Some((std::mem::take(&mut self.samples), t.session_time));
            }
            self.restart(t.lap);
            self.lap_start = t.session_time;
        }
        let surf = me.map(|i| t.cars[i].surface).unwrap_or(surface::ON_TRACK);
        if t.on_pit_road || surf == surface::OFF_TRACK || surf == surface::NOT_IN_WORLD {
            self.valid = false;
        }
        // Time into the lap by the sim clock from the moment the lap began: iRacing's own lap
        // timer still shows the previous lap's time for a moment after the line.
        let p = (t.lap_dist_pct, (t.session_time - self.lap_start).max(0.0) as f32);
        match self.samples.last() {
            // A reset or tow moves the car backwards: the lap no longer counts.
            Some(l) if p.0 < l.0 - 0.01 => self.valid = false,
            Some(l) if p.0 <= l.0 => {}
            None if p.0 > 0.03 => self.valid = false,
            _ => self.samples.push(p),
        }
    }

    fn restart(&mut self, lap: i32) {
        self.lap = lap;
        self.samples.clear();
        self.valid = true;
    }

    /// A clean lap: keep it if it beats the session or all-time best.
    fn finish(&mut self, samples: &[(f32, f32)], official: f32, driver: &str) {
        let Some(&(pct, time)) = samples.last() else { return };
        let estimate = time / pct.max(0.5);
        // iRacing's time for the lap, unless it does not belong to this lap.
        let lap_time = if official > 0.0 && (official - estimate).abs() < estimate * 0.02 { official } else { estimate };
        let Some(times) = resample(samples, lap_time) else { return };
        if !plausible(&times, lap_time) {
            return;
        }
        let r = |source: &str| Reference { driver: driver.to_string(), lap_time: lap_time as f64, source: source.into(), times: times.clone() };
        if self.session.as_ref().is_none_or(|s| (lap_time as f64) < s.lap_time) {
            self.session = Some(r("session"));
            self.revision += 1;
        }
        if self.all_time.as_ref().is_none_or(|s| (lap_time as f64) < s.lap_time) {
            let best = r("allTime");
            if let (Some(f), Ok(json)) = (self.file(), serde_json::to_string(&best)) {
                if let Some(d) = f.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = std::fs::write(f, json);
            }
            self.all_time = Some(best);
            self.revision += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resamples_a_lap() {
        let samples: Vec<(f32, f32)> = (0..=99).map(|i| (i as f32 / 100.0 + 0.001, i as f32 * 0.9)).collect();
        let t = resample(&samples, 90.0).unwrap();
        assert_eq!(t.len(), REF_STEPS + 1);
        assert_eq!(t[0], 0.0);
        assert_eq!(*t.last().unwrap(), 90.0);
        assert!((t[REF_STEPS / 2] - 45.0).abs() < 0.2, "{}", t[REF_STEPS / 2]);
        // A lap joined halfway round is not a reference.
        assert!(resample(&samples[50..], 90.0).is_none());
    }

    #[test]
    fn records_clean_laps_and_keeps_the_best() {
        use crate::model::SessionInfo;
        let dir = std::env::temp_dir().join(format!("apex-laps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut rec = LapRecorder::new(Some(dir.clone()));
        let info = SessionInfo::parse("WeekendInfo:\n TrackID: 7\nDriverInfo:\n DriverCarIdx: 0\n Drivers:\n - CarIdx: 0\n   UserName: Me\n   CarID: 33\n");
        let mut t = Telemetry { is_on_track: true, player_idx: 0, ..Default::default() };
        t.cars[0].surface = surface::ON_TRACK;
        let mut now = 0.0;
        // Out lap from the middle of the track, then laps of 100 s, 95 s (off track) and 98 s.
        let mut drive = |t: &mut Telemetry, lap: i32, from: f32, secs: f32, off: bool| {
            let n = 200;
            for i in 0..n {
                let pct = from + (1.0 - from) * i as f32 / n as f32;
                t.lap = lap;
                t.lap_dist_pct = pct;
                t.lap_current_time = secs * pct;
                t.cars[0].surface = if off && i == 50 { surface::OFF_TRACK } else { surface::ON_TRACK };
                now += (secs / n as f32) as f64;
                t.session_time = now;
                rec.update(t, &info, "Me");
            }
            t.lap_last_time = secs;
        };
        drive(&mut t, 1, 0.5, 100.0, false);
        drive(&mut t, 2, 0.0, 100.0, false);
        drive(&mut t, 3, 0.0, 95.0, true);
        drive(&mut t, 4, 0.0, 98.0, false);
        drive(&mut t, 5, 0.0, 120.0, false);
        let best = rec.session.as_ref().unwrap();
        assert!((best.lap_time - 98.0).abs() < 0.01, "{}", best.lap_time);
        assert!(dir.join("33-7.json").exists());
        // A new recorder (next session) still has the all-time best.
        let mut again = LapRecorder::new(Some(dir.clone()));
        again.update(&t, &info, "Me");
        assert!(again.session.is_none());
        assert!((again.all_time.as_ref().unwrap().lap_time - 98.0).abs() < 0.01);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ignores_the_previous_laps_time_after_the_line() {
        use crate::model::SessionInfo;
        let dir = std::env::temp_dir().join(format!("apex-laps-stale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // A reference saved by an older version: flat at about the lap time from the start.
        let broken = Reference {
            driver: "Me".into(),
            lap_time: 21.725,
            source: "allTime".into(),
            times: (0..=REF_STEPS).map(|k| if k == 0 { 0.0 } else { 21.6 + 0.125 * k as f32 / REF_STEPS as f32 }).collect(),
        };
        std::fs::write(dir.join("33-7.json"), serde_json::to_string(&broken).unwrap()).unwrap();
        let mut rec = LapRecorder::new(Some(dir.clone()));
        let info = SessionInfo::parse("WeekendInfo:\n TrackID: 7\nDriverInfo:\n DriverCarIdx: 0\n Drivers:\n - CarIdx: 0\n   UserName: Me\n   CarID: 33\n");
        let mut t = Telemetry { is_on_track: true, player_idx: 0, ..Default::default() };
        t.cars[0].surface = surface::ON_TRACK;
        rec.update(&t, &info, "Me");
        assert!(rec.all_time.is_none(), "the broken reference is dropped");
        assert!(!dir.join("33-7.json").exists());
        let mut now = 0.0;
        for lap in 1..=3 {
            let n = 400;
            for i in 0..n {
                let pct = i as f32 / n as f32;
                t.lap = lap;
                t.lap_dist_pct = pct;
                // iRacing's timer shows the previous lap's 21.7 s for the first few ticks.
                t.lap_current_time = if i < 5 && lap > 1 { 21.7 } else { 21.7 * pct };
                now += 21.7 / n as f64;
                t.session_time = now;
                rec.update(&t, &info, "Me");
            }
            t.lap_last_time = 21.7;
        }
        let best = rec.session.as_ref().expect("a reference lap");
        assert!((best.lap_time - 21.7).abs() < 0.05, "{}", best.lap_time);
        let half = best.times[REF_STEPS / 2];
        assert!((half - 10.85).abs() < 0.3, "half way in {half} s, not the previous lap's time");
        assert!(plausible(&best.times, best.lap_time as f32));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
