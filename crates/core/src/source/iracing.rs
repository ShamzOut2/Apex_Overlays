//! Live iRacing source (Windows). Elsewhere it simply reports "disconnected".

use super::{Poll, Source};
use crate::model::{CarSetting, Telemetry, WheelTel, MAX_CARS};
use apex_irsdk::{Row, Var, VarTable};

macro_rules! vars {
    ($($field:ident = $name:literal),* $(,)?) => {
        /// Variable handles resolved once per connection.
        #[derive(Default, Clone, Copy)]
        #[allow(non_snake_case)]
        pub struct Vars { $(pub $field: Option<Var>),* }
        impl Vars {
            pub fn resolve(t: &VarTable) -> Self { Self { $($field: t.get($name)),* } }
        }
    };
}

vars! {
    session_time = "SessionTime", session_num = "SessionNum", session_state = "SessionState",
    session_flags = "SessionFlags", session_time_remain = "SessionTimeRemain",
    session_laps_remain = "SessionLapsRemainEx", time_of_day = "SessionTimeOfDay",
    player_idx = "PlayerCarIdx", cam_car_idx = "CamCarIdx", replay = "IsReplayPlaying",
    speed = "Speed", rpm = "RPM", gear = "Gear", throttle = "Throttle", brake = "Brake", clutch = "Clutch",
    steering = "SteeringWheelAngle", steering_max = "SteeringWheelAngleMax", abs_active = "BrakeABSactive",
    fuel_level = "FuelLevel", fuel_level_pct = "FuelLevelPct", fuel_use_per_hour = "FuelUsePerHour",
    lap = "Lap", lap_completed = "LapCompleted", lap_dist_pct = "LapDistPct", lap_dist = "LapDist",
    lap_current_time = "LapCurrentLapTime", lap_last_time = "LapLastLapTime", lap_best_time = "LapBestLapTime",
    delta_best = "LapDeltaToBestLap", delta_best_ok = "LapDeltaToBestLap_OK",
    on_pit_road = "OnPitRoad", is_on_track = "IsOnTrack", incidents = "PlayerCarMyIncidentCount",
    yaw_north = "YawNorth", lat_accel = "LatAccel", long_accel = "LongAccel", tire_compound = "PlayerTireCompound",
    track_temp = "TrackTempCrew", air_temp = "AirTemp", track_wetness = "TrackWetness", precipitation = "Precipitation",
    declared_wet = "WeatherDeclaredWet", wind_vel = "WindVel", wind_dir = "WindDir", humidity = "RelativeHumidity",
    fog = "FogLevel", skies = "Skies", air_pressure = "AirPressure", display_units = "DisplayUnits", car_left_right = "CarLeftRight",
    engine_warnings = "EngineWarnings", in_pit_stall = "PlayerCarInPitStall", pitstop_active = "PitstopActive",
    pit_sv_status = "PlayerCarPitSvStatus", pit_repair_left = "PitRepairLeft", pit_opt_repair_left = "PitOptRepairLeft",
    pit_sv_flags = "PitSvFlags", pit_sv_fuel = "PitSvFuel", pit_sv_lfp = "PitSvLFP", pit_sv_rfp = "PitSvRFP",
    pit_sv_lrp = "PitSvLRP", pit_sv_rrp = "PitSvRRP", pit_sv_compound = "PitSvTireCompound", fast_repairs_used = "PlayerFastRepairsUsed",
    cam_group = "CamGroupNumber", cam_camera = "CamCameraNumber",
    car_lap = "CarIdxLap", car_lap_completed = "CarIdxLapCompleted", car_pct = "CarIdxLapDistPct",
    car_est = "CarIdxEstTime", car_f2 = "CarIdxF2Time", car_pos = "CarIdxPosition", car_class_pos = "CarIdxClassPosition",
    car_pit = "CarIdxOnPitRoad", car_surface = "CarIdxTrackSurface", car_last = "CarIdxLastLapTime",
    car_best = "CarIdxBestLapTime", car_tire = "CarIdxTireCompound", car_gear = "CarIdxGear", car_rpm = "CarIdxRPM", car_flags = "CarIdxSessionFlags",
    // Settings changed from the wheel: each car has only some of these.
    dc_brake_bias = "dcBrakeBias", dc_peak_bias = "dcPeakBrakeBias", dc_abs = "dcABS", dc_tc = "dcTractionControl",
    dc_tc2 = "dcTractionControl2", dc_throttle = "dcThrottleShape", dc_fuel_mix = "dcFuelMixture",
    dc_engine_braking = "dcEngineBraking", dc_engine_power = "dcEnginePower", dc_arb_front = "dcAntiRollFront",
    dc_arb_rear = "dcAntiRollRear", dc_weight_jacker = "dcWeightJackerRight", dc_diff_entry = "dcDiffEntry",
    dc_diff_middle = "dcDiffMiddle", dc_diff_exit = "dcDiffExit", dc_deploy = "dcMGUKDeployMode",
    dc_regen = "dcMGUKRegenGain", dc_boost = "dcBoostLevel",
}

