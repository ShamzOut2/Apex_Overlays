//! Le Mans Ultimate, read from the game's built-in shared memory (`LMU_Data`).
//!
//! The game publishes scoring (timing and standings, about 5 times a second) and telemetry
//! (physics, up to 100 times a second) for every car. This source turns them into the same
//! [`Telemetry`] rows and iRacing-style session info the rest of the app is built on, so every
//! overlay works unchanged. Things LMU does not have (iRating, licences, incidents, car
//! numbers as a separate field, start lights) are left empty or derived.
//!
//! The player enables it in LMU under Settings → Gameplay ("Enable plugins" / shared memory);
//! no plugin needs installing.

pub mod data;
pub mod extras;
pub mod map;

use self::data::{text, ScoringInfo, VehicleScoring, VehicleTelemetry};
use crate::model::{flags, surface, CarSetting, CarTel, Telemetry, WheelTel, MAX_CARS};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Track ids for LMU tracks start here, well clear of iRacing's, so nothing looks an LMU
/// track up in iRacing's track lists (official maps, track data) by mistake.
pub const TRACK_ID_BASE: i64 = 1_000_000_000;

/// Whether a track id is one of iRacing's (as opposed to another game's).
pub fn is_iracing_track(id: i64) -> bool {
    id > 0 && id < TRACK_ID_BASE
}

/// One consistent copy of what the game published.
#[derive(Clone)]
pub struct Snapshot {
    pub info: ScoringInfo,
    pub vehicles: Vec<VehicleScoring>,
    pub telemetry: Vec<VehicleTelemetry>,
}

impl Snapshot {
    /// The game is in a session with cars (not in the menus).
    pub fn in_session(&self) -> bool {
        !self.vehicles.is_empty() && !text(&self.info.mTrackName).is_empty()
    }
}

/// What carries over from one snapshot to the next.
#[derive(Default)]
pub struct State {
    /// Game slot id → our car index (stable while the session lasts).
    slots: HashMap<i32, usize>,
    session_key: String,
    last_yaml: String,
    /// Scoring time the session info was last built for: it only changes with scoring (5 a
    /// second), so it is not rebuilt for every telemetry row (60 a second).
    yaml_et: Option<f64>,
    /// The player's last world position (x, z) and compass heading (radians).
    last_pos: Option<(f64, f64)>,
    heading: f32,
    last_phase: u8,
    /// When the green flag fell after the start countdown, for the "go" of the start lights.
    green_at: Option<f64>,
    tick: i32,
    sectors: Sectors,
}

/// Where sectors 2 and 3 start (lap fractions), learned from cars changing sector: LMU does
/// not publish the boundaries. Settled once each has been seen a few times.
#[derive(Default)]
struct Sectors {
    last: HashMap<i32, i8>,
    s2: Vec<f32>,
    s3: Vec<f32>,
    last_et: f64,
    settled: Option<[f32; 2]>,
}

impl Sectors {
    const SEEN: usize = 5;

    fn feed(&mut self, snap: &Snapshot) {
        let len = snap.info.mLapDist;
        if self.settled.is_some() || len < 100.0 || snap.info.mCurrentET == self.last_et {
            return;
        }
        self.last_et = snap.info.mCurrentET;
        for v in &snap.vehicles {
            let before = self.last.insert(v.mID, v.mSector);
            if v.mInPits != 0 || v.mInGarageStall != 0 {
                continue;
            }
            let pct = (v.mLapDist / len) as f32;
            // LMU numbers sectors 1, 2, then 0 for the third.
            match (before, v.mSector) {
                (Some(1), 2) => self.s2.push(pct),
                (Some(2), 0) => self.s3.push(pct),
                _ => {}
            }
        }
        if self.s2.len() >= Self::SEEN && self.s3.len() >= Self::SEEN {
            let median = |v: &mut Vec<f32>| {
                v.sort_by(f32::total_cmp);
                v[v.len() / 2]
            };
            let (a, b) = (median(&mut self.s2), median(&mut self.s3));
            if 0.0 < a && a < b && b < 1.0 {
                self.settled = Some([a, b]);
            }
        }
    }
}

/// Session type for iRacing-style session info: (SessionType, SessionName, EventType).
fn session_kind(session: i32) -> (&'static str, &'static str, &'static str) {
    match session {
        10..=13 => ("Race", "RACE", "Race"),
        9 => ("Warmup", "WARMUP", "Race"),
        // Open or private qualifying: the engine tells them apart by how the cars move.
        5..=8 => ("Qualify", "QUALIFY", "Race"),
        1..=4 => ("Practice", "PRACTICE", "Practice"),
        _ => ("Offline Testing", "TEST DAY", "Test"),
    }
}

/// LMU's track grip level as iRacing's rubber state words.
fn rubber(grip: u8) -> &'static str {
    match grip {
        0 => "clean",
        1 => "low usage",
        2 => "moderate usage",
        3 => "high usage",
        _ => "max usage",
    }
}

/// WEC-style class colours, so classes look as they do on TV.
fn class_color(name: &str, index: usize) -> u32 {
    let n = name.to_lowercase();
    if n.contains("hyper") {
        0xe10600
    } else if n.contains("lmp2") {
        0x1f6bff
    } else if n.contains("lmp3") {
        0x9b3dff
    } else if n.contains("gte") {
        0xff8c00
    } else if n.contains("gt3") {
        0x10b04a
    } else {
        [0x00b5d8, 0xffc400, 0xff3d8b, 0x8bc34a][index % 4]
    }
}

/// A small stable number for a name (FNV-1a), for ids LMU does not give.
fn stable_id(s: &str, modulo: u64) -> i64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.to_lowercase().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    (h % modulo) as i64
}

/// The track's id in this app (see [`TRACK_ID_BASE`]).
pub fn track_id(track: &str) -> i64 {
    TRACK_ID_BASE + stable_id(track, 100_000_000)
}

