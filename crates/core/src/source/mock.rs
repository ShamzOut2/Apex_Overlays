//! Built-in race simulator for developing and previewing overlays without
//! iRacing: a three-class field on a synthetic circuit, with fuel use, pit
//! stops and a rain shower that comes and goes (so the tyre advisor has
//! something to say). It emits the same Telemetry + YAML a live session does.

use super::{Poll, Source};
use crate::engine::trackmap::{normalise, TrackMap};
use crate::model::{surface, Telemetry};
use std::f64::consts::TAU;
use std::time::{Duration, Instant};

const DT: f64 = 1.0 / 60.0;
const SAMPLES: usize = 3000;
const TARGET_LEN: f64 = 4600.0;
/// Race length; override with APEX_OVERLAYS_MOCK_RACE_SECONDS to test race finishes.
const RACE_SECONDS: f64 = 50.0 * 60.0;
const PIT_ENTRY: f64 = 0.955;
const PIT_EXIT: f64 = 0.04;
const PIT_STALL: f64 = 0.012;
const PIT_SPEED: f64 = 22.0;
const WEATHER_CYCLE: f64 = 1320.0;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

/// Synthetic circuit: a lumpy closed curve with a precomputed speed profile.
pub struct Circuit {
    pub len: f64,
    xy: Vec<[f64; 2]>,
    heading: Vec<f64>,
    curv: Vec<f64>,
    vref: Vec<f64>,
    tref: Vec<f64>,
    pub lap_ref: f64,
}

impl Circuit {
    pub fn new() -> Self {
        let raw: Vec<[f64; 2]> = (0..SAMPLES)
            .map(|i| {
                let a = i as f64 / SAMPLES as f64 * TAU;
                let r = 1.0 + 0.22 * (2.0 * a).sin() + 0.12 * (3.0 * a + 1.0).sin() + 0.07 * (5.0 * a).cos() + 0.03 * (9.0 * a + 0.5).sin();
                [r * a.cos(), 0.68 * r * a.sin()]
            })
            .collect();
        let seg = |p: &[[f64; 2]], i: usize| {
            let (a, b) = (p[i], p[(i + 1) % SAMPLES]);
            ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
        };
        let raw_len: f64 = (0..SAMPLES).map(|i| seg(&raw, i)).sum();
        let k = TARGET_LEN / raw_len;
        let xy: Vec<[f64; 2]> = raw.iter().map(|p| [p[0] * k, p[1] * k]).collect();
        let step = TARGET_LEN / SAMPLES as f64;
        let heading: Vec<f64> = (0..SAMPLES)
            .map(|i| {
                let (a, b) = (xy[i], xy[(i + 1) % SAMPLES]);
                (b[1] - a[1]).atan2(b[0] - a[0])
            })
            .collect();
        let curv: Vec<f64> = (0..SAMPLES)
            .map(|i| {
                let mut d = heading[(i + 1) % SAMPLES] - heading[i];
                while d > std::f64::consts::PI {
                    d -= TAU;
                }
                while d < -std::f64::consts::PI {
                    d += TAU;
                }
                d / seg(&xy, i).max(1e-6)
            })
            .collect();
        // Smooth curvature a little so corners have entries and exits.
        let curv: Vec<f64> = (0..SAMPLES).map(|i| (0..9).map(|j| curv[(i + SAMPLES + j - 4) % SAMPLES]).sum::<f64>() / 9.0).collect();
        let mut v: Vec<f64> = curv.iter().map(|c| (24.0 / c.abs().max(1e-5)).sqrt().min(84.0)).collect();
        for _ in 0..2 {
            for i in (0..SAMPLES).rev() {
                let next = v[(i + 1) % SAMPLES];
                v[i] = v[i].min((next * next + 2.0 * 28.0 * step).sqrt());
            }
            for i in 0..SAMPLES {
                let prev = v[(i + SAMPLES - 1) % SAMPLES];
                let acc = (9.0 * (1.0 - prev / 95.0)).max(1.0);
                v[i] = v[i].min((prev * prev + 2.0 * acc * step).sqrt());
            }
        }
        let mut tref = vec![0.0; SAMPLES];
        for i in 1..SAMPLES {
            tref[i] = tref[i - 1] + step / v[i - 1];
        }
        let lap_ref = tref[SAMPLES - 1] + step / v[SAMPLES - 1];
        Self { len: TARGET_LEN, xy, heading, curv, vref: v, tref, lap_ref }
    }

