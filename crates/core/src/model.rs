//! Normalised simulator state. Sources (live iRacing, mock) fill these structs;
//! everything downstream only depends on this module, never on raw SDK vars.

use apex_irsdk::yaml::{self, Yaml};

pub const MAX_CARS: usize = 64;

/// irsdk_TrkLoc
pub mod surface {
    pub const NOT_IN_WORLD: i32 = -1;
    pub const OFF_TRACK: i32 = 0;
    pub const IN_PIT_STALL: i32 = 1;
    pub const APPROACHING_PITS: i32 = 2;
    pub const ON_TRACK: i32 = 3;
}

/// irsdk_Flags
pub mod flags {
    pub const CHECKERED: u32 = 0x0001;
    pub const WHITE: u32 = 0x0002;
    pub const GREEN: u32 = 0x0004;
    pub const YELLOW: u32 = 0x0008;
    pub const RED: u32 = 0x0010;
    pub const BLUE: u32 = 0x0020;
    pub const DEBRIS: u32 = 0x0040;
    pub const CROSSED: u32 = 0x0080;
    pub const YELLOW_WAVING: u32 = 0x0100;
    pub const ONE_LAP_TO_GREEN: u32 = 0x0200;
    pub const GREEN_HELD: u32 = 0x0400;
    pub const TEN_TO_GO: u32 = 0x0800;
    pub const FIVE_TO_GO: u32 = 0x1000;
    pub const RANDOM_WAVING: u32 = 0x2000;
    pub const CAUTION: u32 = 0x4000;
    pub const CAUTION_WAVING: u32 = 0x8000;
    pub const BLACK: u32 = 0x10000;
    pub const DISQUALIFY: u32 = 0x20000;
    pub const SERVICIBLE: u32 = 0x40000;
    pub const FURLED: u32 = 0x80000;
    pub const REPAIR: u32 = 0x100000;
    pub const START_HIDDEN: u32 = 0x1000_0000;
    pub const START_READY: u32 = 0x2000_0000;
    pub const START_SET: u32 = 0x4000_0000;
    pub const START_GO: u32 = 0x8000_0000;

    /// Every irsdk_Flags bit, by name (all of these are shown by the Flags overlay).
    pub const ALL: &[(u32, &str)] = &[
        (CHECKERED, "checkered"),
        (WHITE, "white"),
        (GREEN, "green"),
        (YELLOW, "yellow"),
        (RED, "red"),
        (BLUE, "blue"),
        (DEBRIS, "debris"),
        (CROSSED, "crossed"),
        (YELLOW_WAVING, "yellowWaving"),
        (ONE_LAP_TO_GREEN, "oneLapToGreen"),
        (GREEN_HELD, "greenHeld"),
        (TEN_TO_GO, "tenToGo"),
        (FIVE_TO_GO, "fiveToGo"),
        (RANDOM_WAVING, "randomWaving"),
        (CAUTION, "caution"),
        (CAUTION_WAVING, "cautionWaving"),
        (BLACK, "black"),
        (DISQUALIFY, "disqualify"),
        (SERVICIBLE, "servicible"),
        (FURLED, "furled"),
        (REPAIR, "repair"),
        (START_HIDDEN, "startHidden"),
        (START_READY, "startReady"),
        (START_SET, "startSet"),
        (START_GO, "startGo"),
    ];

    pub fn names(f: u32) -> Vec<&'static str> {
        ALL.iter().filter(|(b, _)| f & b != 0).map(|(_, n)| *n).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CarTel {
    pub lap: i32,
    pub lap_completed: i32,
    pub pct: f32,
    pub est_time: f32,
    pub f2_time: f32,
    pub position: i32,
    pub class_position: i32,
    pub on_pit_road: bool,
    pub surface: i32,
    pub last_lap: f32,
    pub best_lap: f32,
    pub tire_compound: i32,
    pub gear: i32,
    pub rpm: f32,
    /// Flags shown to this car (irsdk_Flags): black, meatball, blue, ...
    pub session_flags: u32,
}

impl CarTel {
    /// Laps completed plus fraction of the current lap.
    pub fn progress(&self) -> f64 {
        self.lap.max(0) as f64 + self.pct.clamp(0.0, 1.0) as f64
    }
    pub fn in_world(&self) -> bool {
        self.surface != surface::NOT_IN_WORLD && self.pct >= 0.0
    }
}

#[derive(Debug, Clone)]
pub struct Telemetry {
    pub tick: i32,
    pub session_time: f64,
    pub session_num: i32,
    pub session_state: i32,
    pub session_flags: u32,
    pub session_time_remain: f64,
    pub session_laps_remain: i32,
    pub time_of_day: f32,
    pub player_idx: i32,
    /// Car the camera is following (used as focus when spectating).
    pub cam_car_idx: i32,
    /// A replay is playing (the player's car is then whoever recorded it).
    pub is_replay_playing: bool,