/// "#7", "#007" in a vehicle name ("Toyota Gazoo Racing 2025 #7:LM"): the car number.
fn car_number(vehicle: &str) -> Option<String> {
    let i = vehicle.find('#')?;
    let digits: String = vehicle[i + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
    (!digits.is_empty()).then_some(digits)
}

/// The team from a vehicle name: the name without its "#7:LM" number part.
fn team_name(vehicle: &str) -> String {
    let base = vehicle.split('#').next().unwrap_or(vehicle).trim();
    if base.is_empty() {
        vehicle.trim().to_string()
    } else {
        base.to_string()
    }
}

/// "Lastname, F" as iRacing abbreviates names.
fn abbrev(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    match parts.as_slice() {
        [] => String::new(),
        [one] => one.to_string(),
        [first, .., last] => format!("{last}, {}", first.chars().next().unwrap_or(' ')),
    }
}

/// One line of iRacing-style session info: no line breaks inside a value.
fn clean(s: &str) -> String {
    s.replace(['\r', '\n'], " ")
}

/// LMU's average wetness of the racing line (0-1) as iRacing's TrackWetness (1 dry ... 7 extremely wet).
fn wetness(w: f64) -> i32 {
    match w {
        w if w <= 0.01 => 1,
        w if w < 0.1 => 2,
        w if w < 0.2 => 3,
        w if w < 0.35 => 4,
        w if w < 0.5 => 5,
        w if w < 0.75 => 6,
        _ => 7,
    }
}

/// Fills `tel` from a snapshot and returns iRacing-style session info when it changed.
pub fn apply(st: &mut State, snap: &Snapshot, tel: &mut Telemetry) -> Option<String> {
    let info = &snap.info;
    let track = text(&info.mTrackName);
    let track_m = info.mLapDist.max(0.0);
    // A new track or session: car indexes start afresh.
    let key = format!("{track}|{}", info.mSession);
    if key != st.session_key {
        st.session_key = key;
        st.slots.clear();
        st.last_pos = None;
        st.green_at = None;
        st.yaml_et = None;
    }
    for v in &snap.vehicles {
        if !st.slots.contains_key(&v.mID) && st.slots.len() < MAX_CARS {
            let idx = (0..MAX_CARS).find(|i| !st.slots.values().any(|x| x == i)).unwrap_or(0);
            st.slots.insert(v.mID, idx);
        }
    }
    let idx_of = |v: &VehicleScoring| st.slots.get(&v.mID).copied();
    let telem_of = |id: i32| snap.telemetry.iter().find(|t| t.mID == id);
    let player = snap.vehicles.iter().find(|v| v.mIsPlayer != 0);
    let pt = player.and_then(|p| telem_of(p.mID));
    // Telemetry is newer than scoring: carry each car forward along the lap by its speed.
    let now = pt.or(snap.telemetry.first()).map_or(info.mCurrentET, |t| t.mElapsedTime);
    let ahead = (now - info.mCurrentET).clamp(0.0, 0.5);
    let speed_of = |t: &VehicleTelemetry| {
        let v = t.mLocalVel;
        (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
    };

    // Class positions: by overall place within each class.
    let class_of = |v: &VehicleScoring| text(&v.mVehicleClass);
    let mut by_place: Vec<&VehicleScoring> = snap.vehicles.iter().filter(|v| v.mPlace > 0).collect();
    by_place.sort_by_key(|v| v.mPlace);
    let mut class_pos: HashMap<i32, i32> = HashMap::new();
    let mut counts: HashMap<String, i32> = HashMap::new();
    for v in &by_place {
        let n = counts.entry(class_of(v)).or_default();
        *n += 1;
        class_pos.insert(v.mID, *n);
    }

    st.sectors.feed(snap);
    let phase = if info.mGamePhase == 9 { st.last_phase } else { info.mGamePhase };
    if phase == 5 && matches!(st.last_phase, 3 | 4) {
        st.green_at = Some(info.mCurrentET);
    }
    st.last_phase = phase;

    // Cars.
    tel.cars.iter_mut().for_each(|c| *c = CarTel { surface: surface::NOT_IN_WORLD, pct: -1.0, lap: -1, lap_completed: -1, position: 0, ..Default::default() });
    for v in &snap.vehicles {
        let Some(i) = idx_of(v) else { continue };
        let t = telem_of(v.mID);
        let mut dist = v.mLapDist;
        if let Some(t) = t {
            dist += speed_of(t) * ahead;
        }
        // Carried over the line before the game counts the lap: the next lap already.
        let crossed = track_m > 1.0 && v.mLapDist < track_m && dist >= track_m;
        let pct = if track_m > 1.0 { (dist / track_m).rem_euclid(1.0) as f32 } else { 0.0 };
        let off = v.mTrackEdge > 0.0 && v.mPathLateral.abs() > v.mTrackEdge.abs() + 2.0;
        let surf = if v.mInGarageStall != 0 {
            surface::NOT_IN_WORLD
        } else if v.mPitState == 3 {
            surface::IN_PIT_STALL
        } else if v.mPitState == 2 && v.mInPits == 0 {
            surface::APPROACHING_PITS
        } else if off && v.mInPits == 0 {
            surface::OFF_TRACK
        } else {
            surface::ON_TRACK
        };
        let mut car_flags = 0;
        if v.mFlag == 6 {
            car_flags |= flags::BLUE;
        }
        if v.mNumPenalties > 0 {
            car_flags |= flags::BLACK;
        }
        tel.cars[i] = CarTel {
            lap: v.mTotalLaps as i32 + 1 + crossed as i32,
            lap_completed: v.mTotalLaps as i32,
            pct: if surf == surface::NOT_IN_WORLD { -1.0 } else { pct },
            // iRacing's CarIdxEstTime is the time a reference lap takes to reach the car's spot;
            // LMU has no such figure (mEstimatedLapTime is a whole lap), so gaps are reckoned
            // from the distance into the lap instead.
            est_time: 0.0,
            f2_time: v.mTimeBehindLeader as f32,
            position: v.mPlace as i32,
            class_position: class_pos.get(&v.mID).copied().unwrap_or(0),
            on_pit_road: v.mInPits != 0,
            surface: surf,
            last_lap: if v.mLastLapTime > 0.0 { v.mLastLapTime as f32 } else { -1.0 },
            best_lap: if v.mBestLapTime > 0.0 { v.mBestLapTime as f32 } else { -1.0 },
            tire_compound: t.map_or(0, |t| (t.mWheels[0].mCompoundType == 3) as i32),
            gear: t.map_or(0, |t| t.mGear),
            rpm: t.map_or(0.0, |t| t.mEngineRPM as f32),
            session_flags: car_flags,
            fuel: Some(v.mFuelFraction as f32 / 255.0),
            // 0..1 (taken as % when larger); none for cars on fuel, or when the game leaves
            // other cars' energy out.
            energy: t.map(|t| t.mVirtualEnergy).filter(|&v| v > 0.0).map(|v| if v > 1.0 { v.min(100.0) } else { v * 100.0 }),
            pos: t.map(|t| [t.mPos.x as f32, t.mPos.z as f32]),
        };
    }

    // Session.
    st.tick = st.tick.wrapping_add(1);
    tel.tick = st.tick;
    tel.session_time = now;
    tel.session_num = info.mSession;
    tel.session_state = match phase {
        0 => 1,
        1 | 2 => 2,
        3 | 4 => 3,
        5..=7 => 4,
        8 => 5,
        _ => tel.session_state,
    };
    // LMU races start rolling, with no start lights: hidden lights, as in iRacing's rolling starts.
    let mut sflags = flags::START_HIDDEN;
    match phase {
        5 => sflags |= flags::GREEN,
        6 => sflags |= flags::CAUTION | flags::YELLOW,
        7 => sflags |= flags::RED,
        8 => sflags |= flags::CHECKERED,
        _ => {}
    }
    if phase == 5 && st.green_at.is_some_and(|g| info.mCurrentET - g < 5.0) {
        sflags |= flags::GREEN_HELD;
    }
    // A local yellow in the player's sector (LMU numbers sectors 1, 2, 0).
    if let Some(p) = player {
        let sector = match p.mSector {
            1 => 0,
            2 => 1,
            _ => 2,
        };
        if info.mSectorFlag.get(sector).is_some_and(|&f| f == 1) {
            sflags |= flags::YELLOW;
        }
        if p.mFlag == 6 {
            sflags |= flags::BLUE;
        }
        if p.mNumPenalties > 0 {
            sflags |= flags::BLACK;
        }
    }
    tel.session_flags = sflags;
    tel.session_time_remain = if info.mEndET > 0.0 { (info.mEndET - info.mCurrentET).max(0.0) } else { 0.0 };
    let leader_laps = snap.vehicles.iter().map(|v| v.mTotalLaps as i32).max().unwrap_or(0);
    tel.session_laps_remain = if info.mMaxLaps > 0 && info.mMaxLaps < 10_000 { (info.mMaxLaps - leader_laps).max(0) } else { 32_767 };
    tel.time_of_day = info.mTimeOfDay;
    tel.player_idx = player.and_then(idx_of).map_or(-1, |i| i as i32);
    // Spectating: watch the leader (LMU does not say which car the camera follows).
    tel.cam_car_idx = player.or_else(|| by_place.first().copied()).and_then(idx_of).map_or(-1, |i| i as i32);
    tel.is_replay_playing = false;

    // Weather.
    tel.track_temp = info.mTrackTemp as f32;
    tel.air_temp = info.mAmbientTemp as f32;
    tel.track_wetness = wetness(info.mAvgPathWetness);
    tel.precipitation = info.mRaining.clamp(0.0, 1.0) as f32;
    tel.declared_wet = info.mRaining > 0.1 || info.mAvgPathWetness > 0.2;
    let w = info.mWind;
    tel.wind_vel = (w.x * w.x + w.z * w.z).sqrt() as f32;
    tel.wind_dir = (w.x.atan2(w.z) as f32).rem_euclid(std::f32::consts::TAU);
    tel.sky_code = info.mCloudCoverage.min(10) as i32;
    tel.skies = match info.mCloudCoverage {
        0 => 0,
        1 | 2 => 1,
        3 => 2,
        _ => 3,
    };
    tel.display_metric = true;

    // The player's car.
    match (player, pt) {
        (Some(p), Some(t)) => {
            let speed = speed_of(t);
            tel.speed = speed as f32;
            tel.rpm = t.mEngineRPM as f32;
            tel.gear = t.mGear;
            tel.throttle = t.mUnfilteredThrottle.clamp(0.0, 1.0) as f32;
            tel.brake = t.mUnfilteredBrake.clamp(0.0, 1.0) as f32;
            tel.clutch = t.mUnfilteredClutch.clamp(0.0, 1.0) as f32;
            // LMU: -1 (full left) to 1 (full right) of the wheel's range. iRacing: radians, left positive.
            let range = if t.mPhysicalSteeringWheelRange > 0.0 { t.mPhysicalSteeringWheelRange } else { t.mVisualSteeringWheelRange.max(0.0) };
            let half = (range / 2.0).to_radians();
            tel.steering = -(t.mUnfilteredSteering as f32) * half;
            tel.steering_max = half;
            tel.abs_active = t.mABSActive != 0;
            tel.fuel_level = t.mFuel as f32;
            tel.fuel_level_pct = if t.mFuelCapacity > 0.0 { (t.mFuel / t.mFuelCapacity) as f32 } else { 0.0 };
            // Virtual energy (Hypercar, LMGT3...): a share of the allowance, sent as 0..1 (taken
            // as % when larger). None for cars that do not race on it.
            let pct = |v: f64| if v > 1.0 { v.min(100.0) as f32 } else { (v * 100.0) as f32 };
            tel.virtual_energy = (t.mVirtualEnergy > 0.0).then(|| pct(t.mVirtualEnergy as f64));
            tel.battery = (t.mBatteryChargeFraction > 0.0).then(|| pct(t.mBatteryChargeFraction));
            tel.lap_invalid = t.mLapInvalidated != 0;
            car_settings(t, &mut tel.car_settings);
            tel.wheels = Some(wheels(t));
            tel.tyres_live = true;
            let opt = t.mWheels[0].mOptimalTemp;
            tel.tyre_optimal = (opt > 0.0).then_some(if opt > 200.0 { opt - 273.15 } else { opt });
            let me = &tel.cars[tel.player_idx.max(0) as usize];
            tel.lap = me.lap;
            tel.lap_completed = me.lap_completed;
            tel.lap_dist_pct = me.pct.max(0.0);
            tel.lap_dist = me.pct.max(0.0) * track_m as f32;
            tel.lap_current_time = (t.mElapsedTime - t.mLapStartET).max(0.0) as f32;
            tel.lap_last_time = if p.mLastLapTime > 0.0 { p.mLastLapTime as f32 } else { -1.0 };
            tel.lap_best_time = if p.mBestLapTime > 0.0 { p.mBestLapTime as f32 } else { -1.0 };
            tel.delta_best = t.mDeltaBest as f32;
            tel.delta_best_ok = p.mBestLapTime > 0.0;
            tel.on_pit_road = p.mInPits != 0;
            tel.is_on_track = p.mInGarageStall == 0;
            tel.in_pit_stall = p.mPitState == 3;
            tel.pitstop_active = p.mPitState == 3;
            tel.engine_warnings = if t.mSpeedLimiterActive != 0 { 0x10 } else { 0 };
            tel.tire_compound = (t.mWheels[0].mCompoundType == 3) as i32;
            // Local axes: +x left, +y up, +z back. Forward acceleration is -z.
            tel.long_accel = -(t.mLocalAccel.z as f32);
            tel.lat_accel = t.mLocalAccel.x as f32;
            // Compass heading from how the car moves across the world (x east, z north),
            // which is all the recorded track outline needs.
            let (x, z) = (t.mPos.x, t.mPos.z);
            if let Some((lx, lz)) = st.last_pos {
                let (dx, dz) = (x - lx, z - lz);
                if dx * dx + dz * dz > 0.04 {
                    st.heading = (dx.atan2(dz) as f32).rem_euclid(std::f32::consts::TAU);
                    st.last_pos = Some((x, z));
                }
            } else {
                st.last_pos = Some((x, z));
            }
            tel.yaw_north = st.heading;
            tel.car_left_right = spotter(snap, p, t, st.heading);
        }
        _ => {
            tel.is_on_track = false;
            tel.speed = 0.0;
            tel.car_left_right = 0;
            tel.virtual_energy = None;
            tel.battery = None;
            tel.lap_invalid = false;
            tel.car_settings.clear();
            tel.wheels = None;
            tel.tyre_optimal = None;
        }
    }

    if st.yaml_et == Some(info.mCurrentET) {
        return None;
    }
    st.yaml_et = Some(info.mCurrentET);
    let yaml = session_yaml(snap, st, &track, track_m, player, pt, &class_pos);
    if yaml != st.last_yaml {
        st.last_yaml = yaml.clone();
        return Some(yaml);
    }
    None
}

/// The settings changed from the wheel that the car has (a maximum of 0: the car has none).
fn car_settings(t: &VehicleTelemetry, out: &mut Vec<CarSetting>) {
    out.clear();
    // The share of braking at the rear: shown as the front's %, as on the dash.
    if t.mRearBrakeBias > 0.0 && t.mRearBrakeBias < 1.0 {
        out.push(CarSetting { id: "brakeBias", value: ((1.0 - t.mRearBrakeBias) * 1000.0).round() as f32 / 10.0, max: None });
    }
    for (id, v, max) in [
        ("abs", t.mABS, t.mABSMax),
        ("tc", t.mTC, t.mTCMax),
        ("tcSlip", t.mTCSlip, t.mTCSlipMax),
        ("tcCut", t.mTCCut, t.mTCCutMax),
        ("engineMap", t.mMotorMap, t.mMotorMapMax),
        ("migration", t.mMigration, t.mMigrationMax),
        ("arbFront", t.mFrontAntiSway, t.mFrontAntiSwayMax),
        ("arbRear", t.mRearAntiSway, t.mRearAntiSwayMax),
    ] {
        if max > 0 {
            out.push(CarSetting { id, value: v as f32, max: Some(max as f32) });
        }
    }
}

/// Tyres and brakes, live: the game sends temperatures in kelvin and tread left as 0..1.
fn wheels(t: &VehicleTelemetry) -> [WheelTel; 4] {
    let c = |k: f64| (k > 1.0).then_some((k - 273.15) as f32);
    let w = t.mWheels;
    [0, 1, 2, 3].map(|i| {
        let wh = w[i];
        let temps = wh.mTemperature;
        WheelTel {
            temps: (temps[0] > 1.0).then(|| temps.map(|k| (k - 273.15) as f32)),
            carcass: c(wh.mTireCarcassTemperature),
            pressure: (wh.mPressure > 0.0).then_some(wh.mPressure as f32),
            wear: Some((wh.mWear.clamp(0.0, 1.0) * 100.0) as f32),
            brake: c(wh.mBrakeTemp),
            flat: wh.mFlat != 0,
        }
    })
}

/// Cars beside the player, as iRacing's CarLeftRight (1 clear, 2 car left, 3 car right,
/// 4 both sides, 5 two left, 6 two right), from the cars' world positions.
fn spotter(snap: &Snapshot, p: &VehicleScoring, pt: &VehicleTelemetry, heading: f32) -> i32 {
    if p.mInPits != 0 {
        return 0;
    }
    let (s, c) = (heading as f64).sin_cos();
    let (mut left, mut right) = (0, 0);
    for t in &snap.telemetry {
        if t.mID == pt.mID {
            continue;
        }
        let (dx, dz) = (t.mPos.x - pt.mPos.x, t.mPos.z - pt.mPos.z);
        // Along the car (forward is the heading) and across it (right of the heading).
        let along = dx * s + dz * c;
        let across = dx * c - dz * s;
        if along.abs() < 4.8 && (0.5..6.0).contains(&across.abs()) && (t.mPos.y - pt.mPos.y).abs() < 3.0 {
            if across > 0.0 {
                right += 1;
            } else {
                left += 1;
            }
        }
    }
    match (left, right) {
        (0, 0) => 1,
        (l, r) if l > 0 && r > 0 => 4,
        (1, 0) => 2,
        (_, 0) => 5,
        (0, 1) => 3,
        _ => 6,
    }
}

/// The session as iRacing's session info text, which the engine already understands.
fn session_yaml(
    snap: &Snapshot,
    st: &State,
    track: &str,
    track_m: f64,
    player: Option<&VehicleScoring>,
    pt: Option<&VehicleTelemetry>,
    class_pos: &HashMap<i32, i32>,
) -> String {
    let info = &snap.info;
    let (kind, name, event) = session_kind(info.mSession);
    let mut y = String::with_capacity(16 * 1024);
    let track = clean(track);
    let _ = write!(
        y,
        "---\nWeekendInfo:\n TrackName: {track}\n TrackID: {}\n TrackLength: {:.2} km\n TrackDisplayName: {track}\n TrackConfigName: \n SeriesID: 0\n SubSessionID: 0\n LeagueID: 0\n Official: 0\n EventType: {event}\n WeekendOptions:\n  StandingStart: 0\n  IncidentLimit: unlimited\n",
        track_id(&track),
        track_m / 1000.0
    );
    let laps = if info.mMaxLaps > 0 && info.mMaxLaps < 10_000 { info.mMaxLaps.to_string() } else { "unlimited".into() };
    let end = info.mEndET;
    let time = if end > 0.0 { format!("{end:.4} sec") } else { "unlimited".into() };
    let _ = write!(
        y,
        "SessionInfo:\n Sessions:\n - SessionNum: {}\n   SessionLaps: {laps}\n   SessionTime: {time}\n   SessionType: {kind}\n   SessionName: {name}\n   SessionTrackRubberState: {}\n   ResultsPositions:\n",
        info.mSession,
        rubber(info.mTrackGripLevel)
    );
    let mut placed: Vec<&VehicleScoring> = snap.vehicles.iter().filter(|v| v.mPlace > 0).collect();
    placed.sort_by_key(|v| v.mPlace);
    for v in placed {
        let Some(&i) = st.slots.get(&v.mID) else { continue };
        let reason = match v.mFinishStatus {
            2 => "DNF",
            3 => "Disqualified",
            _ => "Running",
        };
        let _ = write!(
            y,
            "   - Position: {}\n     ClassPosition: {}\n     CarIdx: {i}\n     LapsComplete: {}\n     FastestTime: {:.4}\n     LastTime: {:.4}\n     Incidents: 0\n     ReasonOutStr: {reason}\n",
            v.mPlace,
            class_pos.get(&v.mID).copied().unwrap_or(1) - 1,
            v.mTotalLaps,
            if v.mBestLapTime > 0.0 { v.mBestLapTime } else { -1.0 },
            if v.mLastLapTime > 0.0 { v.mLastLapTime } else { -1.0 },
        );
    }
    // Classes in order of appearance, for their colour and id.
    let mut classes: Vec<String> = vec![];
    for v in &snap.vehicles {
        let c = text(&v.mVehicleClass);
        if !classes.contains(&c) {
            classes.push(c);
        }
    }
    let player_idx = player.and_then(|p| st.slots.get(&p.mID)).map_or(-1, |&i| i as i32);
    let max_rpm = pt.map_or(0.0, |t| t.mEngineMaxRPM);
    let pace = snap.vehicles.iter().find(|v| is_pace_car(v)).and_then(|v| st.slots.get(&v.mID)).map_or(-1, |&i| i as i32);
    let _ = write!(
        y,
        "DriverInfo:\n DriverCarIdx: {player_idx}\n PaceCarIdx: {pace}\n DriverCarFuelMaxLtr: {:.3}\n DriverCarMaxFuelPct: 1.000\n DriverCarRedLine: {max_rpm:.1}\n DriverCarSLFirstRPM: {:.1}\n DriverCarSLShiftRPM: {:.1}\n DriverCarSLLastRPM: {:.1}\n DriverCarSLBlinkRPM: {:.1}\n DriverCarEstLapTime: {:.0}\n DriverTires:\n - TireIndex: 0\n   TireCompoundType: \"Dry\"\n - TireIndex: 1\n   TireCompoundType: \"Wet\"\n Drivers:\n",
        pt.map_or(0.0, |t| t.mFuelCapacity),
        max_rpm * 0.82,
        max_rpm * 0.96,
        max_rpm * 0.97,
        max_rpm * 0.985,
        player.map_or(0.0, |p| if p.mEstimatedLapTime > 0.0 { p.mEstimatedLapTime } else { class_est_lap(snap, &p.mVehicleClass) }),
    );
    let mut cars: Vec<&VehicleScoring> = snap.vehicles.iter().filter(|v| st.slots.contains_key(&v.mID)).collect();
    cars.sort_by_key(|v| st.slots[&v.mID]);
    for v in cars {
        let i = st.slots[&v.mID];
        let driver = clean(&text(&v.mDriverName));
        let vehicle = clean(&text(&v.mVehicleName));
        let class = clean(&text(&v.mVehicleClass));
        let ci = classes.iter().position(|c| *c == class).unwrap_or(0);
        let t = snap.telemetry.iter().find(|t| t.mID == v.mID);
        let model = t.map(|t| clean(&text(&t.mVehicleModel))).filter(|m| !m.is_empty()).unwrap_or_else(|| team_name(&vehicle));
        let pit_group = clean(&text(&v.mPitGroup));
        let team = if pit_group.is_empty() { team_name(&vehicle) } else { pit_group };
        let number = car_number(&vehicle).unwrap_or_default();
        let class_est = class_est_lap(snap, &v.mVehicleClass);
        let _ = write!(
            y,
            " - CarIdx: {i}\n   UserName: {driver}\n   AbbrevName: {}\n   Initials: \n   UserID: -1\n   TeamName: {team}\n   CarNumber: \"{number}\"\n   CarNumberRaw: {}\n   CarID: {}\n   CarPath: {}\n   CarScreenName: {model}\n   CarScreenNameShort: {model}\n   CarClassID: {}\n   CarClassShortName: {class}\n   CarClassColor: 0x{:06x}\n   CarClassEstLapTime: {class_est:.0}\n   IRating: 0\n   LicString: \n   LicColor: 0xffffff\n   CarIsAI: {}\n   IsSpectator: 0\n   CarIsPaceCar: {}\n",
            clean(&abbrev(&driver)),
            number.parse::<i32>().unwrap_or(-1),
            stable_id(&model, 1_000_000),
            clean(&text(&v.mVehFilename)),
            stable_id(&class, 1000) + 1,
            class_color(&class, ci),
            (v.mControl == 1) as i32,
            is_pace_car(v) as i32,
        );
    }
    // After the drivers (a top-level block). Sector boundaries, once learned (sector lines on the track map, sector times).
    if let Some([s2, s3]) = st.sectors.settled {
        let _ = write!(
            y,
            "SplitTimeInfo:\n Sectors:\n - SectorNum: 0\n   SectorStartPct: 0.000000\n - SectorNum: 1\n   SectorStartPct: {s2:.6}\n - SectorNum: 2\n   SectorStartPct: {s3:.6}\n"
        );
    }
    y
}

fn is_pace_car(v: &VehicleScoring) -> bool {
    let c = text(&v.mVehicleClass).to_lowercase();
    let n = text(&v.mVehicleName).to_lowercase();
    c.contains("pace") || c.contains("safety") || n.contains("safety car") || n.contains("pace car")
}

/// The game's shared memory, read live (Windows).
pub struct Lmu {
    #[cfg(windows)]
    map: Option<apex_irsdk::NamedMapping>,
    #[cfg(windows)]
    last_open: Option<std::time::Instant>,
    /// When the game's data last moved on; a long pause means the game may have closed.
    #[cfg(windows)]
    changed_at: std::time::Instant,
    #[cfg(windows)]
    last_et: f64,
    #[cfg(windows)]
    next_frame: std::time::Instant,
    /// When we last checked that the game is still running (while its data stands still).
    #[cfg(windows)]
    alive_checked: Option<std::time::Instant>,
    /// The game was found closed (its shared memory kept open by another program): not looked
    /// for again until then, so its last data is not taken for a running game.
    #[cfg(windows)]
    gone_until: Option<std::time::Instant>,
    #[cfg_attr(not(windows), allow(dead_code))]
    state: State,
    /// The track outline, built from the cars' positions.
    outline: map::MapBuilder,
}

impl Default for Lmu {
    fn default() -> Self {
        Self::new()
    }
}

impl Lmu {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            map: None,
            #[cfg(windows)]
            last_open: None,
            #[cfg(windows)]
            changed_at: std::time::Instant::now(),
            #[cfg(windows)]
            last_et: -1.0,
            #[cfg(windows)]
            next_frame: std::time::Instant::now(),
            #[cfg(windows)]
            alive_checked: None,
            #[cfg(windows)]
            gone_until: None,
            state: State::default(),
            outline: map::MapBuilder::default(),
        }
    }

    /// Whether the game's shared memory is there (the game is running with it enabled).
    /// Cheap enough to ask every second or so.
    pub fn available() -> bool {
        #[cfg(windows)]
        {
            apex_irsdk::NamedMapping::open(data::MAP_NAME).is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    /// The game is still running: its window (which it names in the shared memory) still
    /// exists, or, when it has not named one, its process is there. Other programs (SimHub,
    /// CrewChief...) holding the shared memory keep its last data alive after the game closes,
    /// so data standing still does not say whether the game is paused or gone.
    #[cfg(windows)]
    fn game_alive(map: &apex_irsdk::NamedMapping) -> bool {
        use self::data::{ApplicationState, Generic, Layout};
        use std::mem::offset_of;
        let at = offset_of!(Layout, generic) + offset_of!(Generic, appInfo) + offset_of!(ApplicationState, mAppWindow);
        // SAFETY: a u64 read from inside the checked view.
        let hwnd: u64 = unsafe { map.read(at) }.unwrap_or(0);
        if hwnd != 0 {
            return apex_irsdk::window_alive(hwnd);
        }
        let mut sys = sysinfo::System::new();
        sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::nothing());
        sys.processes().values().any(|p| p.name().to_string_lossy().to_lowercase().starts_with("le mans ultimate"))
    }

    /// One consistent copy of the scoring and telemetry, or `None` while the game is
    /// writing (car counts disagree) or the mapping is too small for this layout.
    #[cfg(windows)]
    fn snapshot(map: &apex_irsdk::NamedMapping) -> Option<Snapshot> {
        use self::data::{Layout, ScoringData, TelemetryData, MAX_VEHICLES};
        use std::mem::{offset_of, size_of};
        if map.len() < size_of::<Layout>() {
            return None;
        }
        let scoring = offset_of!(Layout, scoring);
        let telemetry = offset_of!(Layout, telemetry);
        // SAFETY: plain-old-data structs read from inside the checked view.
        unsafe {
            let info: ScoringInfo = map.read(scoring + offset_of!(ScoringData, scoringInfo))?;
            let active: u8 = map.read(telemetry + offset_of!(TelemetryData, activeVehicles))?;
            let n = (info.mNumVehicles.max(0) as usize).min(MAX_VEHICLES);
            let nt = (active as usize).min(MAX_VEHICLES);
            if n != nt {
                return None;
            }
            let veh_at = scoring + offset_of!(ScoringData, vehScoringInfo);
            let tel_at = telemetry + offset_of!(TelemetryData, telemInfo);
            let vehicles = (0..n).map(|i| map.read::<VehicleScoring>(veh_at + i * size_of::<VehicleScoring>())).collect::<Option<Vec<_>>>()?;
            let telemetry = (0..nt).map(|i| map.read::<VehicleTelemetry>(tel_at + i * size_of::<VehicleTelemetry>())).collect::<Option<Vec<_>>>()?;
            // The game may have moved on while we copied: try again next time if so.
            let after: ScoringInfo = map.read(scoring + offset_of!(ScoringData, scoringInfo))?;
            if after.mNumVehicles != info.mNumVehicles {
                return None;
            }
            Some(Snapshot { info: after, vehicles, telemetry })
        }
    }
}

impl super::Source for Lmu {
    fn name(&self) -> &'static str {
        "lmu"
    }

    fn take_track_map(&mut self) -> Option<crate::engine::trackmap::TrackMap> {
        self.outline.take()
    }

    fn track_map_progress(&self) -> Option<f32> {
        Some(self.outline.progress())
    }

    fn reset_track_map(&mut self) {
        self.outline.reset();
    }

    #[cfg(not(windows))]
    fn poll(&mut self, _tel: &mut Telemetry) -> super::Poll {
        std::thread::sleep(std::time::Duration::from_millis(500));
        super::Poll::Disconnected
    }

    #[cfg(windows)]
    fn poll(&mut self, tel: &mut Telemetry) -> super::Poll {
        use std::time::{Duration, Instant};
        // How long to leave a closed game before looking for it again.
        const GONE: Duration = Duration::from_secs(3);
        if self.gone_until.is_some_and(|t| Instant::now() < t) {
            std::thread::sleep(Duration::from_millis(250));
            return super::Poll::Disconnected;
        }
        if self.map.is_none() {
            // Try the mapping at most once a second.
            if self.last_open.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
                std::thread::sleep(Duration::from_millis(250));
                return super::Poll::Disconnected;
            }
            self.last_open = Some(Instant::now());
            self.map = apex_irsdk::NamedMapping::open(data::MAP_NAME);
            if self.map.is_none() {
                // Start afresh next time (the engine forgets the session on a disconnect).
                self.state = State::default();
                std::thread::sleep(Duration::from_millis(250));
                return super::Poll::Disconnected;
            }
            // The shared memory is there, but another program (SimHub, CrewChief...) may be
            // keeping it open after the game closed: only a running game counts.
            if !Self::game_alive(self.map.as_ref().unwrap()) {
                self.map = None;
                self.state = State::default();
                self.gone_until = Some(Instant::now() + GONE);
                std::thread::sleep(Duration::from_millis(250));
                return super::Poll::Disconnected;
            }
            self.gone_until = None;
            self.changed_at = Instant::now();
        }
        // 60 rows a second, like iRacing's.
        let now = Instant::now();
        if self.next_frame > now {
            std::thread::sleep(self.next_frame - now);
        }
        self.next_frame = Instant::now().max(self.next_frame) + Duration::from_micros(16_667);
        let Some(snap) = Self::snapshot(self.map.as_ref().unwrap()) else { return super::Poll::Idle };
        if !snap.in_session() {
            // In the menus: as if disconnected, and start afresh in the next session.
            self.state = State::default();
            std::thread::sleep(Duration::from_millis(200));
            return super::Poll::Disconnected;
        }
        let et = snap.telemetry.iter().map(|t| t.mElapsedTime).fold(snap.info.mCurrentET, f64::max);
        if et != self.last_et {
            self.last_et = et;
            self.changed_at = Instant::now();
            self.alive_checked = None;
        } else {
            // Nothing new for a while: the game is paused, or closed. Check once a second that
            // it is still running; when it has gone, disconnect (the overlays hide and, in
            // automatic mode, the other game is looked for).
            if self.changed_at.elapsed() > Duration::from_secs(2) && self.alive_checked.is_none_or(|t| t.elapsed() > Duration::from_secs(1)) {
                self.alive_checked = Some(Instant::now());
                if !Self::game_alive(self.map.as_ref().unwrap()) {
                    self.map = None;
                    self.last_et = -1.0;
                    self.state = State::default();
                    self.gone_until = Some(Instant::now() + GONE);
                    return super::Poll::Disconnected;
                }
            }
            return super::Poll::Idle;
        }
        let session_yaml = apply(&mut self.state, &snap, tel);
        let track = text(&snap.info.mTrackName);
        self.outline.feed(&snap, track_id(&track), &track);
        super::Poll::Data { session_yaml }
    }
}