/// A brake bias as front %, whatever scale the car reports it in; `None` when it is not one
/// (an offset or a setting of another kind).
pub fn brake_bias_pct(v: f32) -> Option<f32> {
    let pct = if v > 0.0 && v <= 1.0 {
        v * 100.0
    } else if v > 100.0 && v <= 10_000.0 {
        v / 100.0
    } else {
        v
    };
    (20.0..=90.0).contains(&pct).then_some(pct)
}

/// One wheel's tyre readings (iRacing updates them when the crew measures them at a stop).
#[derive(Default, Clone, Copy)]
pub struct WheelVars {
    temps: [Option<Var>; 3],
    wear: [Option<Var>; 3],
    cold_pressure: Option<Var>,
}

/// Front-left, front-right, rear-left, rear-right.
pub fn wheel_vars(t: &VarTable) -> [WheelVars; 4] {
    ["LF", "RF", "LR", "RR"].map(|w| WheelVars {
        temps: ["CL", "CM", "CR"].map(|s| t.get(&format!("{w}temp{s}"))),
        wear: ["L", "M", "R"].map(|s| t.get(&format!("{w}wear{s}"))),
        cold_pressure: t.get(&format!("{w}coldPressure")),
    })
}

/// The player's tyres: temperatures (°C, left to right), tread left and cold pressures.
pub fn fill_wheels(w: &[WheelVars; 4], r: Row, t: &mut Telemetry) {
    t.tyres_live = false;
    t.tyre_optimal = None;
    if w.iter().all(|w| w.temps[0].is_none() && w.cold_pressure.is_none()) {
        t.wheels = None;
        return;
    }
    t.wheels = Some(w.map(|w| {
        let temps = w.temps.map(|v| r.f32(v));
        let wear = w.wear.map(|v| r.f32(v));
        WheelTel {
            // All zero: not measured yet.
            temps: temps.iter().any(|&v| v != 0.0).then_some(temps),
            pressure: w.cold_pressure.map(|v| r.f32(Some(v))).filter(|&p| p > 0.0),
            wear: wear.iter().any(|&v| v > 0.0).then(|| (wear.iter().sum::<f32>() / 3.0 * 100.0).clamp(0.0, 100.0)),
            ..Default::default()
        }
    }));
}