    pub speed: f32,
    pub rpm: f32,
    pub gear: i32,
    pub throttle: f32,
    pub brake: f32,
    /// 0 = pedal released, 1 = pedal fully pressed (already inverted from the SDK's engagement value).
    pub clutch: f32,
    pub steering: f32,
    pub steering_max: f32,
    pub abs_active: bool,

    pub fuel_level: f32,
    pub fuel_level_pct: f32,
    pub fuel_use_per_hour: f32,

    pub lap: i32,
    pub lap_completed: i32,
    pub lap_dist_pct: f32,
    pub lap_dist: f32,
    pub lap_current_time: f32,
    pub lap_last_time: f32,
    pub lap_best_time: f32,
    pub delta_best: f32,
    pub delta_best_ok: bool,

    pub on_pit_road: bool,
    pub is_on_track: bool,
    pub incidents: i32,
    pub yaw_north: f32,
    pub lat_accel: f32,
    pub long_accel: f32,
    pub tire_compound: i32,

    pub track_temp: f32,
    pub air_temp: f32,
    pub track_wetness: i32,
    pub precipitation: f32,
    pub declared_wet: bool,
    pub wind_vel: f32,
    pub wind_dir: f32,
    pub humidity: f32,
    pub fog: f32,
    pub skies: i32,
    pub air_pressure: f32,
    pub display_metric: bool,
    /// Spotter: irsdk_CarLeftRight (0 off, 1 clear, 2 car left, 3 car right, 4 cars left and
    /// right, 5 two cars left, 6 two cars right).
    pub car_left_right: i32,
    /// irsdk_EngineWarnings bits (0x10 = pit speed limiter on).
    pub engine_warnings: u32,
    pub in_pit_stall: bool,
    /// The pit crew is servicing the car.
    pub pitstop_active: bool,
    /// irsdk_PitSvStatus: 0 none, 1 in progress, 2 complete, 100+ problems.
    pub pit_sv_status: i32,
    /// Mandatory and optional repair time left (s).
    pub pit_repair_left: f32,
    pub pit_opt_repair_left: f32,
    /// Camera group and camera in use (for replays).
    pub cam_group: i32,
    pub cam_camera: i32,

    pub cars: Vec<CarTel>,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            tick: 0,
            session_time: 0.0,
            session_num: 0,
            session_state: 0,
            session_flags: 0,
            session_time_remain: 0.0,
            session_laps_remain: 0,
            time_of_day: 0.0,
            player_idx: -1,
            cam_car_idx: -1,
            is_replay_playing: false,
            speed: 0.0,
            rpm: 0.0,
            gear: 0,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            steering: 0.0,
            steering_max: 0.0,
            abs_active: false,
            fuel_level: 0.0,
            fuel_level_pct: 0.0,
            fuel_use_per_hour: 0.0,
            lap: 0,
            lap_completed: 0,
            lap_dist_pct: 0.0,
            lap_dist: 0.0,
            lap_current_time: 0.0,
            lap_last_time: 0.0,
            lap_best_time: 0.0,
            delta_best: 0.0,
            delta_best_ok: false,
            on_pit_road: false,
            is_on_track: false,
            incidents: 0,
            yaw_north: 0.0,
            lat_accel: 0.0,
            long_accel: 0.0,
            tire_compound: 0,
            track_temp: 0.0,
            air_temp: 0.0,
            track_wetness: 0,
            precipitation: 0.0,
            declared_wet: false,
            wind_vel: 0.0,
            wind_dir: 0.0,
            humidity: 0.0,
            fog: 0.0,
            skies: 0,
            air_pressure: 0.0,
            display_metric: true,
            car_left_right: 0,
            engine_warnings: 0,
            in_pit_stall: false,
            pitstop_active: false,
            pit_sv_status: 0,
            pit_repair_left: 0.0,
            pit_opt_repair_left: 0.0,
            cam_group: 0,
            cam_camera: 0,
            cars: vec![CarTel { surface: surface::NOT_IN_WORLD, pct: -1.0, ..Default::default() }; MAX_CARS],
        }
    }
}