/// A lap time for the class, to turn track distance into time: the game's estimate, or when it
/// has none yet the class's best lap so far (0 when nobody has set one).
fn class_est_lap(snap: &Snapshot, class: &[u8]) -> f64 {
    let same = || snap.vehicles.iter().filter(|o| o.mVehicleClass[..] == class[..]);
    same()
        .map(|o| o.mEstimatedLapTime)
        .find(|e| *e > 0.0)
        .unwrap_or_else(|| same().map(|o| o.mBestLapTime).filter(|b| *b > 0.0).fold(0.0, |a: f64, b| if a == 0.0 { b } else { a.min(b) }))
}

#[cfg(test)]
pub mod tests {
    use super::data::*;
    use super::*;
    use crate::model::SessionInfo;

    fn name<const N: usize>(s: &str) -> [u8; N] {
        let mut b = [0u8; N];
        b[..s.len().min(N - 1)].copy_from_slice(&s.as_bytes()[..s.len().min(N - 1)]);
        b
    }

    /// A race at a 13.6 km track: the player in a Hypercar, a car alongside on the right, and
    /// an LMP2 and a GT3 further back.
    pub fn snapshot() -> Snapshot {
        let mut info = ScoringInfo::zeroed();
        info.mTrackName = name("Circuit de la Sarthe");
        info.mSession = 10;
        info.mCurrentET = 600.0;
        info.mEndET = 3600.0;
        info.mMaxLaps = 2_147_483_647;
        info.mLapDist = 13_626.0;
        info.mGamePhase = 5;
        info.mTrackTemp = 31.0;
        info.mAmbientTemp = 22.0;
        info.mTrackGripLevel = 2;
        let car = |id: i32, driver: &str, vehicle: &str, class: &str, place: u8, dist: f64| {
            let mut v = VehicleScoring::zeroed();
            v.mID = id;
            v.mDriverName = name(driver);
            v.mVehicleName = name(vehicle);
            v.mVehicleClass = name(class);
            v.mPlace = place;
            v.mLapDist = dist;
            v.mTotalLaps = 3;
            v.mTrackEdge = 7.0;
            v.mEstimatedLapTime = 210.0;
            v.mBestLapTime = 208.5;
            v.mLastLapTime = 209.1;
            v
        };
        let mut me = car(11, "Ana Ribeiro", "Toyota Gazoo Racing 2025 #7:LM", "Hyper", 2, 5000.0);
        me.mIsPlayer = 1;
        let vehicles = vec![
            car(4, "Max Dunlop", "Ferrari AF Corse 2025 #51:LM", "Hyper", 1, 5003.0),
            me,
            car(20, "Lena Hartmann", "United Autosports 2025 #22:LM", "LMP2", 3, 2000.0),
            car(31, "Kenji Arai", "Manthey 2025 #92:LM", "GT3", 4, 1000.0),
        ];
        let tele = |id: i32, x: f64, z: f64| {
            let mut t = VehicleTelemetry::zeroed();
            t.mID = id;
            t.mElapsedTime = 600.1;
            t.mPos = Vect3 { x, y: 0.0, z };
            t.mLocalVel = Vect3 { x: 0.0, y: 0.0, z: -60.0 };
            t.mGear = 5;
            t.mEngineRPM = 7400.0;
            t.mEngineMaxRPM = 8000.0;
            t.mFuel = 40.0;
            t.mFuelCapacity = 80.0;
            t.mUnfilteredThrottle = 1.0;
            t.mUnfilteredSteering = -0.25;
            t.mPhysicalSteeringWheelRange = 360.0;
            t.mVehicleModel = name("Toyota GR010");
            t
        };
        // Heading north (+z); the Ferrari is 3 m to the east, i.e. on the right.
        let telemetry = vec![tele(4, 3.0, 100.0), tele(11, 0.0, 100.0), tele(20, 500.0, -200.0), tele(31, 900.0, 40.0)];
        Snapshot { info, vehicles, telemetry }
    }