/// Copy one telemetry row into the normalised model.
pub fn fill(v: &Vars, r: Row, t: &mut Telemetry) {
    t.session_time = r.f64(v.session_time);
    t.session_num = r.i32(v.session_num);
    t.session_state = r.i32(v.session_state);
    t.session_flags = r.u32(v.session_flags);
    t.session_time_remain = r.f64(v.session_time_remain);
    t.session_laps_remain = r.i32(v.session_laps_remain);
    t.time_of_day = r.f32(v.time_of_day);
    t.player_idx = r.i32(v.player_idx);
    t.cam_car_idx = r.i32(v.cam_car_idx);
    t.is_replay_playing = r.bool(v.replay);
    t.speed = r.f32(v.speed);
    t.rpm = r.f32(v.rpm);
    t.gear = r.i32(v.gear);
    t.throttle = r.f32(v.throttle);
    t.brake = r.f32(v.brake);
    // The SDK reports clutch *engagement* (1 = pedal released); overlays want pedal travel.
    t.clutch = if v.clutch.is_some() { 1.0 - r.f32(v.clutch) } else { 0.0 };
    t.steering = r.f32(v.steering);
    t.steering_max = r.f32(v.steering_max);
    t.abs_active = r.bool(v.abs_active);
    t.fuel_level = r.f32(v.fuel_level);
    t.fuel_level_pct = r.f32(v.fuel_level_pct);
    t.fuel_use_per_hour = r.f32(v.fuel_use_per_hour);
    t.lap = r.i32(v.lap);
    t.lap_completed = r.i32(v.lap_completed);
    t.lap_dist_pct = r.f32(v.lap_dist_pct);
    t.lap_dist = r.f32(v.lap_dist);
    t.lap_current_time = r.f32(v.lap_current_time);
    t.lap_last_time = r.f32(v.lap_last_time);
    t.lap_best_time = r.f32(v.lap_best_time);
    t.delta_best = r.f32(v.delta_best);
    t.delta_best_ok = r.bool(v.delta_best_ok);
    t.on_pit_road = r.bool(v.on_pit_road);
    t.is_on_track = r.bool(v.is_on_track);
    t.incidents = r.i32(v.incidents);
    t.yaw_north = r.f32(v.yaw_north);
    t.lat_accel = r.f32(v.lat_accel);
    t.long_accel = r.f32(v.long_accel);
    t.tire_compound = r.i32(v.tire_compound);
    t.track_temp = r.f32(v.track_temp);
    t.air_temp = r.f32(v.air_temp);
    t.track_wetness = r.i32(v.track_wetness);
    t.precipitation = r.f32(v.precipitation);
    t.declared_wet = r.bool(v.declared_wet);
    t.wind_vel = r.f32(v.wind_vel);
    t.wind_dir = r.f32(v.wind_dir);
    t.humidity = r.f32(v.humidity);
    t.fog = r.f32(v.fog);
    t.skies = r.i32(v.skies);
    t.air_pressure = r.f32(v.air_pressure);
    t.display_metric = v.display_units.is_none() || r.i32(v.display_units) == 1;
    t.car_left_right = r.i32(v.car_left_right);
    t.engine_warnings = r.u32(v.engine_warnings);
    t.in_pit_stall = r.bool(v.in_pit_stall);
    t.pitstop_active = r.bool(v.pitstop_active);
    t.pit_sv_status = r.i32(v.pit_sv_status);
    t.pit_repair_left = r.f32(v.pit_repair_left);
    t.pit_sv_flags = r.u32(v.pit_sv_flags);
    t.pit_sv_fuel = r.f32(v.pit_sv_fuel);
    t.pit_sv_pressures = [r.f32(v.pit_sv_lfp), r.f32(v.pit_sv_rfp), r.f32(v.pit_sv_lrp), r.f32(v.pit_sv_rrp)];
    t.pit_sv_compound = r.i32(v.pit_sv_compound);
    t.fast_repairs_used = r.i32(v.fast_repairs_used);
    t.pit_opt_repair_left = r.f32(v.pit_opt_repair_left);
    t.cam_group = r.i32(v.cam_group);
    t.cam_camera = r.i32(v.cam_camera);
    t.car_settings.clear();
    let dc = [
        (v.dc_brake_bias, "brakeBias"),
        (v.dc_peak_bias, "peakBias"),
        (v.dc_abs, "abs"),
        (v.dc_tc, "tc"),
        (v.dc_tc2, "tc2"),
        (v.dc_throttle, "throttleMap"),
        (v.dc_fuel_mix, "fuelMix"),
        (v.dc_engine_braking, "engineBraking"),
        (v.dc_engine_power, "enginePower"),
        (v.dc_arb_front, "arbFront"),
        (v.dc_arb_rear, "arbRear"),
        (v.dc_weight_jacker, "weightJacker"),
        (v.dc_diff_entry, "diffEntry"),
        (v.dc_diff_middle, "diffMiddle"),
        (v.dc_diff_exit, "diffExit"),
        (v.dc_deploy, "deployMode"),
        (v.dc_regen, "regenGain"),
        (v.dc_boost, "boost"),
    ];
    for (var, id) in dc {
        if var.is_some() {
            let value = r.f32(var);
            // Brake bias is kept as the car gives it (a %, a fraction, hundredths of a %, or an
            // adjustment from the setup's bias) and read against the setup in the car settings
            // topic. Anything else is a setting step: a value no car has (NaN, huge) is a
            // misread and left out rather than shown.
            let bias = matches!(id, "brakeBias" | "peakBias");
            if !value.is_finite() || value.abs() > if bias { 10_000.0 } else { 1_000.0 } {
                continue;
            }
            t.car_settings.push(CarSetting { id, value, max: None });
        }
    }
    for (i, c) in t.cars.iter_mut().enumerate().take(MAX_CARS) {
        c.lap = r.i32_at(v.car_lap, i);
        c.lap_completed = r.i32_at(v.car_lap_completed, i);
        c.pct = r.f32_at(v.car_pct, i);
        c.est_time = r.f32_at(v.car_est, i);
        c.f2_time = r.f32_at(v.car_f2, i);
        c.position = r.i32_at(v.car_pos, i);
        c.class_position = r.i32_at(v.car_class_pos, i);
        c.on_pit_road = r.bool_at(v.car_pit, i);
        c.surface = if v.car_surface.is_some() { r.i32_at(v.car_surface, i) } else { -1 };
        c.last_lap = r.f32_at(v.car_last, i);
        c.best_lap = r.f32_at(v.car_best, i);
        c.tire_compound = r.i32_at(v.car_tire, i);
        c.gear = r.i32_at(v.car_gear, i);
        c.rpm = r.f32_at(v.car_rpm, i);
        c.session_flags = r.i32_at(v.car_flags, i) as u32;
    }
}

