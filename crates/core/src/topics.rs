//! Topic payloads streamed to overlays. Every topic is serialised at most once
//! per tick and only while at least one client subscribes to it; unchanged
//! payloads are not re-sent.

use crate::engine::{battle, fuel, tyres, Engine, Row, Static, Stint};
use crate::model::{self, flags, surface};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Topic {
    Session,
    Player,
    Cars,
    Relative,
    Standings,
    Fuel,
    Battle,
    Tyres,
    TrackMap,
    Car,
    Spotter,
    Incidents,
    Pit,
    Safety,
    Delta,
}

impl Topic {
    pub const ALL: [Topic; 15] = [
        Topic::Session,
        Topic::Player,
        Topic::Cars,
        Topic::Relative,
        Topic::Standings,
        Topic::Fuel,
        Topic::Battle,
        Topic::Tyres,
        Topic::TrackMap,
        Topic::Car,
        Topic::Spotter,
        Topic::Incidents,
        Topic::Pit,
        Topic::Safety,
        Topic::Delta,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Topic::Session => "session",
            Topic::Player => "player",
            Topic::Cars => "cars",
            Topic::Relative => "relative",
            Topic::Standings => "standings",
            Topic::Fuel => "fuel",
            Topic::Battle => "battle",
            Topic::Tyres => "tyres",
            Topic::TrackMap => "trackmap",
            Topic::Car => "car",
            Topic::Spotter => "spotter",
            Topic::Incidents => "incidents",
            Topic::Pit => "pit",
            Topic::Safety => "safety",
            Topic::Delta => "delta",
        }
    }

    /// Publish every n-th telemetry tick (60 Hz). Driving aids follow every tick; timing
    /// tables can't be read faster than a few times a second, and each window parses all
    /// it is sent.
    pub fn every(self) -> u64 {
        match self {
            Topic::Player | Topic::Cars | Topic::Spotter => 1,
            Topic::Pit => 2,
            Topic::Relative | Topic::Safety => 3,
            Topic::Standings | Topic::Battle => 6,
            Topic::Session | Topic::Fuel | Topic::Tyres | Topic::Incidents => 15,
            // Only re-sent when they change (see `publish`).
            Topic::TrackMap | Topic::Car | Topic::Delta => 1,
        }
    }

    /// The "fast" update rate, for the least CPU: nothing above 30/s, timing tables as in
    /// balanced. Telemetry is still read at 60/s, so nothing is missed, only shown less often.
    pub fn every_fast(self) -> u64 {
        match self {
            Topic::Player | Topic::Cars | Topic::Spotter | Topic::Pit => 2,
            Topic::Relative | Topic::Safety => 3,
            t => t.every(),
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.name() == s)
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap()
    }

    /// Wrap a JSON payload into the wire envelope `{"t":..,"d":..}`.
    pub fn envelope(name: &str, json: &str) -> String {
        format!("{{\"t\":\"{name}\",\"d\":{json}}}")
    }

    pub fn build(self, e: &Engine) -> String {
        let r = match self {
            Topic::Session => serde_json::to_string(&session(e)),
            Topic::Player => serde_json::to_string(&player(e)),
            Topic::Cars => serde_json::to_string(&cars(e)),
            Topic::Relative => serde_json::to_string(&relative(e)),
            Topic::Standings => serde_json::to_string(&standings(e)),
            Topic::Fuel => serde_json::to_string(&fuel_out(e)),
            Topic::Battle => serde_json::to_string(&battle_out(e)),
            Topic::Tyres => serde_json::to_string(&tyres::advise(e)),
            Topic::TrackMap => serde_json::to_string(&trackmap(e)),
            Topic::Car => serde_json::to_string(&car(e)),
            Topic::Spotter => serde_json::to_string(&spotter(e)),
            Topic::Incidents => serde_json::to_string(&incidents_out(e)),
            Topic::Pit => serde_json::to_string(&e.pit_out),
            Topic::Safety => serde_json::to_string(&crate::safety::build(e)),
            Topic::Delta => serde_json::to_string(&DeltaOut { session: e.laps.session.as_ref(), all_time: e.laps.all_time.as_ref() }),
        };
        r.unwrap_or_else(|_| "null".into())
    }
}

/// Your own reference laps for the delta bar (Garage 61's come on the `garage61` topic).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaOut<'a> {
    pub session: Option<&'a crate::garage61::Reference>,
    pub all_time: Option<&'a crate::garage61::Reference>,
}

fn round(v: f32, digits: i32) -> f32 {
    let m = 10f32.powi(digits);
    (v * m).round() / m
}
fn r3(v: f32) -> f32 {
    round(v, 3)
}
fn opt_time(v: f32) -> Option<f32> {
    (v > 0.0).then(|| r3(v))
}
pub fn hex(c: u32) -> String {
    format!("#{:06x}", c & 0xffffff)
}