    #[test]
    fn maps_a_race_into_telemetry_and_session_info() {
        let mut st = State::default();
        let mut tel = Telemetry::default();
        let mut snap = snapshot();
        // Two rows, the player moving north, so the heading is known.
        snap.telemetry[1].mPos.z = 90.0;
        assert!(apply(&mut st, &snap, &mut tel).is_some(), "first row brings session info");
        let snap = snapshot();
        let yaml = apply(&mut st, &snap, &mut tel);
        assert!(yaml.is_none(), "unchanged session info is not sent again");

        let me = tel.player_idx as usize;
        assert_eq!(st.slots[&11], me);
        assert_eq!(tel.session_state, 4);
        assert!(tel.session_flags & flags::GREEN != 0 && tel.session_flags & flags::START_HIDDEN != 0);
        assert!((tel.speed - 60.0).abs() < 0.01);
        assert_eq!((tel.gear, tel.lap, tel.lap_completed), (5, 4, 3));
        // A quarter turn left on a 360° wheel: 45° to the left, positive as in iRacing.
        assert!((tel.steering - 45f32.to_radians()).abs() < 1e-4, "{}", tel.steering);
        assert!((tel.fuel_level_pct - 0.5).abs() < 1e-6);
        // Lap fraction carried forward by speed between scoring updates (0.1 s at 60 m/s).
        assert!((tel.lap_dist_pct - (5006.0 / 13626.0) as f32).abs() < 1e-4, "{}", tel.lap_dist_pct);
        assert!(tel.yaw_north.abs() < 1e-4, "heading north: {}", tel.yaw_north);
        assert_eq!(tel.car_left_right, 3, "car on the right");
        assert_eq!(tel.cars[me].class_position, 2);
        assert_eq!(tel.cars[st.slots[&20]].class_position, 1);

        // The session info reads back through the iRacing parser.
        let mut st2 = State::default();
        let info = SessionInfo::parse(&apply(&mut st2, &snapshot(), &mut Telemetry::default()).unwrap());
        assert!(!is_iracing_track(info.track_id) && info.track_id >= TRACK_ID_BASE);
        assert_eq!(info.track_display_name, "Circuit de la Sarthe");
        assert!((info.track_length_km - 13.63).abs() < 0.01);
        assert_eq!(info.standing_start, Some(false));
        assert_eq!(info.drivers.len(), 4);
        let toyota = info.drivers.iter().find(|d| d.name == "Ana Ribeiro").unwrap();
        assert_eq!((toyota.car_number.as_str(), toyota.team_name.as_str(), toyota.class_name.as_str()), ("7", "Toyota Gazoo Racing 2025", "Hyper"));
        assert_eq!(toyota.class_color, 0xe10600);
        assert_eq!(toyota.abbrev, "Ribeiro, A");
        assert_eq!(info.player_car_idx, me as i32);
        let race = info.sessions.iter().find(|s| s.num == 10).unwrap();
        assert_eq!(race.kind, "Race");
        assert_eq!(race.laps, None, "a timed race");
        assert_eq!(race.results.len(), 4);
        assert!((info.fuel_max_ltr - 80.0).abs() < 0.01);
    }