    fn idx(&self, dist: f64) -> usize {
        ((dist.rem_euclid(self.len) / self.len) * SAMPLES as f64) as usize % SAMPLES
    }

    pub fn est_time(&self, pct: f64) -> f64 {
        self.tref[((pct.rem_euclid(1.0)) * SAMPLES as f64) as usize % SAMPLES]
    }

    /// Corners are the stretches where the speed profile drops well below top speed.
    pub fn corners(&self) -> Vec<crate::config::Corner> {
        const NAMES: [&str; 8] = ["La Première", "", "Lakeside", "The Kink", "", "Hairpin", "", "Last Chance"];
        let slow = |i: usize| self.vref[i] < 62.0;
        let mut out = vec![];
        let mut i = 0;
        while i < SAMPLES {
            if slow(i) {
                let start = i;
                while i < SAMPLES && slow(i) {
                    i += 1;
                }
                if i - start > 20 {
                    let n = out.len();
                    out.push(crate::config::Corner {
                        name: NAMES.get(n).unwrap_or(&"").to_string(),
                        number: n as i32 + 1,
                        start: start as f32 / SAMPLES as f32,
                        end: i as f32 / SAMPLES as f32,
                    });
                }
            }
            i += 1;
        }
        out
    }

    pub fn map(&self, track_id: i64) -> TrackMap {
        let pts: Vec<[f64; 2]> = (0..crate::engine::trackmap::BINS).map(|b| self.xy[b * SAMPLES / crate::engine::trackmap::BINS]).collect();
        TrackMap { track_id, name: "Apex Raceway".into(), points: normalise(&pts), source: "mock".into(), version: crate::engine::trackmap::MAP_VERSION }
    }
}