impl Telemetry {
    pub fn player(&self) -> Option<&CarTel> {
        usize::try_from(self.player_idx).ok().and_then(|i| self.cars.get(i))
    }

    /// The player is driving live (not spectating or watching a replay).
    pub fn driving(&self) -> bool {
        self.is_on_track && !self.is_replay_playing
    }

    /// The car overlays centre on: the player's car while driving, otherwise the car the
    /// camera follows (spectating, or any car in a replay).
    pub fn focus_idx(&self) -> Option<usize> {
        let ok = |i: i32| usize::try_from(i).ok().filter(|&i| self.cars.get(i).is_some_and(|c| c.in_world()));
        if self.driving() {
            ok(self.player_idx).or_else(|| ok(self.cam_car_idx))
        } else {
            ok(self.cam_car_idx).or_else(|| ok(self.player_idx))
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Driver {
    pub car_idx: usize,
    pub name: String,
    pub abbrev: String,
    pub initials: String,
    pub user_id: i64,
    pub team_id: i64,
    pub team_name: String,
    pub car_number: String,
    /// Car number as iRacing encodes it for camera commands (CarNumberRaw).
    pub car_number_raw: i32,
    pub car_id: i64,
    pub car_path: String,
    pub car_name: String,
    pub car_short: String,
    pub class_id: i64,
    pub class_name: String,
    pub class_color: u32,
    pub class_est_lap: f32,
    pub irating: i32,
    pub lic_string: String,
    pub lic_color: u32,
    pub club: String,
    /// The country flag ("flair") the driver chose on their iRacing account.
    pub flair: String,
    /// ISO 3166 alpha-2 code derived from the flair (or a single-country club), "" if unknown.
    pub country: String,
    /// Manufacturer derived from the car name, e.g. "Porsche".
    pub make: String,
    pub is_spectator: bool,
    pub is_pace_car: bool,
    pub is_ai: bool,
    pub incidents: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ResultPos {
    pub position: i32,
    pub class_position: i32,
    pub car_idx: usize,
    pub laps_complete: i32,
    pub fastest_time: f32,
    pub last_time: f32,
    pub incidents: i32,
    pub reason_out: String,
}

#[derive(Debug, Clone, Default)]
pub struct SessionDef {
    pub num: i32,
    pub kind: String,
    pub name: String,
    pub laps: Option<i32>,
    pub time: Option<f64>,
    pub results: Vec<ResultPos>,
    /// How much rubber is down: iRacing's SessionTrackRubberState ("clean", "low usage",
    /// "moderately high usage", "carry over"...).
    pub rubber: String,
}

impl SessionDef {
    pub fn is_race(&self) -> bool {
        self.kind.eq_ignore_ascii_case("race")
    }
}

#[derive(Debug, Clone, Default)]
pub struct TireDef {
    pub index: i32,
    pub kind: String,
}

impl TireDef {
    pub fn is_wet(&self) -> bool {
        self.kind.to_ascii_lowercase().contains("wet") || self.kind.to_ascii_lowercase().contains("rain")
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionInfo {
    pub track_id: i64,
    pub track_name: String,
    pub track_display_name: String,
    pub track_config: String,
    pub track_length_km: f32,
    pub track_city: String,
    pub track_country: String,
    pub series_id: i64,
    /// Official (ranked) session: iRating and SR change.
    pub official: bool,
    pub subsession_id: i64,
    pub league_id: i64,
    pub event_type: String,
    pub incident_limit: Option<i32>,
    /// Pit lane speed limit (km/h, 0 = unknown).
    pub pit_speed_limit_kph: f32,
    pub sessions: Vec<SessionDef>,
    pub drivers: Vec<Driver>,
    pub player_car_idx: i32,
    pub pace_car_idx: i32,
    pub fuel_max_ltr: f32,
    pub max_fuel_pct: f32,
    pub est_lap_time: f32,
    pub shift_rpm: f32,
    pub redline_rpm: f32,
    /// iRacing's own shift-light points: first light, last light, flashing.
    pub sl_first_rpm: f32,
    pub sl_last_rpm: f32,
    pub sl_blink_rpm: f32,
    pub tires: Vec<TireDef>,
    pub sectors: Vec<f32>,
    /// Qualifying order carried into the race (QualifyResultsInfo), in finishing order.
    pub qualify: Vec<ResultPos>,
}

impl SessionInfo {
    pub fn parse(src: &str) -> Self {
        Self::from_yaml(&yaml::parse(src))
    }

    pub fn from_yaml(y: &Yaml) -> Self {
        let wi = y.get("WeekendInfo");
        let di = y.get("DriverInfo");
        let unlimited = |v: &Yaml| v.str().map(|s| s.starts_with("unlimited")).unwrap_or(true);
        let sessions = y
            .path("SessionInfo.Sessions")
            .list()
            .iter()
            .map(|s| SessionDef {
                num: s.get("SessionNum").i32_or(0),
                kind: s.get("SessionType").string(),
                name: s.get("SessionName").string(),
                rubber: s.get("SessionTrackRubberState").string(),
                laps: (!unlimited(s.get("SessionLaps"))).then(|| s.get("SessionLaps").i32_or(0)),
                time: (!unlimited(s.get("SessionTime"))).then(|| s.get("SessionTime").f64().unwrap_or(0.0)),
                results: s
                    .get("ResultsPositions")
                    .list()
                    .iter()
                    .map(|r| ResultPos {
                        position: r.get("Position").i32_or(0),
                        class_position: r.get("ClassPosition").i32_or(-1) + 1,
                        car_idx: r.get("CarIdx").i32_or(0).max(0) as usize,
                        laps_complete: r.get("LapsComplete").i32_or(0),
                        fastest_time: r.get("FastestTime").f32_or(-1.0),
                        last_time: r.get("LastTime").f32_or(-1.0),
                        incidents: r.get("Incidents").i32_or(0),
                        reason_out: r.get("ReasonOutStr").string(),
                    })
                    .collect(),
            })
            .collect();
        let drivers = di
            .get("Drivers")
            .list()
            .iter()
            .map(|d| Driver {
                car_idx: d.get("CarIdx").i32_or(0).max(0) as usize,
                name: d.get("UserName").string(),
                abbrev: d.get("AbbrevName").string(),
                initials: d.get("Initials").string(),
                user_id: d.get("UserID").i64().unwrap_or(-1),
                team_id: d.get("TeamID").i64().unwrap_or(0),
                team_name: d.get("TeamName").string(),
                car_number: d.get("CarNumber").string(),
                car_number_raw: d.get("CarNumberRaw").i32_or(-1),
                car_id: d.get("CarID").i64().unwrap_or(0),
                car_path: d.get("CarPath").string(),
                car_name: d.get("CarScreenName").string(),
                car_short: d.get("CarScreenNameShort").string(),
                class_id: d.get("CarClassID").i64().unwrap_or(0),
                class_name: d.get("CarClassShortName").string(),
                class_color: d.get("CarClassColor").i64().unwrap_or(0xffffff) as u32,
                class_est_lap: d.get("CarClassEstLapTime").f32_or(0.0),
                irating: d.get("IRating").i32_or(0),
                lic_string: d.get("LicString").string(),
                lic_color: d.get("LicColor").i64().unwrap_or(0xffffff) as u32,
                club: d.get("ClubName").string(),
                flair: match (d.get("FlairName").str(), d.get("FlairID").i64()) {
                    (Some(n), Some(id)) => format!("{n} (#{id})"),
                    (Some(n), None) => n.to_string(),
                    (None, Some(id)) => format!("flair #{id}"),
                    (None, None) => String::new(),
                },
                // iRacing has used several flair fields; take the first one that names a country.
                country: crate::lookup::driver_country(
                    d.get("FlairCountryCode").str().or(d.get("CountryCode").str()).unwrap_or(""),
                    &d.get("FlairShortName").string(),
                    d.get("FlairName").str().or(d.get("Country").str()).unwrap_or(""),
                    d.get("FlairID").i64().unwrap_or(0),
                )
                .to_string(),
                make: crate::lookup::make(&d.get("CarScreenName").string(), &d.get("CarPath").string()),
                is_spectator: d.get("IsSpectator").i32_or(0) != 0,
                is_pace_car: d.get("CarIsPaceCar").i32_or(0) != 0,
                is_ai: d.get("CarIsAI").i32_or(0) != 0,
                incidents: d.get("CurDriverIncidentCount").i32_or(0),
            })
            .collect();
        SessionInfo {
            track_id: wi.get("TrackID").i64().unwrap_or(0),
            track_name: wi.get("TrackName").string(),
            track_display_name: wi.get("TrackDisplayName").string(),
            track_config: wi.get("TrackConfigName").string(),
            track_length_km: wi.get("TrackLength").f32_or(0.0),
            track_city: wi.get("TrackCity").string(),
            track_country: wi.get("TrackCountry").string(),
            series_id: wi.get("SeriesID").i64().unwrap_or(0),
            official: wi.get("Official").i32_or(0) != 0,
            subsession_id: wi.get("SubSessionID").i64().unwrap_or(0),
            league_id: wi.get("LeagueID").i64().unwrap_or(0),
            event_type: wi.get("EventType").string(),
            incident_limit: wi.path("WeekendOptions.IncidentLimit").i64().map(|v| v as i32),
            pit_speed_limit_kph: {
                let s = wi.get("TrackPitSpeedLimit").string();
                let v = wi.get("TrackPitSpeedLimit").f32_or(0.0);
                if s.contains("mph") {
                    v * 1.609_344
                } else {
                    v
                }
            },
            sessions,
            drivers,
            player_car_idx: di.get("DriverCarIdx").i32_or(-1),
            pace_car_idx: di.get("PaceCarIdx").i32_or(-1),
            fuel_max_ltr: di.get("DriverCarFuelMaxLtr").f32_or(0.0),
            max_fuel_pct: di.get("DriverCarMaxFuelPct").f32_or(1.0),
            est_lap_time: di.get("DriverCarEstLapTime").f32_or(0.0),
            shift_rpm: di.get("DriverCarSLShiftRPM").f32_or(0.0),
            redline_rpm: di.get("DriverCarRedLine").f32_or(0.0),
            sl_first_rpm: di.get("DriverCarSLFirstRPM").f32_or(0.0),
            sl_last_rpm: di.get("DriverCarSLLastRPM").f32_or(0.0),
            sl_blink_rpm: di.get("DriverCarSLBlinkRPM").f32_or(0.0),
            tires: di
                .get("DriverTires")
                .list()
                .iter()
                .map(|t| TireDef { index: t.get("TireIndex").i32_or(0), kind: t.get("TireCompoundType").string() })
                .collect(),
            sectors: y.path("SplitTimeInfo.Sectors").list().iter().map(|s| s.get("SectorStartPct").f32_or(0.0)).collect(),
            qualify: y
                .path("QualifyResultsInfo.Results")
                .list()
                .iter()
                .map(|r| ResultPos {
                    position: r.get("Position").i32_or(0) + 1,
                    class_position: r.get("ClassPosition").i32_or(0) + 1,
                    car_idx: r.get("CarIdx").i32_or(0).max(0) as usize,
                    fastest_time: r.get("FastestTime").f32_or(-1.0),
                    ..Default::default()
                })
                .collect(),
        }
    }

    pub fn session(&self, num: i32) -> Option<&SessionDef> {
        self.sessions.iter().find(|s| s.num == num)
    }

    pub fn driver(&self, car_idx: usize) -> Option<&Driver> {
        self.drivers.iter().find(|d| d.car_idx == car_idx)
    }

    pub fn fuel_capacity(&self) -> f32 {
        self.fuel_max_ltr * if self.max_fuel_pct > 0.0 { self.max_fuel_pct } else { 1.0 }
    }

    pub fn tire_is_wet(&self, compound: i32) -> Option<bool> {
        if self.tires.is_empty() {
            return None;
        }
        self.tires.iter().find(|t| t.index == compound).map(|t| t.is_wet())
    }

    /// Wet (true) or dry (false) for the relative/standings tyre column. Anything that is not a
    /// known wet compound counts as dry (cars without wets often report no compound list at all).
    pub fn tire_shown(&self, c: &CarTel) -> Option<bool> {
        c.in_world().then(|| self.tire_is_wet(c.tire_compound).unwrap_or(false))
    }

    pub fn has_wet_tires(&self) -> bool {
        self.tires.iter().any(|t| t.is_wet())
    }
}

impl Driver {
    /// Licence class letter and safety rating from e.g. "A 3.45".
    pub fn licence(&self) -> (char, f32) {
        let mut it = self.lic_string.split_whitespace();
        let class = it.next().and_then(|c| c.chars().next()).unwrap_or('R');
        (class, it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0))
    }
}

pub fn wetness_label(w: i32) -> &'static str {
    match w {
        1 => "Dry",
        2 => "Mostly dry",
        3 => "Very lightly wet",
        4 => "Lightly wet",
        5 => "Moderately wet",
        6 => "Very wet",
        7 => "Extremely wet",
        _ => "Unknown",
    }
}

pub fn skies_label(s: i32) -> &'static str {
    match s {
        0 => "Clear",
        1 => "Partly cloudy",
        2 => "Mostly cloudy",
        3 => "Overcast",
        _ => "Unknown",
    }
}

pub fn session_state_label(s: i32) -> &'static str {
    match s {
        1 => "Get in car",
        2 => "Warmup",
        3 => "Parade laps",
        4 => "Racing",
        5 => "Checkered",
        6 => "Cool down",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countries_from_flair() {
        // As sent by iRacing (2026): FlairName / FlairID per driver.
        let y = "DriverInfo:\n DriverCarIdx: 0\n Drivers:\n - CarIdx: 0\n   UserName: A\n   ClubName: None\n   FlairName: Antarctica\n   FlairID: 11\n - CarIdx: 1\n   UserName: B\n   FlairName: Global\n   FlairID: 2\n - CarIdx: 2\n   UserName: C\n   FlairName: Poland\n   FlairID: 167\n";
        let s = SessionInfo::parse(y);
        let c: Vec<&str> = s.drivers.iter().map(|d| d.country.as_str()).collect();
        assert_eq!(c, ["AQ", "GLOBE", "PL"]);
    }

    #[test]
    fn parses_session_info() {
        let s = SessionInfo::parse(
            "WeekendInfo:\n TrackID: 9\n TrackDisplayName: Road Atlanta\n TrackLength: 4.08 km\n WeekendOptions:\n  IncidentLimit: unlimited\nSessionInfo:\n Sessions:\n - SessionNum: 0\n   SessionLaps: 20\n   SessionTime: unlimited\n   SessionType: Race\n   SessionTrackRubberState: moderately high usage\nDriverInfo:\n DriverCarIdx: 2\n DriverCarFuelMaxLtr: 100.0\n DriverCarMaxFuelPct: 0.5\n DriverTires:\n - TireIndex: 0\n   TireCompoundType: \"Hard\"\n - TireIndex: 1\n   TireCompoundType: \"Wet\"\n Drivers:\n - CarIdx: 2\n   UserName: Me\n   CarClassColor: 0xff0000\n",
        );
        assert_eq!(s.track_id, 9);
        assert_eq!(s.incident_limit, None);
        assert_eq!(s.sessions[0].laps, Some(20));
        assert_eq!(s.sessions[0].time, None);
        assert!(s.sessions[0].is_race());
        assert_eq!(s.sessions[0].rubber, "moderately high usage");
        assert_eq!(s.fuel_capacity(), 50.0);
        assert_eq!(s.tire_is_wet(1), Some(true));
        assert_eq!(s.tire_is_wet(0), Some(false));
        assert_eq!(s.driver(2).unwrap().class_color, 0xff0000);
    }
}