    #[test]
    fn rolling_start_and_cars_in_the_garage() {
        let mut st = State::default();
        let mut tel = Telemetry::default();
        let mut snap = snapshot();
        snap.info.mGamePhase = 3;
        snap.vehicles[3].mInGarageStall = 1;
        apply(&mut st, &snap, &mut tel);
        assert_eq!(tel.session_state, 3, "formation lap: pacing");
        assert_eq!(crate::model::rolling_start(tel.session_state, tel.session_flags), "pacing");
        let gt3 = st.slots[&31];
        assert!(!tel.cars[gt3].in_world(), "a car in its garage is not on track");
        snap.info.mGamePhase = 5;
        apply(&mut st, &snap, &mut tel);
        assert_eq!(crate::model::rolling_start(tel.session_state, tel.session_flags), "green");
    }

    #[test]
    fn every_overlay_topic_builds_from_lmu_data() {
        use crate::topics::Topic;
        let mut e = crate::engine::Engine::new(crate::config::Config::default(), None);
        e.source = "lmu";
        let mut st = State::default();
        let mut snap = snapshot();
        for k in 0..120 {
            // Two minutes of racing at 5 Hz scoring: everyone moves on.
            snap.info.mCurrentET += 0.2;
            for (v, t) in snap.vehicles.iter_mut().zip(snap.telemetry.iter_mut()) {
                v.mLapDist = (v.mLapDist + 12.0) % 13_626.0;
                t.mElapsedTime = snap.info.mCurrentET + 0.05;
                t.mPos.z += 12.0;
            }
            if let Some(y) = apply(&mut st, &snap, &mut e.tel) {
                e.set_session(SessionInfo::parse(&y));
            }
            e.connected = true;
            e.tick();
            if k % 30 == 0 {
                for t in Topic::ALL {
                    let json = t.build(&e);
                    assert!(json.starts_with('{') || json.starts_with('['), "{}: {json}", t.name());
                    assert!(!json.contains("NaN") && !json.contains("inf"), "{}: {json}", t.name());
                }
            }
        }
        let rel = crate::topics::relative(&e);
        assert!(!rel.rows.is_empty(), "the relative has cars");
        let st = crate::topics::standings(&e);
        assert_eq!(st.classes.len(), 3, "Hyper, LMP2 and GT3");
    }