fn units(e: &Engine) -> &'static str {
    match e.cfg.units.as_str() {
        "metric" => "metric",
        "imperial" => "imperial",
        _ if e.tel.display_metric => "metric",
        _ => "imperial",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassOut {
    pub id: String,
    pub name: String,
    pub color: String,
    pub count: usize,
    pub sof: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOut<'a> {
    pub connected: bool,
    pub source: &'a str,
    pub units: &'a str,
    pub track_id: i64,
    pub track_name: &'a str,
    pub track_config: &'a str,
    pub track_length_km: f32,
    pub track_city: &'a str,
    pub track_country: &'a str,
    pub session_type: &'a str,
    pub session_name: &'a str,
    pub session_state: &'a str,
    /// Rubber on the track (iRacing's words, e.g. "moderately high usage"; "" if unknown).
    pub track_state: &'a str,
    pub is_race: bool,
    pub time_remain: Option<f32>,
    pub time_total: Option<f32>,
    pub laps_remain: Option<i32>,
    pub laps_total: Option<i32>,
    pub leader_lap: i32,
    pub time_of_day: f32,
    pub track_temp: f32,
    pub track_temp_delta: f32,
    pub air_temp: f32,
    pub wetness: i32,
    pub wetness_label: &'a str,
    pub precipitation: f32,
    pub declared_wet: bool,
    pub weather_trend: &'a str,
    pub wind_speed: f32,
    pub wind_dir: f32,
    pub humidity: f32,
    pub fog: f32,
    pub skies: &'a str,
    pub air_pressure: f32,
    pub flags: Vec<&'static str>,
    pub sof: i32,
    pub classes: Vec<ClassOut>,
    pub incidents: i32,
    pub incident_limit: Option<i32>,
    pub num_cars: usize,
    pub league: Option<&'a str>,
    /// iRacing league of the current session (0 if not a league session).
    pub league_id: i64,
    pub official: bool,
    pub subsession_id: i64,
}

pub fn session(e: &Engine) -> SessionOut<'_> {
    let t = &e.tel;
    let def = e.session_def();
    let irs: Vec<i32> = e.order.iter().filter_map(|r| e.static_of(r.idx)).map(|s| s.irating).collect();
    let time_total = def.and_then(|d| d.time).map(|v| v as f32);
    SessionOut {
        connected: e.connected,
        source: e.source,
        units: units(e),
        track_id: e.info.track_id,
        track_name: &e.info.track_display_name,
        track_config: &e.info.track_config,
        track_length_km: e.info.track_length_km,
        track_city: &e.info.track_city,
        track_country: &e.info.track_country,
        session_type: def.map(|d| d.kind.as_str()).unwrap_or(""),
        session_name: def.map(|d| d.name.as_str()).unwrap_or(""),
        track_state: def.map(|d| d.rubber.as_str()).unwrap_or(""),
        session_state: model::session_state_label(t.session_state),
        is_race: e.is_race(),
        time_remain: (time_total.is_some() && t.session_time_remain >= 0.0 && t.session_time_remain < 1e6).then(|| round(t.session_time_remain as f32, 1)),
        time_total,
        laps_remain: (def.and_then(|d| d.laps).is_some() && t.session_laps_remain >= 0 && t.session_laps_remain < 30000).then_some(t.session_laps_remain),
        laps_total: def.and_then(|d| d.laps),
        leader_lap: e.order.first().map(|r| t.cars[r.idx].lap).unwrap_or(0),
        time_of_day: t.time_of_day.round(),
        track_temp: round(t.track_temp, 1),
        track_temp_delta: round(e.weather.track_temp_delta(), 1),
        air_temp: round(t.air_temp, 1),
        wetness: t.track_wetness,
        wetness_label: model::wetness_label(t.track_wetness),
        precipitation: round(t.precipitation, 2),
        declared_wet: t.declared_wet,
        weather_trend: e.weather.trend().as_str(),
        wind_speed: round(t.wind_vel, 1),
        wind_dir: round(t.wind_dir.to_degrees().rem_euclid(360.0), 0),
        humidity: round(t.humidity, 2),
        fog: round(t.fog, 2),
        skies: model::skies_label(t.skies),
        air_pressure: round(t.air_pressure, 1),
        flags: flags::names(t.session_flags),
        sof: crate::engine::sof(&irs),
        classes: e
            .classes
            .iter()
            .map(|c| ClassOut { id: c.key.id.clone(), name: c.key.name.clone(), color: hex(c.key.color), count: c.count, sof: c.sof })
            .collect(),
        incidents: t.incidents,
        incident_limit: e.info.incident_limit,
        num_cars: e.order.len(),
        league: e.resolver.league_name(),
        league_id: e.info.league_id,
        official: e.info.official,
        subsession_id: e.info.subsession_id,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerOut {
    pub on_track: bool,
    pub speed: f32,
    pub rpm: f32,
    pub gear: i32,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub steering: f32,
    pub steering_max: f32,
    pub abs: bool,
    pub shift_rpm: f32,
    pub redline: f32,
    pub lap: i32,
    pub lap_pct: f32,
    pub lap_time: f32,
    pub last_lap: Option<f32>,
    pub best_lap: Option<f32>,
    pub delta_best: Option<f32>,
    pub fuel: f32,
    pub on_pit_road: bool,
    pub position: i32,
    pub class_position: i32,
    pub incidents: i32,
    pub lat_g: f32,
    pub long_g: f32,
    /// Compass heading in degrees of the car being followed (the player, or the camera car
    /// when spectating / in a replay). None when it cannot be known yet.
    pub heading: Option<f32>,
}

pub fn player(e: &Engine) -> PlayerOut {
    let t = &e.tel;
    let row = usize::try_from(t.player_idx).ok().and_then(|i| e.row(i));
    PlayerOut {
        on_track: t.is_on_track,
        speed: round(t.speed, 2),
        rpm: t.rpm.round(),
        gear: t.gear,
        throttle: r3(t.throttle),
        brake: r3(t.brake),
        clutch: r3(t.clutch),
        steering: r3(t.steering),
        steering_max: r3(t.steering_max),
        abs: t.abs_active,
        shift_rpm: e.info.shift_rpm,
        redline: e.info.redline_rpm,
        lap: t.lap,
        lap_pct: round(t.lap_dist_pct, 4),
        lap_time: r3(t.lap_current_time),
        last_lap: opt_time(t.lap_last_time),
        best_lap: opt_time(t.lap_best_time),
        delta_best: t.delta_best_ok.then(|| r3(t.delta_best)),
        fuel: r3(t.fuel_level),
        on_pit_road: t.on_pit_road,
        position: row.map(|r| r.pos).unwrap_or(0),
        class_position: row.map(|r| r.class_pos).unwrap_or(0),
        incidents: t.incidents,
        lat_g: round(t.lat_accel / 9.81, 2),
        long_g: round(t.long_accel / 9.81, 2),
        heading: e.focus_heading().map(|h| round(h, 1)),
    }
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct SpotterOut {
    /// "left", "right", "both", "left2" (two cars left), "right2", or "" when clear.
    pub state: &'static str,
    /// How far each car alongside is ahead of you (metres, negative = behind), when iRacing's
    /// report says which side it is on. Empty for "both" or an unknown track length.
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

pub fn spotter(e: &Engine) -> SpotterOut {
    let t = &e.tel;
    let state = match t.car_left_right {
        2 => "left",
        3 => "right",
        4 => "both",
        5 => "left2",
        6 => "right2",
        _ => "",
    };
    if state.is_empty() || !t.driving() {
        return SpotterOut::default();
    }
    let near = crate::engine::incidents::near_player(t, e.info.track_length_km * 1000.0, 12.0);
    let take = |n: usize| near.iter().take(n).map(|n| round(n.1, 1)).collect::<Vec<_>>();
    let (left, right) = match state {
        "left" => (take(1), vec![]),
        "right" => (vec![], take(1)),
        "left2" => (take(2), vec![]),
        "right2" => (vec![], take(2)),
        _ => (vec![], vec![]),
    };
    SpotterOut { state, left, right }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentsOut<'a> {
    pub total: i32,
    pub limit: Option<i32>,
    /// A replay is playing, so the replay buttons can jump to an incident.
    pub can_replay: bool,
    pub session_num: i32,
    pub list: &'a [crate::engine::incidents::Incident],
}

pub fn incidents_out(e: &Engine) -> IncidentsOut<'_> {
    IncidentsOut {
        total: e.tel.incidents,
        limit: e.info.incident_limit,
        can_replay: e.tel.is_replay_playing,
        session_num: e.tel.session_num,
        list: &e.incidents.list,
    }
}

#[derive(Serialize)]
pub struct CarDot<'a> {
    pub i: usize,
    pub p: f32,
    pub n: &'a str,
    pub c: String,
    pub cls: &'a str,
    pub pos: i32,
    pub cpos: i32,
    pub pit: bool,
    pub off: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarsOut<'a> {
    pub focus: Option<usize>,
    pub player: Option<usize>,
    pub sectors: &'a [f32],
    pub cars: Vec<CarDot<'a>>,
    /// Sector times of the focus car for the track map's sector colours.
    pub sector_times: SectorTimesOut<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorTimesOut<'a> {
    pub starts: &'a [f32],
    pub last: &'a [Option<crate::engine::sectors::SectorTime>],
    pub lap: u32,
    /// Seconds since the focus car last crossed the line.
    pub lap_end_ago: Option<f64>,
}

pub fn cars(e: &Engine) -> CarsOut<'_> {
    let t = &e.tel;
    CarsOut {
        focus: t.focus_idx(),
        player: usize::try_from(t.player_idx).ok(),
        sectors: &e.info.sectors,
        sector_times: SectorTimesOut {
            starts: &e.sectors.starts,
            last: &e.sectors.last,
            lap: e.sectors.lap,
            lap_end_ago: e.sectors.lap_end_at.map(|at| round((t.session_time - at) as f32, 1) as f64),
        },
        cars: e
            .order
            .iter()
            .filter(|r| t.cars[r.idx].in_world())
            .filter_map(|r| {
                let s = e.static_of(r.idx)?;
                let c = &t.cars[r.idx];
                Some(CarDot {
                    i: r.idx,
                    p: round(c.pct, 4),
                    n: &s.number,
                    c: hex(s.class.color),
                    cls: &s.class.id,
                    pos: r.pos,
                    cpos: r.class_pos,
                    pit: c.on_pit_road,
                    off: c.surface == surface::OFF_TRACK,
                })
            })
            .collect(),
    }
}

/// Fields shared by relative and standings rows.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverOut<'a> {
    pub idx: usize,
    pub number: &'a str,
    pub name: &'a str,
    pub abbrev: &'a str,
    pub team: &'a str,
    pub car: &'a str,
    pub class_id: &'a str,
    pub class_name: &'a str,
    pub class_color: String,
    pub user_id: i64,
    pub car_id: i64,
    pub irating: i32,
    pub license: &'a str,
    pub license_color: String,
    pub club: &'a str,
    /// The flair iRacing sent (country flag choice), raw.
    pub flair: &'a str,
    /// ISO country code for the flag ("" if unknown).
    pub country: &'a str,
    pub make: &'a str,
    pub is_player: bool,
    pub is_focus: bool,
}