pub struct IRacing {
    #[cfg(windows)]
    shm: Option<apex_irsdk::SharedMemory>,
    #[cfg(windows)]
    vars: Vars,
    #[cfg(windows)]
    wheels: [WheelVars; 4],
    #[cfg(windows)]
    row: Vec<u8>,
    #[cfg(windows)]
    last_tick: i32,
    #[cfg(windows)]
    last_session_update: i32,
    #[cfg(windows)]
    layout: (i32, i32),
    /// When the telemetry last moved on (iRacing ticks 60 times a second even when paused).
    #[cfg(windows)]
    ticked_at: std::time::Instant,
}

impl Default for IRacing {
    fn default() -> Self {
        Self::new()
    }
}

impl IRacing {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            shm: None,
            #[cfg(windows)]
            vars: Vars::default(),
            #[cfg(windows)]
            wheels: Default::default(),
            #[cfg(windows)]
            row: Vec::new(),
            #[cfg(windows)]
            last_tick: -1,
            #[cfg(windows)]
            last_session_update: -1,
            #[cfg(windows)]
            layout: (-1, -1),
            #[cfg(windows)]
            ticked_at: std::time::Instant::now(),
        }
    }
}

impl Source for IRacing {
    fn name(&self) -> &'static str {
        "iracing"
    }

    #[cfg(windows)]
    fn replay(&mut self, session_num: i32, session_time: f64, car_number_raw: Option<i32>, camera: (i32, i32)) {
        use apex_irsdk::{broadcast, BroadcastMsg};
        if let Some(car) = car_number_raw {
            broadcast(BroadcastMsg::CamSwitchNum, car, (camera.0 & 0xffff) | (camera.1 << 16));
        }
        broadcast(BroadcastMsg::ReplaySearchSessionTime, session_num, (session_time.max(0.0) * 1000.0) as i32);
        broadcast(BroadcastMsg::ReplaySetPlaySpeed, 1, 0);
    }

    #[cfg(not(windows))]
    fn poll(&mut self, _tel: &mut Telemetry) -> Poll {
        std::thread::sleep(std::time::Duration::from_millis(500));
        Poll::Disconnected
    }

    #[cfg(windows)]
    fn poll(&mut self, tel: &mut Telemetry) -> Poll {
        use apex_irsdk::SharedMemory;
        if self.shm.is_none() {
            self.shm = SharedMemory::open();
            self.last_session_update = -1;
            self.last_tick = -1;
            self.layout = (-1, -1);
            self.ticked_at = std::time::Instant::now();
            if self.shm.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(500));
                return Poll::Disconnected;
            }
        }
        let shm = self.shm.as_mut().unwrap();
        let got_signal = shm.wait(50);
        let h = shm.header();
        if !h.is_connected() {
            self.last_session_update = -1;
            // The variables are listed again for the next session (another car has other dc…
            // settings), so their places are looked up again then.
            self.layout = (-1, -1);
            if !got_signal {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            return Poll::Disconnected;
        }
        // Looked up again when the table moves or changes size, and with every new session
        // info (a new session or car): the same number of variables does not mean the same
        // variables in the same places, and stale places read the wrong bytes (an ABS of
        // millions, a brake bias that never moves).
        if (h.num_vars, h.var_header_offset) != self.layout || h.session_info_update != self.last_session_update {
            self.layout = (h.num_vars, h.var_header_offset);
            self.vars = Vars::resolve(shm.vars(&h));
            self.wheels = wheel_vars(shm.vars(&h));
        }
        let Some(tick) = shm.read_row(&mut self.row) else { return Poll::Idle };
        if tick == self.last_tick {
            // Still marked connected but nothing new for 10 s: the sim closed without saying
            // so (a crash); treat it as gone, so the overlays hide and the other game is looked for.
            if self.ticked_at.elapsed() > std::time::Duration::from_secs(10) {
                std::thread::sleep(std::time::Duration::from_millis(200));
                return Poll::Disconnected;
            }
            return Poll::Idle;
        }
        self.last_tick = tick;
        self.ticked_at = std::time::Instant::now();
        tel.tick = tick;
        fill(&self.vars, Row(&self.row), tel);
        fill_wheels(&self.wheels, Row(&self.row), tel);
        let session_yaml = (h.session_info_update != self.last_session_update).then(|| {
            self.last_session_update = h.session_info_update;
            shm.session_info(&h)
        });
        Poll::Data { session_yaml }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apex_irsdk::{VarHeader, VarType};

    /// A variable table of floats, and one row holding their values.
    fn table(vars: &[(&str, f32)]) -> (VarTable, Vec<u8>) {
        let headers = vars
            .iter()
            .enumerate()
            .map(|(i, (name, _))| VarHeader {
                ty: VarType::Float,
                offset: i * 4,
                count: 1,
                count_as_time: false,
                name: name.to_string(),
                desc: String::new(),
                unit: String::new(),
            })
            .collect();
        (VarTable::new(headers), vars.iter().flat_map(|(_, v)| v.to_le_bytes()).collect())
    }

    #[test]
    fn settings_the_car_has_and_tyres_from_the_last_stop() {
        let (vars, row) = table(&[
            ("dcBrakeBias", 54.5),
            ("dcABS", 3.0),
            ("dcTractionControl", 4.0),
            ("LFtempCL", 80.0),
            ("LFtempCM", 85.0),
            ("LFtempCR", 90.0),
            ("LFwearL", 0.9),
            ("LFwearM", 0.88),
            ("LFwearR", 0.86),
            ("LFcoldPressure", 165.0),
            ("RFtempCL", 0.0),
        ]);
        let mut t = Telemetry::default();
        fill(&Vars::resolve(&vars), Row(&row), &mut t);
        fill_wheels(&wheel_vars(&vars), Row(&row), &mut t);
        let ids: Vec<_> = t.car_settings.iter().map(|s| (s.id, s.value)).collect();
        assert_eq!(ids, vec![("brakeBias", 54.5), ("abs", 3.0), ("tc", 4.0)]);
        // A misread (a value no setting has) is left out.
        let (vars, row) = table(&[("dcABS", 4.2e9), ("dcTractionControl", f32::NAN), ("dcBrakeBias", -1.5)]);
        let mut t2 = Telemetry::default();
        fill(&Vars::resolve(&vars), Row(&row), &mut t2);
        let ids: Vec<_> = t2.car_settings.iter().map(|s| (s.id, s.value)).collect();
        assert_eq!(ids, vec![("brakeBias", -1.5)], "the bias adjustment is kept, read against the setup later");
        // Brake bias in other scales: hundredths of a % (5500), a fraction; not a bias: an offset.
        assert_eq!(brake_bias_pct(5500.0), Some(55.0));
        assert_eq!(brake_bias_pct(0.565), Some(56.5));
        assert_eq!(brake_bias_pct(54.25), Some(54.25));
        assert_eq!(brake_bias_pct(-2.0), None);
        assert_eq!(brake_bias_pct(3.0), None);
        let w = t.wheels.unwrap();
        assert_eq!(w[0].temps, Some([80.0, 85.0, 90.0]));
        assert_eq!(w[0].pressure, Some(165.0));
        assert!((w[0].wear.unwrap() - 88.0).abs() < 0.01);
        assert_eq!(w[1].temps, None, "not measured yet");
        assert!(!t.tyres_live);
        // A car without any of these.
        let (vars, row) = table(&[("Speed", 10.0)]);
        fill(&Vars::resolve(&vars), Row(&row), &mut t);
        fill_wheels(&wheel_vars(&vars), Row(&row), &mut t);
        assert!(t.car_settings.is_empty() && t.wheels.is_none());
    }
}