    #[test]
    fn private_qualifying_leaves_other_cars_off_the_relative_radar_map_spotter_and_battle() {
        let mut e = crate::engine::Engine::new(crate::config::Config::default(), None);
        e.source = "lmu";
        let (mut st, mut snap) = (State::default(), snapshot());
        snap.info.mSession = 5; // qualifying
        run(&mut e, &mut st, &mut snap, 3, |_| {});
        // Open qualifying (nothing says otherwise yet): everyone is on track together.
        let focus = e.tel.focus_idx().unwrap();
        assert!(!e.alone_on_track());
        assert!(crate::topics::relative(&e).rows.len() > 1);
        assert!(!crate::engine::radar::build(&e).cars.is_empty(), "the Ferrari alongside");
        assert!(crate::topics::cars(&e).cars.len() > 1, "everyone on the map");
        e.tel.car_left_right = 3;
        assert_eq!(crate::topics::spotter(&e).state, "right");
        assert!(crate::engine::battle::neighbours(&e, focus, crate::engine::battle::Mode::Track) != (None, None));
        // Two cars drive through each other: private qualifying.
        run(&mut e, &mut st, &mut snap, 4, |s| {
            let kenji = s.vehicles.iter().position(|v| v.mID == 31).unwrap();
            let lena = s.vehicles.iter().position(|v| v.mID == 20).unwrap();
            s.vehicles[lena].mLapDist = s.vehicles[kenji].mLapDist + 0.4;
            let (k, l) = (s.telemetry.iter().position(|t| t.mID == 31).unwrap(), s.telemetry.iter().position(|t| t.mID == 20).unwrap());
            s.telemetry[l].mPos = s.telemetry[k].mPos;
        });
        assert!(e.alone_on_track());
        let rel = crate::topics::relative(&e);
        assert_eq!(rel.rows.len(), 1, "only you");
        assert!(crate::engine::radar::build(&e).cars.is_empty());
        // The track map shows only your car, the spotter calls nothing, no one to battle.
        let map = crate::topics::cars(&e);
        assert_eq!(map.cars.len(), 1, "only you on the map");
        assert_eq!(Some(map.cars[0].i), e.tel.focus_idx());
        e.tel.car_left_right = 3;
        assert_eq!(crate::topics::spotter(&e).state, "", "no one is beside you");
        let sa = crate::safety::build(&e);
        assert!(sa.hazards.is_empty() && sa.approaching.is_empty() && sa.overtakes.is_empty(), "no safety alerts");
        for mode in crate::engine::battle::Mode::ALL {
            assert_eq!(crate::engine::battle::neighbours(&e, focus, mode), (None, None));
        }
        // The switch off: everyone shows even then.
        e.cfg.lmu_private_qualifying = false;
        assert!(crate::topics::relative(&e).rows.len() > 1);
        // Races are never private.
        e.cfg.lmu_private_qualifying = true;
        snap.info.mSession = 10;
        run(&mut e, &mut st, &mut snap, 3, |_| {});
        assert!(crate::topics::relative(&e).rows.len() > 1);
    }