pub fn driver<'a>(e: &Engine, s: &'a Static) -> DriverOut<'a> {
    DriverOut {
        idx: s.idx,
        number: &s.number,
        name: &s.name,
        abbrev: &s.abbrev,
        team: &s.team,
        car: &s.car,
        class_id: &s.class.id,
        class_name: &s.class.name,
        class_color: hex(s.class.color),
        user_id: s.user_id,
        car_id: s.car_id,
        irating: s.irating,
        license: &s.lic,
        license_color: hex(s.lic_color),
        club: &s.club,
        flair: &s.flair,
        country: &s.country,
        make: &s.make,
        is_player: s.idx as i32 == e.tel.player_idx,
        is_focus: Some(s.idx) == e.tel.focus_idx(),
    }
}

/// Per-car extras shared by relative and standings rows.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Extras {
    /// Current race lap.
    pub lap: i32,
    /// "pit" | "out" | "lap"
    pub stint_state: &'static str,
    pub stint_lap: Option<i32>,
    pub best_lap: Option<f32>,
    pub last_lap: Option<f32>,
    /// Best lap is the fastest of the class.
    pub class_best: bool,
    /// Last lap was this driver's best so far (personal best).
    pub last_is_pb: bool,
    /// Has taken the chequered flag.
    pub finished: bool,
    pub laps_led: i32,
    /// Flags for this car: "yellow" (caused one), "black", "disqualify", "repair", "slow"
    /// (slow-down penalty, iRacing's furled black flag), "blue".
    pub flags: Vec<&'static str>,
    /// "DT", "S&G 40s", "+5s" ... and whether it has just been served.
    pub penalty: Option<String>,
    pub penalty_served: bool,
    pub incidents: Option<i32>,
    /// Estimated iRating change if the race finished now (official races).
    pub ir_delta: Option<i32>,
    /// Rough estimated safety-rating change this session.
    pub sr_delta: Option<f32>,
    /// League points for the current position and the projected championship total.
    pub points: Option<f32>,
    pub season_points: Option<f32>,
    /// Serving a slow-down penalty: for how long, and the time given up so far (estimated).
    pub slowdown: Option<SlowdownOut>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct SlowdownOut {
    pub seconds: f32,
    pub lost: f32,
}

