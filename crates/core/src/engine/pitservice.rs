//! How long a pit stop takes for what is done at it: fuel (litres per second), tyres (how
//! many), and whether the crew does them at the same time or one after the other. Learned per
//! car from your own stops (iRacing): the fuel level rising while you stand in the box gives
//! the fill rate, and the time left over once the fuel is in, the tyres. Kept on disk; with
//! sharing on (opt-in), the same few numbers per stop go to a shared pool, and the pool's
//! figures stand in until your own stops have been measured.

use crate::model::Telemetry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// irsdk_PitSvFlags: the four tyres, and fuel.
pub const TYRES_MASK: u32 = 0x0f;
pub const FUEL_FILL: u32 = 0x10;

/// One stop, as measured: only what the prediction needs (no driver, session or track).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopSample {
    pub car_id: i64,
    /// Tyres changed (0-4).
    pub tyres: u8,
    /// Litres put in, and over how long the level rose.
    pub fuel_l: f32,
    pub fuel_secs: f32,
    /// Seconds from stopping to the fuel starting to go in (tyres first, when long).
    pub fuel_delay: f32,
    /// Seconds stood in the box.
    pub stood: f32,
}

impl StopSample {
    /// Plausible for a real stop (anything else is left out, here and in the shared pool).
    pub fn plausible(&self) -> bool {
        self.car_id > 0
            && self.tyres <= 4
            && (0.0..=200.0).contains(&self.fuel_l)
            && (0.0..=200.0).contains(&self.fuel_secs)
            && (0.0..=120.0).contains(&self.fuel_delay)
            && (1.5..=300.0).contains(&self.stood)
            && self.fuel_secs <= self.stood + 1.0
            && (self.fuel_l < 1.0 || (self.fuel_secs > 0.5 && (0.3..=15.0).contains(&(self.fuel_l / self.fuel_secs))))
    }
}

/// What a stop takes in one car: worked out from its stops (yours, or the shared pool's).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ServiceModel {
    /// Litres per second.
    pub rate: Option<f32>,
    /// Seconds a stop takes beyond its fuel and tyres (stopping, the jacks, release).
    pub base: f32,
    /// Tyres changed at the same time as the fuel goes in (else after or before it).
    pub parallel: Option<bool>,
    /// Seconds to change 1, 2, 3 and 4 tyres, where measured.
    pub tyres: [Option<f32>; 4],
    /// Stops it was worked out from.
    pub stops: u32,
}

fn median(mut v: Vec<f32>) -> Option<f32> {
    v.sort_by(f32::total_cmp);
    (!v.is_empty()).then(|| v[v.len() / 2])
}

impl ServiceModel {
    /// The model from a car's stops.
    pub fn from_samples(samples: &[StopSample]) -> Self {
        let rate = median(samples.iter().filter(|s| s.fuel_l >= 2.0 && s.fuel_secs >= 1.0).map(|s| s.fuel_l / s.fuel_secs).collect());
        // Fuel only: what is left once the fuel is in is the stop's own time.
        let base = median(samples.iter().filter(|s| s.tyres == 0 && s.fuel_l >= 1.0).map(|s| (s.stood - s.fuel_secs - s.fuel_delay).max(0.0)).collect())
            .unwrap_or(0.0);
        // Tyres and fuel at the same time: tyres changed with no time beyond the fuel. One
        // after the other: the fuel started late (tyres first) or the time beyond the fuel
        // is the tyres' (the same however much fuel went in).
        let with_both: Vec<&StopSample> = samples.iter().filter(|s| s.tyres > 0 && s.fuel_l >= 2.0).collect();
        let together = with_both.iter().filter(|s| s.stood <= s.fuel_secs + s.fuel_delay + base + 1.5).count();
        let late_fuel = with_both.iter().filter(|s| s.fuel_delay > 3.0).count();
        let parallel = if late_fuel > together {
            Some(false)
        } else if together > 0 {
            Some(true)
        } else if with_both.len() >= 2 {
            // Time beyond the fuel the same whatever the fuel: one after the other.
            Some(false)
        } else {
            None
        };
        let mut tyres = [None; 4];
        for (n, slot) in tyres.iter_mut().enumerate() {
            let count = n as u8 + 1;
            let v: Vec<f32> = samples
                .iter()
                .filter(|s| s.tyres == count)
                .filter_map(|s| {
                    if s.fuel_l < 1.0 {
                        // Tyres only.
                        Some((s.stood - base).max(0.0))
                    } else if parallel == Some(true) {
                        // Only a stop where the tyres took longer than the fuel says how long.
                        (s.stood > s.fuel_secs + s.fuel_delay + base + 1.5).then(|| (s.stood - base).max(0.0))
                    } else {
                        Some((s.stood - s.fuel_secs - base).max(0.0))
                    }
                })
                .collect();
            *slot = median(v);
        }
        Self { rate, base, parallel, tyres, stops: samples.len() as u32 }
    }