    #[test]
    fn car_settings_and_live_tyres() {
        let mut e = crate::engine::Engine::new(crate::config::Config::default(), None);
        e.source = "lmu";
        let (mut st, mut snap) = (State::default(), snapshot());
        let t = &mut snap.telemetry[1];
        t.mRearBrakeBias = 0.452;
        (t.mTC, t.mTCMax, t.mMotorMap, t.mMotorMapMax) = (4, 11, 2, 5);
        let mut w = t.mWheels;
        w[0].mTemperature = [353.15, 358.15, 363.15];
        w[0].mPressure = 171.4;
        w[0].mWear = 0.83;
        w[0].mBrakeTemp = 773.15;
        w[0].mTireCarcassTemperature = 348.15;
        t.mWheels = w;
        run(&mut e, &mut st, &mut snap, 2, |_| {});
        let json = serde_json::to_string(&crate::topics::car_settings(&e)).unwrap();
        assert_eq!(
            json,
            r#"{"game":"lmu","driving":true,"items":[{"id":"brakeBias","value":54.8,"max":null},{"id":"tc","value":4.0,"max":11.0},{"id":"engineMap","value":2.0,"max":5.0}]}"#,
            "no ABS on this car"
        );
        let out = crate::topics::tyre_data(&e);
        assert!(out.live);
        let fl = out.wheels.unwrap()[0];
        assert_eq!(fl.temps, Some([80.0, 85.0, 90.0]));
        assert_eq!((fl.carcass, fl.pressure, fl.wear, fl.brake), (Some(75.0), Some(171.4), Some(83.0), Some(500.0)));
    }

    /// An engine fed from LMU scoring, `steps` updates at 5 Hz with `step` run between them.
    fn run(e: &mut crate::engine::Engine, st: &mut State, snap: &mut Snapshot, steps: usize, mut step: impl FnMut(&mut Snapshot)) {
        for _ in 0..steps {
            snap.info.mCurrentET += 0.2;
            for t in snap.telemetry.iter_mut() {
                t.mElapsedTime = snap.info.mCurrentET;
                t.mLocalVel.z = 0.0;
            }
            step(snap);
            if let Some(y) = apply(st, snap, &mut e.tel) {
                e.set_session(SessionInfo::parse(&y));
            }
            e.connected = true;
            e.tick();
        }
    }

    #[test]
    fn virtual_energy_per_lap_and_to_the_finish() {
        let mut e = crate::engine::Engine::new(crate::config::Config::default(), None);
        e.source = "lmu";
        let (mut st, mut snap) = (State::default(), snapshot());
        // The player's Hypercar: 90% of its virtual energy, using 6% a lap.
        snap.telemetry[1].mVirtualEnergy = 0.90;
        snap.telemetry[1].mBatteryChargeFraction = 0.42;
        run(&mut e, &mut st, &mut snap, 3, |_| {});
        for _ in 0..4 {
            run(&mut e, &mut st, &mut snap, 1, |s| {
                for v in s.vehicles.iter_mut() {
                    v.mTotalLaps += 1;
                }
                s.telemetry[1].mVirtualEnergy -= 0.06;
            });
        }
        let out = crate::topics::energy_out(&e);
        let json = serde_json::to_string(&out).unwrap();
        assert!(out.available && out.game == "lmu", "{json}");
        assert!((out.level.unwrap() - 66.0).abs() < 0.2, "{json}");
        assert_eq!(out.battery, Some(42.0));
        assert!((out.per_lap_avg.unwrap() - 6.0).abs() < 0.05, "{json}");
        assert!((out.laps_in_tank.unwrap() - 11.0).abs() < 0.1, "{json}");
        // A car that does not race on virtual energy (LMP2): nothing to show.
        snap.telemetry[1].mVirtualEnergy = 0.0;
        run(&mut e, &mut st, &mut snap, 1, |_| {});
        assert!(!crate::topics::energy_out(&e).available);
    }