pub fn extras(e: &Engine, idx: usize) -> Extras {
    let c = &e.tel.cars[idx];
    let (stint_state, stint_lap) = match e.stint(idx) {
        Stint::Pit => ("pit", None),
        Stint::OutLap => ("out", None),
        Stint::Lap(n) => ("lap", Some(n)),
    };
    let (points, season_points) = e.points(idx);
    let penalty = e.penalty(idx);
    let best = e.best_lap(idx);
    let last = e.last_lap(idx);
    let class_id = e.static_of(idx).map(|s| s.class.id.as_str());
    let class_best = e.classes.iter().find(|k| Some(k.key.id.as_str()) == class_id).and_then(|k| k.best_lap).map(|b| b.1);
    let ir_ok = e.info.official || !e.cfg.ratings_official_only;
    Extras {
        lap: c.lap.max(0),
        stint_state,
        stint_lap,
        best_lap: best.map(r3),
        last_lap: last.map(r3),
        class_best: best.is_some() && class_best == best,
        last_is_pb: matches!((last, best), (Some(l), Some(b)) if (l - b).abs() < 1e-3),
        finished: e.hist[idx].finished,
        laps_led: e.hist[idx].laps_led_overall,
        flags: e.car_flags(idx),
        penalty: penalty.as_ref().map(|p| p.0.clone()),
        penalty_served: penalty.is_some_and(|p| p.1),
        incidents: e.incidents_visible(idx).then(|| e.incidents(idx)),
        ir_delta: if ir_ok { e.ir_delta(idx).map(|v| v.round() as i32) } else { None },
        sr_delta: if ir_ok { e.sr_delta(idx).map(|v| round(v, 2)) } else { None },
        points,
        season_points,
        slowdown: e.hist[idx]
            .slowdown_since
            .map(|since| SlowdownOut { seconds: round((e.tel.session_time - since) as f32, 1), lost: round(e.hist[idx].slowdown_lost, 1) }),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelRow<'a> {
    #[serde(flatten)]
    pub d: DriverOut<'a>,
    #[serde(flatten)]
    pub x: Extras,
    /// On-track gap to the focus car (positive = ahead).
    pub gap: f32,
    /// Race gap to the class leader and to the car ahead in class.
    pub gap_leader: Option<f32>,
    pub gap_leader_laps: i32,
    pub interval: Option<f32>,
    pub interval_laps: i32,
    pub position: i32,
    pub class_position: i32,
    /// +1 = lapping you (a lap ahead), -1 = you are lapping them.
    pub lap_diff: i32,
    pub on_pit_road: bool,
    pub off_track: bool,
    pub tire_wet: Option<bool>,
}

#[derive(Serialize)]
pub struct RelativeOut<'a> {
    pub rows: Vec<RelRow<'a>>,
}