impl Default for Circuit {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Pit {
    None,
    Requested,
    Lane { served: bool },
    Stopped(f64),
}

struct Car {
    class: usize,
    skill: f64,
    dist: f64,
    lap: i32,
    lap_start: f64,
    last_lap: f64,
    best_lap: f64,
    fuel: f64,
    wets: bool,
    wet_trigger: i32,
    pit: Pit,
    incidents: i32,
    speed: f64,
    prev_speed: f64,
    wobble: f64,
    /// Off track (causing a yellow) until this time.
    off_until: f64,
    /// Meatball (repair) flag until this time.
    repair_until: f64,
    /// Black flag: Some(stop seconds), 0 = drive-through.
    penalty: Option<f64>,
    /// Serving a slow-down (furled black flag) until this time.
    slowdown_until: f64,
}

const CLASSES: [(&str, &str, u32, f64, f64); 3] = [
    // name, car, colour, pace factor, fuel capacity (L)
    ("GTP", "Porsche 963 GTP", 0xffda59, 1.00, 88.0),
    ("LMP2", "Dallara P217 LMP2", 0x33ceff, 1.07, 75.0),
    ("GT3", "BMW M4 GT3", 0xff5888, 1.13, 63.6),
];
const NAMES: [&str; 22] = [
    "Ayrton Vale",
    "Mika Rautio",
    "Lena Hartmann",
    "Jules Moreau",
    "Sofia Bianchi",
    "Kenji Arai",
    "Max Dunlop",
    "Priya Nair",
    "Tom Kerrigan",
    "Ana Ribeiro",
    "Lars Eklund",
    "Chloe Martin",
    "Diego Herrera",
    "Hannah Weiss",
    "Oscar Lind",
    "Yuki Tanaka",
    "Ravi Mehta",
    "Emma Clarke",
    "Felix Braun",
    "Nina Petrov",
    "Sam O'Neill",
    "Zoe Laurent",
];
const FLAIRS: [&str; 12] =
    ["United Kingdom", "Finland", "Germany", "France", "Italy", "Japan", "Australia", "India", "Ireland", "Brazil", "Sweden", "Netherlands"];
const PLAYER: usize = 14;

pub struct Mock {
    track: Circuit,
    cars: Vec<Car>,
    rng: Rng,
    time: f64,
    tick: i32,
    started: Instant,
    fast_forward_until: f64,
    sent_yaml: bool,
    incidents: i32,
    /// Smoothed longitudinal acceleration of the player car, drives the pedal inputs.
    accel_smooth: f64,
    race_seconds: f64,
    /// The leader has crossed the line after the clock ran out.
    checkered: bool,
    /// Make the player pit on this lap (APEX_OVERLAYS_MOCK_PIT_LAP, for trying the pit overlay).
    pit_lap: Option<i32>,
    /// The player's pit limiter stays on this long after the pit exit line.
    limiter_until: std::cell::Cell<f64>,
    player_was_on_pit: std::cell::Cell<bool>,
}

impl Mock {
    pub fn new() -> Self {
        let track = Circuit::new();
        let race_seconds = std::env::var("APEX_OVERLAYS_MOCK_RACE_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(RACE_SECONDS);
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut cars = vec![];
        for i in 0..20 {
            let class = match i {
                0..=5 => 0,
                6..=9 => 1,
                _ => 2,
            };
            let grid_gap = 22.0 * i as f64;
            cars.push(Car {
                class,
                skill: if i == 0 { 1.0 } else { rng.range(1.0, 1.025) },
                dist: (track.len - 30.0 - grid_gap).rem_euclid(track.len),
                lap: 0,
                lap_start: 0.0,
                last_lap: -1.0,
                best_lap: -1.0,
                fuel: CLASSES[class].4 * rng.range(0.85, 1.0),
                wets: false,
                wet_trigger: if i == PLAYER { 5 } else { (rng.range(3.0, 5.99)) as i32 },
                pit: Pit::None,
                incidents: 0,
                speed: 0.0,
                prev_speed: 0.0,
                wobble: rng.range(0.0, TAU),
                off_until: 0.0,
                repair_until: 0.0,
                slowdown_until: 0.0,
                penalty: None,
            });
        }
        Self {
            track,
            cars,
            rng,
            time: 0.0,
            tick: 0,
            started: Instant::now(),
            fast_forward_until: 420.0,
            sent_yaml: false,
            incidents: 0,
            accel_smooth: 0.0,
            race_seconds,
            checkered: false,
            pit_lap: std::env::var("APEX_OVERLAYS_MOCK_PIT_LAP").ok().and_then(|v| v.parse().ok()),
            limiter_until: Default::default(),
            player_was_on_pit: Default::default(),
        }
    }

    pub fn circuit(&self) -> &Circuit {
        &self.track
    }

    fn weather(&self) -> (i32, f64) {
        let p = self.time.rem_euclid(WEATHER_CYCLE);
        if p < 240.0 {
            (1, 0.0)
        } else if p < 720.0 {
            ((1 + ((p - 240.0) / 75.0) as i32).min(6), ((p - 240.0) / 200.0).min(0.7))
        } else {
            ((6 - ((p - 720.0) / 95.0) as i32).max(1), 0.0)
        }
    }

    fn step(&mut self) {
        self.time += DT;
        self.tick += 1;
        let (wetness, _) = self.weather();
        let len = self.track.len;
        let n = self.cars.len();
        let leader = (1..n).max_by(|&a, &b| {
            let p = |c: &Car| c.lap as f64 + c.dist / len;
            p(&self.cars[a]).total_cmp(&p(&self.cars[b]))
        });
        for i in 0..n {
            let wobble = (self.time * 0.05 + self.cars[i].wobble).sin() * 0.004;
            let c = &mut self.cars[i];
            let pct = c.dist / len;
            let grip = match (c.wets, wetness) {
                (false, w) if w >= 2 => 1.0 + (w - 1) as f64 * 0.028,
                (true, w) if w < 4 => 1.0 + (4 - w) as f64 * 0.015,
                (true, _) => 1.0 + (wetness - 1) as f64 * 0.012,
                _ => 1.0,
            };
            let pace = CLASSES[c.class].3 * c.skill * grip * (1.0 + wobble);
            let mut v = self.track.vref[self.track.idx(c.dist)] / pace;
            if self.time < c.off_until {
                v *= 0.35;
            }
            if self.time < c.slowdown_until {
                v *= 0.8;
            }

            // Pit strategy: fuel or tyres.
            let laps_of_fuel = c.fuel / (CLASSES[c.class].4 / 22.0);
            let wrong_tyres = (!c.wets && wetness >= c.wet_trigger) || (c.wets && wetness <= 2);
            let planned = i == PLAYER && self.pit_lap.is_some_and(|l| l == c.lap);
            if c.pit == Pit::None && (laps_of_fuel < 1.3 || wrong_tyres || c.penalty.is_some() || planned) && pct > 0.5 && pct < PIT_ENTRY {
                c.pit = Pit::Requested;
            }
            if c.pit == Pit::Requested && pct >= PIT_ENTRY {
                c.pit = Pit::Lane { served: false };
            }
            match c.pit {
                Pit::Lane { served } => {
                    v = v.min(PIT_SPEED);
                    if !served && (PIT_STALL..PIT_EXIT).contains(&pct) && c.penalty == Some(0.0) {
                        // Drive-through: no stop.
                        c.pit = Pit::Lane { served: true };
                    } else if !served && (PIT_STALL..PIT_EXIT).contains(&pct) && c.penalty.is_some() {
                        c.pit = Pit::Stopped(self.time + c.penalty.unwrap_or(0.0));
                        v = 0.0;
                    } else if !served && (PIT_STALL..PIT_EXIT).contains(&pct) {
                        let refuel = (CLASSES[c.class].4 - c.fuel) / 2.6;
                        let tyres = if wrong_tyres { 12.0 } else { 0.0 };
                        c.pit = Pit::Stopped(self.time + refuel.max(4.0) + tyres);
                        v = 0.0;
                    } else if (PIT_EXIT..0.5).contains(&pct) {
                        c.pit = Pit::None;
                        // Penalty served on pit exit.
                        c.penalty = None;
                    }
                }
                Pit::Stopped(until) => {
                    v = 0.0;
                    if self.time >= until && c.penalty.is_some() {
                        c.pit = Pit::Lane { served: true };
                    } else if self.time >= until {
                        c.fuel = CLASSES[c.class].4;
                        if wrong_tyres {
                            c.wets = !c.wets;
                        }
                        c.pit = Pit::Lane { served: true };
                    }
                }
                _ => {}
            }
            c.prev_speed = c.speed;
            c.speed = v;
            c.dist += v * DT;
            c.fuel = (c.fuel - v * DT * (CLASSES[c.class].4 / 22.0 / len) * 0.92).max(0.0);
            if c.dist >= len {
                c.dist -= len;
                c.lap += 1;
                if c.lap > 1 {
                    c.last_lap = self.time - c.lap_start;
                    if c.best_lap < 0.0 || c.last_lap < c.best_lap {
                        c.best_lap = c.last_lap;
                    }
                }
                c.lap_start = self.time;
                // The chequered flag comes out when the leader crosses the line after the clock runs out.
                if Some(i) == leader && self.time >= self.race_seconds {
                    self.checkered = true;
                }
            }
        }
        let me = &self.cars[PLAYER];
        let accel = (me.speed - me.prev_speed) / DT;
        self.accel_smooth = self.accel_smooth * 0.9 + accel.clamp(-40.0, 20.0) * 0.1;
        if self.rng.next() < 0.00005 {
            let r = self.rng.next();
            self.incidents += if r < 0.6 {
                1
            } else if r < 0.88 {
                2
            } else {
                4
            };
        }
        // Occasional incidents so the per-driver flags can be seen.
        if self.rng.next() < 1.0 / (60.0 * 45.0) {
            let i = 1 + (self.rng.next() * (n - 1) as f64) as usize;
            self.cars[i].off_until = self.time + 4.0;
        }
        if self.rng.next() < 1.0 / (60.0 * 90.0) {
            let i = 1 + (self.rng.next() * (n - 1) as f64) as usize;
            self.cars[i].repair_until = self.time + 25.0;
        }
        if self.rng.next() < 1.0 / (60.0 * 70.0) {
            let i = 1 + (self.rng.next() * (n - 1) as f64) as usize;
            if self.cars[i].penalty.is_none() && self.cars[i].pit == Pit::None {
                self.cars[i].penalty = Some(if self.rng.next() < 0.5 { 0.0 } else { 15.0 });
            }
        }
        // A slow-down for cutting the track, often for the car just ahead of you so the
        // safety alert's overtake warning can be seen.
        if self.rng.next() < 1.0 / (60.0 * 40.0) {
            let me = self.cars[PLAYER].dist;
            let ahead = (1..n).filter(|&i| i != PLAYER).min_by(|&a, &b| {
                let d = |i: usize| (self.cars[i].dist - me).rem_euclid(len);
                d(a).total_cmp(&d(b))
            });
            let i = if self.rng.next() < 0.5 { ahead.unwrap_or(1) } else { 1 + (self.rng.next() * (n - 1) as f64) as usize };
            if i != PLAYER {
                self.cars[i].slowdown_until = self.time + 8.0 + self.rng.next() * 8.0;
            }
        }
        if self.rng.next() < 0.0004 {
            let i = 1 + (self.rng.next() * (self.cars.len() - 1) as f64) as usize;
            self.cars[i].incidents += if self.rng.next() < 0.7 { 1 } else { 2 };
        }
    }

    fn yaml(&self) -> String {
        let mut y = format!(
            "---\nWeekendInfo:\n TrackName: apex raceway\n TrackID: 9001\n TrackLength: {:.2} km\n TrackDisplayName: Apex Raceway\n TrackConfigName: Grand Prix\n TrackPitSpeedLimit: 79.20 kph\n TrackCity: Nowhere\n TrackCountry: Simland\n SeriesID: 1\n SubSessionID: 424242\n LeagueID: 0\n Official: 1\n EventType: Race\n WeekendOptions:\n  IncidentLimit: 25\nSessionInfo:\n Sessions:\n - SessionNum: 0\n   SessionLaps: unlimited\n   SessionTime: {:.4} sec\n   SessionType: Race\n   SessionName: RACE\n   ResultsPositions:\n",
            self.track.len / 1000.0,
            self.race_seconds
        );
        y += &format!(
            "DriverInfo:\n DriverCarIdx: {PLAYER}\n PaceCarIdx: 0\n DriverCarFuelMaxLtr: 106.000\n DriverCarMaxFuelPct: 0.600\n DriverCarEstLapTime: {:.4}\n DriverCarSLShiftRPM: 7800.000\n DriverCarRedLine: 8500.000\n DriverTires:\n - TireIndex: 0\n   TireCompoundType: \"Hard\"\n - TireIndex: 1\n   TireCompoundType: \"Wet\"\n Drivers:\n",
            self.track.lap_ref * CLASSES[2].3
        );
        y += " - CarIdx: 0\n   UserName: Pace Car\n   CarIsPaceCar: 1\n   CarNumber: \"0\"\n";
        for (i, c) in self.cars.iter().enumerate().skip(1) {
            let (cls, car, color, pace, _) = CLASSES[c.class];
            let name = NAMES[(i - 1) % NAMES.len()];
            let ir = 1200 + ((i * 7919) % 4300) as i32;
            let lic = ["R", "D", "C", "B", "A", "P"][(ir as usize / 900).min(5)];
            y += &format!(
                " - CarIdx: {i}\n   UserName: {name}\n   AbbrevName: {}\n   UserID: {}\n   TeamName: {name}\n   CarNumber: \"{}\"\n   CarID: {}\n   CarPath: {}\n   CarScreenName: {car}\n   CarScreenNameShort: {car}\n   CarClassID: {}\n   CarClassShortName: {cls}\n   CarClassColor: 0x{color:06x}\n   CarClassEstLapTime: {:.4}\n   IRating: {ir}\n   LicString: {lic} {:.2}\n   LicColor: 0x{:06x}\n   ClubName: Mock Club\n   FlairName: {}\n   CurDriverIncidentCount: {}\n   IsSpectator: 0\n   CarIsPaceCar: 0\n",
                abbreviate(name),
                100000 + i,
                if i < 10 { format!("{}", i * 3) } else { format!("{}", 100 + i) },
                c.class + 1,
                car.to_lowercase().replace(' ', ""),
                c.class + 1,
                self.track.lap_ref * pace,
                1.5 + (i % 30) as f64 / 10.0,
                [0xfc0706, 0xff8c00, 0xfeec04, 0x00c702, 0x0153db, 0x000000][(ir as usize / 900).min(5)],
                FLAIRS[i % FLAIRS.len()],
                if i == PLAYER { self.incidents } else { c.incidents },
            );
        }
        y += "SplitTimeInfo:\n Sectors:\n - SectorNum: 0\n   SectorStartPct: 0.000000\n - SectorNum: 1\n   SectorStartPct: 0.330000\n - SectorNum: 2\n   SectorStartPct: 0.680000\n";
        y
    }

    fn fill(&self, t: &mut Telemetry) {
        let len = self.track.len;
        let (wetness, precip) = self.weather();
        t.tick = self.tick;
        t.session_time = self.time;
        t.session_num = 0;
        t.session_state = if self.checkered { 5 } else { 4 };
        // A local yellow every few minutes, a blue flag when a faster class is right behind, then the chequered flag.
        use crate::model::flags as fl;
        let cycle = self.time.rem_euclid(180.0);
        let mut f = if cycle < 8.0 { fl::GREEN } else { 0 };
        if (60.0..70.0).contains(&cycle) {
            f |= fl::YELLOW_WAVING | fl::YELLOW;
        }
        let me = &self.cars[PLAYER];
        let faster_behind = self.cars.iter().enumerate().skip(1).any(|(i, c)| {
            let d = (me.dist - c.dist).rem_euclid(len);
            i != PLAYER && c.class < me.class && d > 0.0 && d < 60.0
        });
        if faster_behind {
            f |= fl::BLUE;
        }
        if self.time >= self.race_seconds && !self.checkered {
            f |= fl::WHITE;
        }
        if self.checkered {
            f = fl::CHECKERED;
        }
        t.session_flags = f;
        t.session_time_remain = (self.race_seconds - self.time).max(0.0);
        t.session_laps_remain = 32767;
        t.time_of_day = (14.0 * 3600.0 + self.time) as f32;
        t.player_idx = PLAYER as i32;
        t.cam_car_idx = PLAYER as i32;
        t.track_wetness = wetness;
        t.precipitation = precip as f32;
        t.declared_wet = wetness >= 3;
        t.track_temp = 33.0 - 1.6 * (wetness - 1) as f32;
        t.air_temp = 24.0 - 0.6 * (wetness - 1) as f32;
        t.wind_vel = 3.2 + (self.time * 0.01).sin() as f32;
        t.wind_dir = 1.2;
        t.humidity = 0.45 + 0.08 * (wetness - 1) as f32;
        t.fog = 0.0;
        t.skies = if precip > 0.0 {
            3
        } else if wetness > 1 {
            2
        } else {
            1
        };
        t.air_pressure = 29.92;
        t.display_metric = true;

        let mut order: Vec<usize> = (1..self.cars.len()).collect();
        let prog = |c: &Car| c.lap as f64 + c.dist / len;
        order.sort_by(|&a, &b| prog(&self.cars[b]).total_cmp(&prog(&self.cars[a])));
        let mut class_counts = [0; 3];
        for c in t.cars.iter_mut() {
            *c = crate::model::CarTel { surface: surface::NOT_IN_WORLD, pct: -1.0, ..Default::default() };
        }
        for (pos, &i) in order.iter().enumerate() {
            let c = &self.cars[i];
            class_counts[c.class] += 1;
            let ct = &mut t.cars[i];
            let pct = c.dist / len;
            ct.lap = c.lap;
            ct.lap_completed = (c.lap - 1).max(0);
            ct.pct = pct as f32;
            ct.est_time = (self.track.est_time(pct) * CLASSES[c.class].3) as f32;
            ct.position = pos as i32 + 1;
            ct.class_position = class_counts[c.class];
            ct.on_pit_road = matches!(c.pit, Pit::Lane { .. } | Pit::Stopped(_)) && !(PIT_EXIT..PIT_ENTRY).contains(&pct);
            let heading_in = matches!(c.pit, Pit::Requested) && (PIT_ENTRY - 0.02..PIT_ENTRY).contains(&pct);
            ct.surface = if matches!(c.pit, Pit::Stopped(_)) {
                surface::IN_PIT_STALL
            } else if heading_in {
                surface::APPROACHING_PITS
            } else if self.time < c.off_until {
                surface::OFF_TRACK
            } else {
                surface::ON_TRACK
            };
            let faster_behind = self.cars.iter().skip(1).any(|o| {
                let d = (c.dist - o.dist).rem_euclid(len);
                o.class < c.class && d > 0.0 && d < 60.0 && o.lap >= c.lap
            });
            ct.session_flags = (if c.penalty.is_some() { crate::model::flags::BLACK } else { 0 })
                | (if self.time < c.repair_until { crate::model::flags::REPAIR } else { 0 })
                | (if self.time < c.slowdown_until { crate::model::flags::FURLED } else { 0 })
                | (if faster_behind { crate::model::flags::BLUE } else { 0 });
            ct.last_lap = c.last_lap as f32;
            ct.best_lap = c.best_lap as f32;
            ct.tire_compound = c.wets as i32;
            ct.gear = gear(c.speed);
            ct.rpm = rpm(c.speed) as f32;
        }

        let me = &self.cars[PLAYER];
        let ti = self.track.idx(me.dist);
        let pct = me.dist / len;
        let accel = (me.speed - me.prev_speed) / DT;
        t.speed = me.speed as f32;
        t.gear = gear(me.speed);
        t.rpm = rpm(me.speed) as f32;
        let a = self.accel_smooth as f32;
        if a < -2.0 {
            t.brake = (-a / 26.0).clamp(0.0, 1.0);
            t.throttle = 0.0;
        } else {
            t.brake = 0.0;
            t.throttle = (0.55 + a * 0.25).clamp(0.25, 1.0);
        }
        if me.speed < 1.0 {
            t.throttle = 0.0;
        }
        t.clutch = if me.speed < 3.0 && matches!(me.pit, Pit::Stopped(_)) { 1.0 } else { 0.0 };
        t.steering = (-(self.track.curv[ti] * 2.7).atan() * 14.0) as f32;
        t.steering_max = 7.85;
        t.abs_active = t.brake > 0.95 && me.speed > 30.0;
        t.fuel_level = me.fuel as f32;
        t.fuel_level_pct = (me.fuel / 63.6) as f32;
        t.fuel_use_per_hour = (me.speed * 2.9) as f32;
        t.lap = me.lap;
        t.lap_completed = (me.lap - 1).max(0);
        t.lap_dist_pct = pct as f32;
        t.lap_dist = me.dist as f32;
        t.lap_current_time = (self.time - me.lap_start) as f32;
        t.lap_last_time = me.last_lap as f32;
        t.lap_best_time = me.best_lap as f32;
        t.delta_best_ok = me.best_lap > 0.0;
        t.delta_best = if me.best_lap > 0.0 { ((self.time - me.lap_start) - me.best_lap * self.track.est_time(pct) / self.track.lap_ref) as f32 } else { 0.0 };
        t.on_pit_road = t.cars[PLAYER].on_pit_road;
        // Pit lane: the limiter goes on just before the line and off a moment after the exit.
        if self.player_was_on_pit.get() && !t.on_pit_road {
            self.limiter_until.set(self.time + 1.5);
        }
        self.player_was_on_pit.set(t.on_pit_road);
        let limiter =
            t.on_pit_road || self.time < self.limiter_until.get() || matches!(me.pit, Pit::Requested) && (PIT_ENTRY - 0.003..PIT_ENTRY).contains(&pct);
        t.engine_warnings = if limiter { 0x10 } else { 0 };
        t.in_pit_stall = matches!(me.pit, Pit::Stopped(_));
        t.pitstop_active = matches!(me.pit, Pit::Stopped(until) if self.time < until - 0.5);
        t.pit_sv_status = match me.pit {
            Pit::Stopped(_) if t.pitstop_active => 1,
            Pit::Stopped(_) => 2,
            _ => 0,
        };
        t.is_on_track = true;
        t.incidents = self.incidents;
        // iRacing's YawNorth is a compass heading (0 = north, clockwise); the circuit uses maths angles.
        t.yaw_north = (std::f64::consts::FRAC_PI_2 - self.track.heading[ti]).rem_euclid(std::f64::consts::TAU) as f32;
        t.lat_accel = (me.speed * me.speed * self.track.curv[ti]) as f32;
        t.long_accel = accel as f32;
        t.tire_compound = me.wets as i32;
        // Spotter: cars overlapping the player sit on a side picked by their index.
        let (mut left, mut right) = (0, 0);
        for (i, c) in self.cars.iter().enumerate().skip(1) {
            let d = (c.dist - me.dist + len * 1.5).rem_euclid(len) - len * 0.5;
            if i != PLAYER && d.abs() < 4.8 && t.cars[i].on_pit_road == t.on_pit_road {
                if i % 2 == 0 {
                    left += 1;
                } else {
                    right += 1;
                }
            }
        }
        t.car_left_right = match (left, right) {
            (0, 0) => 1,
            (1, 0) => 2,
            (0, 1) => 3,
            (0, _) => 6,
            (_, 0) => 5,
            _ => 4,
        };
    }
}

impl Default for Mock {
    fn default() -> Self {
        Self::new()
    }
}

fn abbreviate(name: &str) -> String {
    let mut parts = name.split_whitespace();
    let first = parts.next().unwrap_or("");
    let last = parts.last().unwrap_or("");
    format!("{last}, {}", first.chars().next().unwrap_or(' '))
}

fn gear(speed: f64) -> i32 {
    [0.0, 18.0, 30.0, 41.0, 52.0, 63.0, 74.0].iter().filter(|&&s| speed >= s).count().max(1) as i32
}

fn rpm(speed: f64) -> f64 {
    let bands = [0.0, 18.0, 30.0, 41.0, 52.0, 63.0, 74.0, 95.0];
    let g = gear(speed) as usize;
    let (lo, hi) = (bands[g - 1], bands[g]);
    3500.0 + (speed - lo) / (hi - lo) * 4800.0
}

impl Source for Mock {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn poll(&mut self, tel: &mut Telemetry) -> Poll {
        if self.time >= self.fast_forward_until {
            let due = self.started + Duration::from_secs_f64((self.time - self.fast_forward_until) / 1.0);
            let now = Instant::now();
            if due > now {
                std::thread::sleep(due - now);
            }
        } else if self.time + DT >= self.fast_forward_until {
            self.started = Instant::now();
        }
        self.step();
        self.fill(tel);
        // Like iRacing, re-send the session string when results/incidents change.
        let session_yaml = (!self.sent_yaml || self.tick % (60 * 30) == 0).then(|| {
            self.sent_yaml = true;
            self.yaml()
        });
        Poll::Data { session_yaml }
    }