    #[test]
    fn gaps_come_from_where_the_cars_are() {
        let mut e = crate::engine::Engine::new(crate::config::Config::default(), None);
        e.source = "lmu";
        let (mut st, mut snap) = (State::default(), snapshot());
        // The Ferrari 70 m ahead of the player, the LMP2 3 km behind on the same lap.
        snap.vehicles[0].mLapDist = 5070.0;
        run(&mut e, &mut st, &mut snap, 3, |_| {});
        let me = st.slots[&11];
        let (ferrari, lmp2) = (st.slots[&4], st.slots[&20]);
        // 210 s laps of 13,626 m: 70 m is about 1.1 s, 3 km about 46 s.
        let g = e.relative_gap(me, ferrari);
        assert!((g - 70.0 / 13626.0 * 210.0).abs() < 0.1, "Ferrari ahead by {g}");
        let g = e.relative_gap(me, lmp2);
        assert!((g + 3000.0 / 13626.0 * 210.0).abs() < 0.5, "LMP2 behind by {g}");
        // Race order: the player 1.1 s behind the class leader.
        let row = e.order.iter().find(|r| r.idx == me).unwrap();
        assert_eq!(row.class_pos, 2);
        let gap = row.gap.unwrap();
        assert!((gap - 70.0 / 13626.0 * 210.0).abs() < 0.1, "gap to the leader {gap}");
        let rel = crate::topics::relative(&e);
        let f = rel.rows.iter().find(|r| r.d.idx == ferrari).unwrap();
        assert!(f.gap > 1.0, "relative shows the gap: {}", f.gap);
    }

    #[test]
    fn crossing_the_line_counts_the_lap_straight_away() {
        let mut st = State::default();
        let mut tel = Telemetry::default();
        let mut snap = snapshot();
        // 2 m before the line, carried 6 m on by speed before the game counts the lap.
        snap.vehicles[1].mLapDist = 13_624.0;
        apply(&mut st, &snap, &mut tel);
        let me = &tel.cars[st.slots[&11]];
        assert_eq!(me.lap, 5, "on the next lap");
        assert!(me.pct < 0.01 && me.progress() > 5.0, "{}", me.progress());
    }

    #[test]
    fn fuel_or_energy_left_for_every_car() {
        let mut e = crate::engine::Engine::new(crate::config::Config::default(), None);
        e.source = "lmu";
        let (mut st, mut snap) = (State::default(), snapshot());
        snap.vehicles[0].mFuelFraction = 140; // the Ferrari: fuel too, but it races on energy
        snap.telemetry[0].mVirtualEnergy = 0.62;
        snap.vehicles[2].mFuelFraction = 51; // the LMP2: 20% of a tank
        run(&mut e, &mut st, &mut snap, 2, |_| {});
        let left = |id: i32| crate::topics::extras(&e, st.slots[&id]).fuel_left.map(|f| (f.value, f.unit, f.energy));
        assert_eq!(left(4), Some((62.0, "%", true)));
        assert_eq!(left(20), Some((20.0, "%", false)));
    }

    #[test]
    fn fuel_added_at_every_cars_stops() {
        let mut e = crate::engine::Engine::new(crate::config::Config::default(), None);
        e.source = "lmu";
        let (mut st, mut snap) = (State::default(), snapshot());
        for v in snap.vehicles.iter_mut() {
            v.mFuelFraction = 51; // 20%
        }
        run(&mut e, &mut st, &mut snap, 2, |_| {});
        let ferrari = st.slots[&4];
        let me = st.slots[&11];
        assert!(crate::topics::extras(&e, ferrari).fuel_added.is_none(), "no stop yet");
        // The Ferrari pits and takes 60% of a tank, a little at a time.
        run(&mut e, &mut st, &mut snap, 3, |s| {
            s.vehicles[0].mInPits = 1;
            s.vehicles[0].mPitState = 3;
        });
        let mut fill = 51u8;
        run(&mut e, &mut st, &mut snap, 6, |s| {
            fill = (fill + 26).min(204);
            s.vehicles[0].mFuelFraction = fill;
        });
        let x = crate::topics::extras(&e, ferrari).fuel_added.unwrap();
        assert!(x.filling && x.unit == "%" && x.last > 50.0, "filling shows as it goes: {}", x.last);
        run(&mut e, &mut st, &mut snap, 3, |s| {
            s.vehicles[0].mInPits = 0;
            s.vehicles[0].mPitState = 0;
        });
        let x = crate::topics::extras(&e, ferrari).fuel_added.unwrap();
        assert!(!x.filling && (x.last - 60.0).abs() < 0.5 && (x.total - 60.0).abs() < 0.5, "{} / {}", x.last, x.total);
        // A second, shorter stop: last is that one, the total adds up.
        run(&mut e, &mut st, &mut snap, 2, |s| {
            s.vehicles[0].mInPits = 1;
            s.vehicles[0].mFuelFraction = 102;
        });
        run(&mut e, &mut st, &mut snap, 2, |s| s.vehicles[0].mFuelFraction = 153);
        run(&mut e, &mut st, &mut snap, 2, |s| s.vehicles[0].mInPits = 0);
        let x = crate::topics::extras(&e, ferrari).fuel_added.unwrap();
        assert!((x.last - 20.0).abs() < 0.5 && (x.total - 80.0).abs() < 0.5, "{} / {}", x.last, x.total);
        // Driving through the pit lane without a stop adds nothing.
        run(&mut e, &mut st, &mut snap, 2, |s| s.vehicles[1].mInPits = 1);
        run(&mut e, &mut st, &mut snap, 2, |s| s.vehicles[1].mInPits = 0);
        assert!(crate::topics::extras(&e, me).fuel_added.is_none());
        // Shown on the relative and the standings.
        let json = crate::topics::Topic::ALL.iter().find(|t| t.name() == "standings").unwrap().build(&e);
        assert!(json.contains("\"fuelAdded\":{\"last\":20.0,\"total\":80.0,\"unit\":\"%\",\"filling\":false}"), "{json}");
    }

    #[test]
    fn learns_where_the_sectors_start() {
        let mut st = State::default();
        let mut tel = Telemetry::default();
        let mut snap = snapshot();
        let mut last = None;
        // Cars crossing into sector 2 at 4,000 m and sector 3 at 9,000 m of the 13,626 m lap.
        for k in 0..12 {
            snap.info.mCurrentET += 0.2;
            for v in snap.vehicles.iter_mut() {
                let (d, sector) = if k % 2 == 0 { (3990.0, 1) } else { (4005.0, 2) };
                v.mLapDist = d;
                v.mSector = sector;
            }
            if let Some(y) = apply(&mut st, &snap, &mut tel) {
                last = Some(y);
            }
            for v in snap.vehicles.iter_mut() {
                v.mSector = if k % 2 == 0 { 2 } else { 0 };
                v.mLapDist = if k % 2 == 0 { 4005.0 } else { 9001.0 };
            }
            snap.info.mCurrentET += 0.2;
            if let Some(y) = apply(&mut st, &snap, &mut tel) {
                last = Some(y);
            }
        }
        let info = SessionInfo::parse(&last.unwrap());
        assert_eq!(info.sectors.len(), 3, "{:?}", info.sectors);
        assert!((info.sectors[1] - 4005.0 / 13626.0).abs() < 0.002, "{:?}", info.sectors);
        assert!((info.sectors[2] - 9001.0 / 13626.0).abs() < 0.002, "{:?}", info.sectors);
    }

    #[test]
    fn names_numbers_and_tracks() {
        assert_eq!(car_number("Toyota Gazoo Racing 2025 #7:LM").as_deref(), Some("7"));
        assert_eq!(car_number("Iron Dames #85"), Some("85".into()));
        assert_eq!(car_number("Practice car"), None);
        assert_eq!(team_name("Iron Dames 2025 #85:LM"), "Iron Dames 2025");
        assert_eq!(abbrev("Max Verstappen"), "Verstappen, M");
        assert_eq!(track_id("Spa"), track_id("SPA"), "stable whatever the case");
        assert!(is_iracing_track(127) && !is_iracing_track(track_id("Monza")));
        assert_eq!(wetness(0.0), 1);
        assert_eq!(wetness(0.9), 7);
    }
}