pub fn relative(e: &Engine) -> RelativeOut<'_> {
    let t = &e.tel;
    let Some(focus) = t.focus_idx() else { return RelativeOut { rows: vec![] } };
    let race = e.is_race();
    let fprog = e.progress(focus);
    let mut rows: Vec<RelRow> = e
        .order
        .iter()
        .filter(|r| t.cars[r.idx].in_world())
        .filter_map(|r: &Row| {
            let s = e.static_of(r.idx)?;
            let c = &t.cars[r.idx];
            let gap = if r.idx == focus { 0.0 } else { e.relative_gap(focus, r.idx) };
            let mut dp = c.pct - t.cars[focus].pct;
            if dp > 0.5 {
                dp -= 1.0;
            } else if dp < -0.5 {
                dp += 1.0;
            }
            let lap_diff = if race { (c.progress() - fprog - dp as f64).round() as i32 } else { 0 };
            Some(RelRow {
                d: driver(e, s),
                x: extras(e, r.idx),
                gap: r3(gap),
                gap_leader: r.gap.map(r3),
                gap_leader_laps: r.gap_laps,
                interval: r.interval.map(r3),
                interval_laps: r.interval_laps,
                position: r.pos,
                class_position: r.class_pos,
                lap_diff,
                on_pit_road: c.on_pit_road,
                off_track: c.surface == surface::OFF_TRACK,
                tire_wet: e.info.tire_shown(c),
            })
        })
        .collect();
    rows.sort_by(|a, b| b.gap.total_cmp(&a.gap));
    let me = rows.iter().position(|r| r.d.idx == focus).unwrap_or(0);
    const SIDE: usize = 10;
    let start = me.saturating_sub(SIDE);
    let end = (me + SIDE + 1).min(rows.len());
    RelativeOut { rows: rows.drain(start..end).collect() }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandRow<'a> {
    #[serde(flatten)]
    pub d: DriverOut<'a>,
    #[serde(flatten)]
    pub x: Extras,
    pub position: i32,
    pub class_position: i32,
    pub gap: Option<f32>,
    pub gap_laps: i32,
    pub interval: Option<f32>,
    pub interval_laps: i32,
    /// Gap and interval overall (to P1 and the car ahead), for a single overall list.
    pub gap_overall: Option<f32>,
    pub gap_overall_laps: i32,
    pub interval_overall: Option<f32>,
    pub interval_overall_laps: i32,
    pub is_class_best: bool,
    pub laps: i32,
    pub on_pit_road: bool,
    pub in_world: bool,
    pub pit_stops: i32,
    pub stint_laps: i32,
    pub positions_gained: Option<i32>,
    pub tire_wet: Option<bool>,
    pub progress: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandClass<'a> {
    #[serde(flatten)]
    pub class: ClassOut,
    pub best_lap: Option<f32>,
    pub rows: Vec<StandRow<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingsOut<'a> {
    pub is_race: bool,
    pub focus_class: Option<&'a str>,
    pub classes: Vec<StandClass<'a>>,
}