    /// Seconds to change `n` tyres: as measured, else in proportion to another count.
    pub fn tyre_secs(&self, n: u8) -> Option<f32> {
        if n == 0 {
            return Some(0.0);
        }
        let n = n.min(4) as usize;
        self.tyres[n - 1].or_else(|| self.tyres.iter().enumerate().rev().find_map(|(i, t)| t.map(|t| t * n as f32 / (i + 1) as f32)))
    }

    /// Seconds stood in the box for `fuel_l` litres and `tyres` tyres; None when a part of
    /// it has not been measured (no fill rate with fuel to add, no tyre time with tyres).
    pub fn predict(&self, fuel_l: f32, tyres: u8) -> Option<StopPlan> {
        let fuel_secs = if fuel_l > 0.0 { fuel_l / self.rate? } else { 0.0 };
        let tyre_secs = self.tyre_secs(tyres)?;
        let parallel = self.parallel.unwrap_or(false);
        let stood = self.base + if parallel { fuel_secs.max(tyre_secs) } else { fuel_secs + tyre_secs };
        Some(StopPlan { fuel_l, fuel_secs, tyres, tyre_secs, parallel, base: self.base, stood })
    }

    /// Each figure from this model where it has one, else from `other`.
    pub fn or(&self, other: &ServiceModel) -> ServiceModel {
        let mut tyres = self.tyres;
        for (t, o) in tyres.iter_mut().zip(other.tyres) {
            *t = t.or(o);
        }
        ServiceModel {
            rate: self.rate.or(other.rate),
            base: if self.stops > 0 { self.base } else { other.base },
            parallel: self.parallel.or(other.parallel),
            tyres,
            stops: self.stops + other.stops,
        }
    }
}

/// A stop worked out: what is done and how long each part takes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopPlan {
    pub fuel_l: f32,
    pub fuel_secs: f32,
    pub tyres: u8,
    pub tyre_secs: f32,
    pub parallel: bool,
    pub base: f32,
    pub stood: f32,
}

/// Your stop under way.
#[derive(Debug, Clone, Copy)]
struct Standing {
    since: f64,
    fuel_start: f32,
    fuel_max: f32,
    fill_from: Option<f64>,
    fill_to: f64,
    tyres: u8,
    repair: bool,
    penalty: bool,
}

/// Learns your stops and keeps each car's samples on disk.
#[derive(Debug, Default)]
pub struct PitService {
    file: Option<PathBuf>,
    /// Car id → its last stops (yours).
    samples: BTreeMap<i64, Vec<StopSample>>,
    /// Car id → the shared pool's model for it.
    pub community: BTreeMap<i64, ServiceModel>,
    standing: Option<Standing>,
    /// Stops measured and not yet offered to the shared pool (taken by the uploader).
    pub outbox: Vec<StopSample>,
}

const KEEP: usize = 30;

impl PitService {
    pub fn new(file: Option<PathBuf>) -> Self {
        let samples = file.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Self { file, samples, ..Default::default() }
    }

    /// Your car's model: your own stops first, the shared pool's for what they have not shown.
    pub fn model(&self, car_id: i64) -> Option<(ServiceModel, &'static str)> {
        let mine = self.samples.get(&car_id).filter(|v| !v.is_empty()).map(|v| ServiceModel::from_samples(v));
        let pool = self.community.get(&car_id).filter(|m| m.stops > 0);
        match (mine, pool) {
            (Some(m), Some(p)) => Some((m.or(p), "you")),
            (Some(m), None) => Some((m, "you")),
            (None, Some(p)) => Some((p.clone(), "community")),
            (None, None) => None,
        }
    }