    fn track_map(&self, track_id: i64) -> Option<TrackMap> {
        Some(self.track.map(track_id))
    }

    fn pit_lane(&self) -> Option<(f32, f32)> {
        Some((PIT_ENTRY as f32, PIT_EXIT as f32))
    }

    fn corners(&self) -> Option<Vec<crate::config::Corner>> {
        Some(self.track.corners())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::model::SessionInfo;

    #[test]
    fn circuit_is_plausible() {
        let c = Circuit::new();
        assert!((c.len - TARGET_LEN).abs() < 1.0);
        assert!(c.lap_ref > 60.0 && c.lap_ref < 160.0, "{}", c.lap_ref);
        let corners = c.corners();
        assert!((5..=20).contains(&corners.len()), "{}", corners.len());
    }

    #[test]
    fn mock_race_feeds_the_engine() {
        let mut m = Mock::new();
        m.fast_forward_until = 1e9;
        let mut e = Engine::new(crate::config::Config::default(), None);
        let mut tel = Telemetry::default();
        for _ in 0..(60 * 600) {
            if let Poll::Data { session_yaml: Some(y) } = m.poll(&mut tel) {
                e.set_session(SessionInfo::parse(&y));
            }
            e.tel.clone_from(&tel);
            e.tick();
        }
        assert_eq!(e.info.drivers.len(), 20);
        assert_eq!(e.order.len(), 19);
        assert_eq!(e.classes.len(), 3);
        assert!(e.fuel.average(5).is_some(), "fuel samples");
        let f = crate::topics::fuel_out(&e);
        assert!(f.laps_remaining.unwrap() > 1.0);
        let b = crate::topics::battle_out(&e);
        assert!(b.overall.ahead.is_some() || b.overall.behind.is_some());
        assert!(b.track.ahead.is_some() && b.track.behind.is_some());
        // Ten minutes in, it is raining (weather cycle) and the advisor reacts.
        assert!(e.tel.track_wetness >= 3, "{}", e.tel.track_wetness);
    }
}