pub fn standings(e: &Engine) -> StandingsOut<'_> {
    let t = &e.tel;
    let mut classes: Vec<StandClass> = e
        .classes
        .iter()
        .map(|c| StandClass {
            class: ClassOut { id: c.key.id.clone(), name: c.key.name.clone(), color: hex(c.key.color), count: c.count, sof: c.sof },
            best_lap: c.best_lap.map(|b| r3(b.1)),
            rows: vec![],
        })
        .collect();
    for r in &e.order {
        let Some(s) = e.static_of(r.idx) else { continue };
        let Some(ci) = e.classes.iter().position(|c| c.key.id == s.class.id) else { continue };
        let c = &t.cars[r.idx];
        let h = &e.hist[r.idx];
        classes[ci].rows.push(StandRow {
            d: driver(e, s),
            x: extras(e, r.idx),
            position: r.pos,
            class_position: r.class_pos,
            gap: r.gap.map(r3),
            gap_laps: r.gap_laps,
            interval: r.interval.map(r3),
            interval_laps: r.interval_laps,
            gap_overall: r.gap_overall.map(r3),
            gap_overall_laps: r.gap_overall_laps,
            interval_overall: r.interval_overall.map(r3),
            interval_overall_laps: r.interval_overall_laps,
            is_class_best: e.classes[ci].best_lap.is_some_and(|b| b.0 == r.idx),
            laps: c.lap_completed.max(0),
            on_pit_road: c.on_pit_road,
            in_world: c.in_world(),
            pit_stops: h.pit_stops,
            stint_laps: (c.lap - h.stint_start_lap).max(0),
            positions_gained: (h.start_class_pos > 0 && e.is_race()).then(|| h.start_class_pos - r.class_pos),
            tire_wet: e.info.tire_shown(c),
            progress: round(e.progress(r.idx) as f32, 3),
        });
    }
    StandingsOut { is_race: e.is_race(), focus_class: t.focus_idx().and_then(|f| e.static_of(f)).map(|s| s.class.id.as_str()), classes }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuelOut<'a> {
    pub level: f32,
    pub capacity: f32,
    pub pct: f32,
    pub per_lap_avg: Option<f32>,
    pub per_lap_last: Option<f32>,
    pub per_lap_max: Option<f32>,
    pub laps_sampled: usize,
    pub laps_in_tank: Option<f32>,
    pub use_per_hour: f32,
    pub laps_remaining: Option<f32>,
    pub lap_time: Option<f32>,
    pub fuel_to_finish: Option<f32>,
    pub fuel_to_add: Option<f32>,
    pub stops: Option<i32>,
    pub laps_per_tank: Option<f32>,
    pub target_per_lap: Option<f32>,
    pub save_per_lap: Option<f32>,
    pub window_opens_lap: Option<i32>,
    pub window_closes_lap: Option<i32>,
    pub margin_laps: f32,
    pub history: Vec<f32>,
    /// What this car used here in earlier sessions.
    pub expected: Option<&'a fuel::ComboRecord>,
    /// "session" (laps driven now) or "history" (earlier sessions, until enough laps now).
    pub estimate_from: &'static str,
}

pub fn race_clock(e: &Engine, focus: usize) -> fuel::RaceClock {
    let t = &e.tel;
    let def = e.session_def();
    let leader = e.order.first().map(|r| r.idx).unwrap_or(focus);
    let n = e.cfg.fuel.avg_laps;
    let earlier = e.fuel_history.get(&e.fuel_key()).map(|x| x.lap_time).filter(|t| *t > 0.0);
    let player_lap = e.fuel.lap_time(n).or(earlier).unwrap_or_else(|| e.ref_lap(focus));
    let race = e.is_race();
    fuel::RaceClock {
        laps_total: def.and_then(|d| d.laps).filter(|_| race),
        time_remaining: def.and_then(|d| d.time).map(|_| t.session_time_remain),
        leader_progress: if race { e.progress(leader) } else { e.progress(focus) },
        leader_lap: if race && leader != focus { e.ref_lap(leader) } else { player_lap },
        player_progress: e.progress(focus),
        player_lap,
    }
}