    /// Follows your car (iRacing): a stop starts when you stand in your box, the fuel level
    /// rising while you stand gives the fill, and it is recorded when you drive off.
    pub fn update(&mut self, t: &Telemetry, car_id: i64, black_flag: bool) {
        let standing = t.in_pit_stall && t.speed.abs() < 0.3 && t.is_on_track;
        match (&mut self.standing, standing) {
            (None, true) => {
                self.standing = Some(Standing {
                    since: t.session_time,
                    fuel_start: t.fuel_level,
                    fuel_max: t.fuel_level,
                    fill_from: None,
                    fill_to: t.session_time,
                    tyres: (t.pit_sv_flags & TYRES_MASK).count_ones() as u8,
                    repair: t.pit_repair_left > 0.0,
                    penalty: black_flag,
                })
            }
            (Some(s), true) => {
                if t.fuel_level > s.fuel_max + 0.005 {
                    s.fill_from.get_or_insert(t.session_time);
                    s.fill_to = t.session_time;
                    s.fuel_max = t.fuel_level;
                }
                s.repair |= t.pit_repair_left > 0.0;
                s.penalty |= black_flag;
            }
            (Some(s), false) => {
                let s = *s;
                self.standing = None;
                // A stop to learn from: driven off from the box, no repairs or penalty in it.
                if s.repair || s.penalty || car_id <= 0 {
                    return;
                }
                let fuel_l = (s.fuel_max - s.fuel_start).max(0.0);
                let filled = fuel_l >= 0.5;
                let sample = StopSample {
                    car_id,
                    tyres: s.tyres,
                    fuel_l: if filled { fuel_l } else { 0.0 },
                    fuel_secs: if filled { (s.fill_to - s.fill_from.unwrap_or(s.fill_to)) as f32 } else { 0.0 },
                    fuel_delay: if filled { (s.fill_from.unwrap_or(s.since) - s.since) as f32 } else { 0.0 },
                    stood: (t.session_time - s.since) as f32,
                };
                if sample.plausible() {
                    log::info!(
                        "pit service: car {car_id}: {:.1} s stood, {:.1} L in {:.1} s (from {:.1} s in), {} tyres",
                        sample.stood,
                        sample.fuel_l,
                        sample.fuel_secs,
                        sample.fuel_delay,
                        sample.tyres
                    );
                    self.add(sample);
                    self.outbox.push(sample);
                }
            }
            (None, false) => {}
        }
    }

    fn add(&mut self, s: StopSample) {
        let v = self.samples.entry(s.car_id).or_default();
        v.push(s);
        if v.len() > KEEP {
            v.drain(..v.len() - KEEP);
        }
        if let (Some(f), Ok(json)) = (&self.file, serde_json::to_string(&self.samples)) {
            let _ = std::fs::write(f, json);
        }
    }
}

/// What you have set in the black box: litres the crew will put in (no more than fits) and
/// tyres to change.
pub fn planned(t: &Telemetry, capacity: f32) -> (f32, u8) {
    let fuel = if t.pit_sv_flags & FUEL_FILL != 0 { t.pit_sv_fuel.min((capacity - t.fuel_level).max(0.0)).max(0.0) } else { 0.0 };
    (fuel, (t.pit_sv_flags & TYRES_MASK).count_ones() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(tyres: u8, fuel_l: f32, fuel_secs: f32, delay: f32, stood: f32) -> StopSample {
        StopSample { car_id: 1, tyres, fuel_l, fuel_secs, fuel_delay: delay, stood }
    }

    #[test]
    fn fuel_and_tyres_at_the_same_time() {
        // 2.5 L/s; four tyres take 20 s; the stop itself 1 s.
        let m = ServiceModel::from_samples(&[s(0, 50.0, 20.0, 0.2, 21.2), s(4, 75.0, 30.0, 0.2, 31.3), s(4, 20.0, 8.0, 0.2, 21.0)]);
        assert!((m.rate.unwrap() - 2.5).abs() < 0.01);
        assert_eq!(m.parallel, Some(true));
        assert!((m.tyre_secs(4).unwrap() - 20.0).abs() < 0.3);
        // 60 L (24 s) and four tyres (20 s) together: 24 s plus the stop's own second.
        let p = m.predict(60.0, 4).unwrap();
        assert!((p.stood - 25.0).abs() < 0.3, "{p:?}");
        // Two tyres: half the time of four.
        assert!((m.tyre_secs(2).unwrap() - 10.0).abs() < 0.3);
    }

    #[test]
    fn tyres_after_the_fuel() {
        // 3 L/s, then 4 tyres in 16 s.
        let m = ServiceModel::from_samples(&[s(0, 30.0, 10.0, 0.0, 10.5), s(4, 60.0, 20.0, 0.0, 36.5), s(4, 30.0, 10.0, 0.0, 26.5)]);
        assert_eq!(m.parallel, Some(false));
        let p = m.predict(45.0, 4).unwrap();
        assert!((p.stood - (0.5 + 15.0 + 16.0)).abs() < 0.3, "{p:?}");
        // Nothing measured for the fuel: no prediction with fuel to add.
        assert!(ServiceModel::default().predict(10.0, 0).is_none());
        assert_eq!(ServiceModel::default().predict(0.0, 0).map(|p| p.stood), Some(0.0));
    }

    #[test]
    fn your_stops_first_then_the_pool() {
        let mut p = PitService::default();
        p.community.insert(1, ServiceModel { rate: Some(2.0), tyres: [None, None, None, Some(18.0)], parallel: Some(true), stops: 40, ..Default::default() });
        assert_eq!(p.model(1).unwrap().1, "community");
        p.add(s(0, 40.0, 10.0, 0.0, 10.5));
        let (m, from) = p.model(1).unwrap();
        assert_eq!(from, "you");
        // Your fill rate (4 L/s), the pool's tyres.
        assert!((m.rate.unwrap() - 4.0).abs() < 0.01 && m.tyres[3] == Some(18.0));
        assert!(!s(4, 500.0, 10.0, 0.0, 20.0).plausible());
    }
}