pub fn fuel_out(e: &Engine) -> FuelOut<'_> {
    let t = &e.tel;
    let cfg = &e.cfg.fuel;
    let n = cfg.avg_laps;
    let expected = e.fuel_history.get(&e.fuel_key());
    // Until a lap has been timed this session, plan with what the car used here before.
    let session_avg = e.fuel.average(n);
    let avg = session_avg.or(expected.map(|x| x.per_lap));
    let capacity = e.info.fuel_capacity();
    let focus = usize::try_from(t.player_idx).ok().filter(|&i| i < t.cars.len());
    let laps_left = focus.and_then(|f| fuel::laps_remaining(&race_clock(e, f))).map(|v| v as f32);
    let plan = match (avg, laps_left) {
        (Some(a), Some(l)) => Some(fuel::plan(t.fuel_level, capacity, a, l, cfg.margin_laps)),
        _ => None,
    };
    let lap = t.lap.max(0);
    FuelOut {
        level: r3(t.fuel_level),
        capacity: r3(capacity),
        pct: r3(t.fuel_level_pct),
        per_lap_avg: avg.map(r3),
        per_lap_last: e.fuel.laps.back().map(|l| r3(l.used)),
        per_lap_max: e.fuel.max(n).map(r3),
        laps_sampled: e.fuel.laps.iter().filter(|l| l.valid).count(),
        laps_in_tank: avg.filter(|a| *a > 0.0).map(|a| round(t.fuel_level / a, 2)),
        use_per_hour: round(t.fuel_use_per_hour, 2),
        laps_remaining: laps_left.map(|v| round(v, 2)),
        lap_time: e.fuel.lap_time(n).map(r3),
        fuel_to_finish: plan.as_ref().map(|p| round(p.fuel_to_finish, 2)),
        fuel_to_add: plan.as_ref().map(|p| round(p.fuel_to_add, 2)),
        stops: plan.as_ref().map(|p| p.stops),
        laps_per_tank: plan.as_ref().filter(|p| p.laps_per_tank > 0.0).map(|p| round(p.laps_per_tank, 1)),
        target_per_lap: plan.as_ref().filter(|p| p.target_per_lap > 0.0).map(|p| r3(p.target_per_lap)),
        save_per_lap: match (&plan, avg) {
            (Some(p), Some(a)) if p.stops >= 1 && p.target_per_lap > 0.0 => {
                // How much to save per lap to skip the last stop: fuel available over the
                // remaining stints vs. what is needed.
                let tank_after_stops = t.fuel_level + capacity * (p.stops - 1) as f32;
                let target = tank_after_stops / (laps_left.unwrap_or(0.0) + cfg.margin_laps);
                // Only worth showing when the saving is realistic (<= 12% of current use).
                let save = a - target;
                (save > 0.0 && save <= a * 0.12).then(|| r3(save))
            }
            _ => None,
        },
        window_opens_lap: plan.as_ref().filter(|p| p.stops > 0).map(|p| lap + p.window_opens_in.ceil() as i32),
        window_closes_lap: plan.as_ref().filter(|p| p.stops > 0).map(|p| lap + p.window_closes_in.floor() as i32),
        margin_laps: cfg.margin_laps,
        history: e.fuel.laps.iter().rev().filter(|l| l.valid).take(10).map(|l| r3(l.used)).collect(),
        expected,
        estimate_from: if session_avg.is_some() {
            "session"
        } else if expected.is_some() {
            "history"
        } else {
            "none"
        },
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleCar<'a> {
    #[serde(flatten)]
    pub d: DriverOut<'a>,
    pub position: i32,
    pub class_position: i32,
    pub gap: Option<f32>,
    pub gap_laps: i32,
    /// Seconds per lap the gap is changing (+ growing, - shrinking).
    pub trend: Option<f32>,
    /// Laps until the gap closes at the current rate.
    pub laps_to_catch: Option<f32>,
    pub last_lap: Option<f32>,
    pub best_lap: Option<f32>,
    pub on_pit_road: bool,
    pub tire_wet: Option<bool>,
    pub in_battle: bool,
}

#[derive(Serialize, Default)]
pub struct BattlePair<'a> {
    pub ahead: Option<BattleCar<'a>>,
    pub behind: Option<BattleCar<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleOut<'a> {
    /// Same class (position battle), overall order, and nearest on track.
    pub class: BattlePair<'a>,
    pub overall: BattlePair<'a>,
    pub track: BattlePair<'a>,
    pub my_last_lap: Option<f32>,
    pub my_best_lap: Option<f32>,
    pub max_gap: f32,
}

pub fn battle_out(e: &Engine) -> BattleOut<'_> {
    let t = &e.tel;
    let mut out = BattleOut {
        class: Default::default(),
        overall: Default::default(),
        track: Default::default(),
        my_last_lap: None,
        my_best_lap: None,
        max_gap: e.cfg.battle.max_gap,
    };
    let Some(focus) = t.focus_idx() else { return out };
    out.my_last_lap = e.last_lap(focus).map(r3);
    out.my_best_lap = e.best_lap(focus).map(r3);
    let lap = e.ref_lap(focus);
    let mk = |idx: usize, is_ahead: bool, mode: battle::Mode| -> Option<BattleCar> {
        let s = e.static_of(idx)?;
        let r = e.row(idx)?;
        let c = &t.cars[idx];
        let (gap, gap_laps) = battle::gap(e, focus, idx, is_ahead, mode);
        let hist = e.battle.get(mode, is_ahead);
        let trend = (hist.idx == Some(idx)).then(|| hist.slope()).flatten().map(|s| r3(s * lap));
        Some(BattleCar {
            d: driver(e, s),
            position: r.pos,
            class_position: r.class_pos,
            gap: gap.map(r3),
            gap_laps,
            trend,
            laps_to_catch: match (gap, trend) {
                (Some(g), Some(tr)) if tr < -0.01 && gap_laps == 0 => Some(round(g / -tr, 1)),
                _ => None,
            },
            last_lap: e.last_lap(idx).map(r3),
            best_lap: e.best_lap(idx).map(r3),
            on_pit_road: c.on_pit_road,
            tire_wet: e.info.tire_shown(c),
            in_battle: gap_laps == 0 && gap.is_some_and(|g| g <= e.cfg.battle.max_gap),
        })
    };
    let pair = |mode| {
        let (a, b) = battle::neighbours(e, focus, mode);
        BattlePair { ahead: a.and_then(|a| mk(a, true, mode)), behind: b.and_then(|b| mk(b, false, mode)) }
    };
    out.class = pair(battle::Mode::Class);
    out.overall = pair(battle::Mode::Overall);
    out.track = pair(battle::Mode::Track);
    out
}

/// The player's car: shift-light data (Lovely Car Data, else iRacing's own shift points).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarOut<'a> {
    pub car_path: &'a str,
    pub car_name: &'a str,
    /// "lovely" when per-gear shift-light data was found, else "iracing".
    pub source: &'a str,
    pub lovely: Option<&'a crate::cardata::CarData>,
    pub sl_first: f32,
    pub sl_shift: f32,
    pub sl_last: f32,
    pub sl_blink: f32,
    pub redline: f32,
}

pub fn car(e: &Engine) -> CarOut<'_> {
    let me = e.info.driver(e.info.player_car_idx.max(0) as usize);
    let path = me.map(|d| d.car_path.as_str()).unwrap_or("");
    let lovely = e.car_data.get(&path.to_lowercase());
    CarOut {
        car_path: path,
        car_name: me.map(|d| d.car_name.as_str()).unwrap_or(""),
        source: if lovely.is_some() { "lovely" } else { "iracing" },
        lovely,
        sl_first: e.info.sl_first_rpm,
        sl_shift: e.info.shift_rpm,
        sl_last: e.info.sl_last_rpm,
        sl_blink: e.info.sl_blink_rpm,
        redline: e.info.redline_rpm,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackMapOut<'a> {
    pub track_id: i64,
    pub track_name: &'a str,
    pub points: Option<&'a [[f32; 2]]>,
    pub source: Option<&'a str>,
    pub recording: bool,
    pub progress: f32,
    pub sectors: &'a [f32],
    pub corners: &'a [crate::config::Corner],
    /// "custom" | "lovely" | "mock" | "none"
    pub corners_source: &'a str,
    pub straights: &'a [crate::config::Corner],
    pub pit_entry: Option<f32>,
    pub pit_exit: Option<f32>,
}

pub fn trackmap(e: &Engine) -> TrackMapOut<'_> {
    TrackMapOut {
        track_id: e.info.track_id,
        track_name: &e.info.track_display_name,
        points: e.map.shown(e.cfg.track_outline != "recorded").map(|m| m.points.as_slice()),
        source: e.map.shown(e.cfg.track_outline != "recorded").map(|m| m.source.as_str()),
        recording: e.map.recording(),
        progress: round(e.map.progress(), 2),
        sectors: &e.info.sectors,
        corners: &e.corners,
        corners_source: e.corners_source,
        straights: e.track_data().map(|d| d.straights.as_slice()).unwrap_or(&[]),
        pit_entry: e.track_data().and_then(|d| d.pit_entry),
        pit_exit: e.track_data().and_then(|d| d.pit_exit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::{engine, place};

    #[test]
    fn every_topic_serialises() {
        let mut e = engine(6, true);
        for i in 0..6 {
            place(&mut e, i, 2, 0.1 * i as f32);
        }
        e.tick();
        for t in Topic::ALL {
            let s = t.build(&e);
            assert!(s.starts_with('{'), "{}: {s}", t.name());
        }
        let rel = relative(&e);
        assert_eq!(rel.rows.len(), 6);
        assert!(rel.rows.windows(2).all(|w| w[0].gap >= w[1].gap));
        let st = standings(&e);
        assert_eq!(st.classes.len(), 2);
        assert_eq!(st.classes.iter().map(|c| c.rows.len()).sum::<usize>(), 6);
    }

    #[test]
    fn spotter_reports_side_and_overlap() {
        let mut e = engine(4, true);
        e.info.track_length_km = 5.0;
        place(&mut e, 0, 2, 0.5);
        place(&mut e, 1, 2, 0.5006); // 3 m ahead
        place(&mut e, 2, 2, 0.4996); // 2 m behind
        place(&mut e, 3, 2, 0.6);
        e.tel.lap_dist_pct = 0.5;
        e.tel.is_on_track = true;
        e.tick();
        assert_eq!(spotter(&e), SpotterOut::default());
        e.tel.car_left_right = 2;
        assert_eq!(spotter(&e), SpotterOut { state: "left", left: vec![-2.0], right: vec![] });
        e.tel.car_left_right = 6;
        assert_eq!(spotter(&e), SpotterOut { state: "right2", left: vec![], right: vec![-2.0, 3.0] });
        e.tel.car_left_right = 4;
        assert_eq!(spotter(&e).state, "both");
        // Across the line.
        e.tel.lap_dist_pct = 0.9999;
        place(&mut e, 1, 3, 0.0005);
        place(&mut e, 2, 2, 0.2);
        e.tel.car_left_right = 3;
        assert_eq!(spotter(&e).right, vec![3.0]);
        // Spectating / replay: nothing.
        e.tel.is_replay_playing = true;
        assert_eq!(spotter(&e).state, "");
    }
}
