//! Topic payloads streamed to overlays. Every topic is serialised at most once
//! per tick and only while at least one client subscribes to it; unchanged
//! payloads are not re-sent.

use crate::engine::{battle, fuel, tyres, Engine, Row, Static, Stint};
use crate::model::{self, flags, surface};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Topic {
    /// Each car's fixed details (name, team, car, class, licence...), sent only when they
    /// change; the timing rows carry just the car index and the client fills them in.
    Drivers,
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
    Forecast,
    /// Your recent laps with their sector times (lap times overlay).
    Laptimes,
    /// Le Mans Ultimate's virtual energy: per lap, to the finish.
    Energy,
    /// Cars around you, in metres ahead and to the side.
    Radar,
    /// Settings changed from the wheel: brake bias, ABS, TC, engine map...
    CarSettings,
    /// Tyre temperatures, pressures and wear, and brake temperatures.
    TyreData,
    /// Where you would rejoin if you pitted now.
    PitRejoin,
    /// Your last lap, corner by corner, against your best.
    CornerTimes,
}

impl Topic {
    // Drivers first: when they change, they reach the clients before the rows that use them.
    pub const ALL: [Topic; 24] = [
        Topic::Drivers,
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
        Topic::Forecast,
        Topic::Laptimes,
        Topic::Energy,
        Topic::Radar,
        Topic::CarSettings,
        Topic::TyreData,
        Topic::PitRejoin,
        Topic::CornerTimes,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Topic::Drivers => "drivers",
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
            Topic::Forecast => "forecast",
            Topic::Laptimes => "laptimes",
            Topic::Energy => "energy",
            Topic::Radar => "radar",
            Topic::CarSettings => "carsettings",
            Topic::TyreData => "tyredata",
            Topic::PitRejoin => "pitrejoin",
            Topic::CornerTimes => "cornertimes",
        }
    }

    /// Publish every n-th telemetry tick (60 Hz). Driving aids follow every tick; timing
    /// tables can't be read faster than a few times a second, and each window parses all
    /// it is sent.
    pub fn every(self) -> u64 {
        match self {
            Topic::Player | Topic::Spotter => 1,
            // The track map's dots: 30 a second is smooth (a car moves well under a pixel).
            Topic::Cars | Topic::Pit => 2,
            Topic::Relative | Topic::Safety => 3,
            Topic::Standings | Topic::Battle => 6,
            Topic::Session | Topic::Fuel | Topic::Tyres | Topic::Incidents | Topic::Laptimes | Topic::Energy => 15,
            // Cars around you: 30 a second.
            Topic::Radar => 2,
            // A change shows within a tenth of a second (only sent when something changes).
            Topic::CarSettings => 6,
            Topic::TyreData => 15,
            // Four times a second: it moves with you round the lap.
            Topic::PitRejoin => 15,
            // Weather changes slowly: once a second.
            Topic::Forecast => 60,
            // Only re-sent when they change (see `publish`).
            Topic::TrackMap | Topic::Car | Topic::Delta | Topic::Drivers | Topic::CornerTimes => 1,
        }
    }

    /// The "fast" update rate, the lightest on CPU and frame rate: inputs, spotter and pit
    /// 30/s, track map 20/s, relative and safety alert 10/s, standings and battle 5/s.
    /// Telemetry is still read at 60/s, so nothing is missed, only shown less often.
    pub fn every_fast(self) -> u64 {
        match self {
            Topic::Player | Topic::Spotter | Topic::Pit => 2,
            Topic::Cars => 3,
            Topic::Relative | Topic::Safety => 6,
            Topic::Standings | Topic::Battle => 12,
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
            Topic::Drivers => serde_json::to_string(&drivers(e)),
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
            Topic::Forecast => serde_json::to_string(&forecast(e)),
            Topic::Laptimes => serde_json::to_string(&laptimes(e)),
            Topic::Energy => serde_json::to_string(&energy_out(e)),
            Topic::Radar => serde_json::to_string(&crate::engine::radar::build(e)),
            Topic::CarSettings => serde_json::to_string(&car_settings(e)),
            Topic::TyreData => serde_json::to_string(&tyre_data(e)),
            Topic::PitRejoin => serde_json::to_string(&pit_rejoin(e)),
            Topic::CornerTimes => serde_json::to_string(&corner_times(e)),
        };
        r.unwrap_or_else(|_| "null".into())
    }
}

/// The weather now and to come. Le Mans Ultimate publishes a forecast for each session;
/// iRacing shares only the conditions now with apps.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastOut {
    pub game: &'static str,
    /// Minutes left in the session (`None` when it runs on laps, with no clock to place the forecast on).
    pub minutes_left: Option<f32>,
    /// Time of day at the track, seconds since midnight (sun or moon icons).
    pub time_of_day: f32,
    pub now: WeatherNow,
    /// Forecast points, minutes from now (negative: already begun), in time order.
    pub points: Vec<ForecastPoint>,
    /// Why there is no forecast ("" when there is one).
    pub note: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherNow {
    pub sky: &'static str,
    pub air: f32,
    pub track: f32,
    /// Rain falling now, 0-1.
    pub rain: f32,
    /// iRacing-style track wetness (1 dry ... 7 extremely wet).
    pub wetness: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastPoint {
    pub minutes: f32,
    pub sky: &'static str,
    pub temp: f32,
    /// Chance of rain, 0-1.
    pub rain: f32,
}

/// Sky words shared by both games: clear, light, partly, mostly, overcast (clouds), then
/// drizzle, lightRain, rain, heavyRain, storm.
pub fn lmu_sky(code: i32) -> &'static str {
    ["clear", "light", "partly", "mostly", "overcast", "drizzle", "lightRain", "lightRain", "rain", "heavyRain", "storm"][code.clamp(0, 10) as usize]
}

/// iRacing: Skies (0 clear, 1 partly, 2 mostly, 3 overcast) and how hard it is raining.
pub fn iracing_sky(skies: i32, precipitation: f32) -> &'static str {
    match precipitation {
        p if p >= 0.6 => "heavyRain",
        p if p >= 0.3 => "rain",
        p if p >= 0.1 => "lightRain",
        p if p > 0.01 => "drizzle",
        _ => ["clear", "partly", "mostly", "overcast"][skies.clamp(0, 3) as usize],
    }
}

pub fn forecast(e: &Engine) -> ForecastOut {
    let t = &e.tel;
    // The mock sim plays LMU here, so the overlay can be tried without a game.
    let lmu = e.source == "lmu" || (e.source == "mock" && e.lmu_forecast.is_some());
    let now = WeatherNow {
        sky: if t.sky_code >= 0 { lmu_sky(t.sky_code) } else { iracing_sky(t.skies, t.precipitation) },
        air: round(t.air_temp, 1),
        track: round(t.track_temp, 1),
        rain: round(t.precipitation, 2),
        wetness: t.track_wetness,
    };
    let length = (t.session_time + t.session_time_remain) as f32;
    let timed = t.session_time_remain > 0.0 && length > 0.0;
    let minutes_left = timed.then(|| round(t.session_time_remain as f32 / 60.0, 1));
    let nodes = if lmu { e.lmu_forecast.as_ref().map(|f| f.for_session(e.session_def().map_or("", |s| s.kind.as_str()))).unwrap_or(&[]) } else { &[] };
    let points: Vec<ForecastPoint> = if timed {
        nodes
            .iter()
            .map(|n| ForecastPoint {
                minutes: round((n.frac * length - t.session_time as f32) / 60.0, 1),
                sky: lmu_sky(n.sky),
                temp: round(n.temp, 1),
                rain: round(n.rain, 2),
            })
            .collect()
    } else {
        vec![]
    };
    let note = if !lmu {
        "iRacing does not share its forecast with apps: conditions now only"
    } else if nodes.is_empty() {
        "Waiting for Le Mans Ultimate's forecast"
    } else if !timed {
        "This session runs on laps: no clock to place the forecast on"
    } else {
        ""
    };
    ForecastOut { game: e.source, minutes_left, time_of_day: t.time_of_day, now, points, note }
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
    /// Timed sessions: laps left at your pace (the time left over your lap time), and the
    /// laps the session will be (the leader's in a race; yours otherwise), lap-number style.
    pub est_laps_left: Option<f32>,
    pub est_laps_total: Option<i32>,
    /// The race's length in laps at the leader's pace (laps done plus the laps left by the
    /// clock): it counts up past the laps run, changing with the leader's pace.
    pub est_race_laps: Option<f32>,
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
    /// Every incident count that brings a penalty or the disqualification, in order.
    pub incident_limits: Vec<i32>,
    pub num_cars: usize,
    pub league: Option<&'a str>,
    /// iRacing league of the current session (0 if not a league session).
    pub league_id: i64,
    pub official: bool,
    pub subsession_id: i64,
}

/// Timed sessions: the laps left at your pace (time left over your lap time), and the lap the
/// session will end on (the leader's in a race, when the clock runs out and they finish the
/// lap they are on; the car you watch otherwise).
/// Estimated laps in a timed session: (laps left at your pace, the lap the leader finishes on,
/// the race's length in laps at the leader's pace: laps done plus the time left over their
/// lap time, e.g. 12.89).
fn estimated_laps(e: &Engine) -> Option<(f32, Option<i32>, Option<f32>)> {
    let t = &e.tel;
    let def = e.session_def()?;
    def.time?;
    let remain = t.session_time_remain;
    if !(0.0..1e6).contains(&remain) {
        return None;
    }
    let focus = t.focus_idx()?;
    let lap = race_clock(e, focus).player_lap;
    if lap <= 0.0 {
        return None;
    }
    let left = (remain / lap as f64) as f32;
    let car = if e.is_race() { e.order.first().map(|r| r.idx).unwrap_or(focus) } else { focus };
    let pace = if car == focus { lap } else { e.ref_lap(car) };
    // Progress counts the lap the car is on (lap 12, halfway: 12.5), so the lap the clock runs
    // out on is the whole part of where it will be then; reaching the line just as it does
    // ends the session there.
    let total = (pace > 0.0 && t.cars[car].lap > 0).then(|| (((e.progress(car) + remain / pace as f64) - 1e-3).ceil() as i32 - 1).max(t.cars[car].lap));
    // Laps done (lap 12, halfway: 11.5) plus the laps the time left allows at their pace.
    let length = (pace > 0.0 && t.cars[car].lap > 0).then(|| ((e.progress(car) - 1.0).max(0.0) + remain / pace as f64) as f32);
    Some((left, total, length))
}

pub fn session(e: &Engine) -> SessionOut<'_> {
    let t = &e.tel;
    let def = e.session_def();
    let irs: Vec<i32> = e.order.iter().filter_map(|r| e.static_of(r.idx)).map(|s| s.irating).collect();
    let time_total = def.and_then(|d| d.time).map(|v| v as f32);
    let est = estimated_laps(e);
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
        track_state: e.info.rubber_state(t.session_num),
        session_state: model::session_state_label(t.session_state),
        is_race: e.is_race(),
        time_remain: (time_total.is_some() && t.session_time_remain >= 0.0 && t.session_time_remain < 1e6).then(|| round(t.session_time_remain as f32, 1)),
        time_total,
        laps_remain: (def.and_then(|d| d.laps).is_some() && t.session_laps_remain >= 0 && t.session_laps_remain < 30000).then_some(t.session_laps_remain),
        laps_total: def.and_then(|d| d.laps),
        // Cars in the garage are on lap -1: no lap then.
        leader_lap: e.order.first().map(|r| t.cars[r.idx].lap.max(0)).unwrap_or(0),
        est_laps_left: est.map(|(left, _, _)| round(left, 2)),
        est_laps_total: est.and_then(|(_, total, _)| total),
        est_race_laps: est.and_then(|(_, _, length)| length).map(|v| round(v, 2)),
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
        incident_limits: e.info.incident_limits(),
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
    /// Start lights: "ready" (red), "set", "go" (green) or "" (not showing). Sent with the
    /// player at 60 Hz so the lights go out on the frame they do in the sim.
    pub start: &'static str,
    /// Seconds until a race session's get-in-car / warm-up countdown ends (the grid forms
    /// next); None outside it.
    pub race_start_in: Option<f32>,
    /// A race session is forming up / on its parade laps (the start comes next).
    pub gridding: bool,
    /// Rolling start: "pacing", "oneToGreen", "green" or "" (see `model::rolling_start`).
    pub rolling: &'static str,
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
        start: model::start_lights(t.session_flags),
        // iRacing counts down to the start in these states; other games' timers mean the race length.
        race_start_in: (e.source != "lmu" && e.is_race() && matches!(t.session_state, 1 | 2) && t.session_time_remain > 0.0 && t.session_time_remain < 3600.0)
            .then(|| round(t.session_time_remain as f32, 1)),
        gridding: e.is_race() && t.session_state == 3,
        // Standing starts have formation laps too: only a rolling start's pace laps count.
        rolling: if e.is_race() && e.info.standing_start == Some(false) { model::rolling_start(t.session_state, t.session_flags) } else { "" },
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
    // LMU private qualifying: nobody is really beside you (their cars are in other sessions).
    if state.is_empty() || !t.driving() || e.alone_on_track() {
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
        // Only the newest: the overlay shows up to 30, however many are kept.
        list: &e.incidents.list[e.incidents.list.len().saturating_sub(crate::engine::incidents::SENT)..],
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
    // LMU private qualifying: the others drive in sessions of their own, so the map shows you.
    let alone = e.alone_on_track();
    let me = t.focus_idx().or(usize::try_from(t.player_idx).ok());
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
            .filter(|r| t.cars[r.idx].in_world() && (!alone || Some(r.idx) == me))
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

/// Fields shared by relative, standings, safety and battle rows: which car, and whether it
/// is yours or the one being watched. Its fixed details come on the `drivers` topic (see
/// `DriverInfo`): the rows are sent many times a second, those only when they change.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverOut {
    pub idx: usize,
    pub is_player: bool,
    pub is_focus: bool,
}

pub fn driver(e: &Engine, s: &Static) -> DriverOut {
    DriverOut { idx: s.idx, is_player: s.idx as i32 == e.tel.player_idx, is_focus: Some(s.idx) == e.tel.focus_idx() }
}

/// A car's fixed details, merged into its rows by the client (ui/src/lib/client.ts).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverInfo<'a> {
    pub idx: usize,
    pub number: &'a str,
    pub name: &'a str,
    pub abbrev: &'a str,
    pub team: &'a str,
    pub car: &'a str,
    pub class_id: &'a str,
    pub class_name: &'a str,
    pub class_color: String,
    /// Secondary class (League builder): "" when the driver has none.
    pub class2_id: &'a str,
    pub class2_name: &'a str,
    pub class2_color: String,
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
    /// League team (Team builder): colour ("" for none), short name, logo URLs ("" for none).
    pub team_color: String,
    pub team_short: &'a str,
    pub team_logo: String,
    pub team_logo_small: String,
}

#[derive(Serialize)]
pub struct DriversOut<'a> {
    pub cars: Vec<DriverInfo<'a>>,
}

pub fn drivers(e: &Engine) -> DriversOut<'_> {
    DriversOut { cars: e.statics.iter().flatten().map(driver_info).collect() }
}

pub fn driver_info(s: &Static) -> DriverInfo<'_> {
    DriverInfo {
        idx: s.idx,
        number: &s.number,
        name: &s.name,
        abbrev: &s.abbrev,
        team: &s.team,
        car: &s.car,
        class_id: &s.class.id,
        class_name: &s.class.name,
        class_color: hex(s.class.color),
        class2_id: s.class2.as_ref().map_or("", |c| c.id.as_str()),
        class2_name: s.class2.as_ref().map_or("", |c| c.name.as_str()),
        class2_color: s.class2.as_ref().map(|c| hex(c.color)).unwrap_or_default(),
        user_id: s.user_id,
        car_id: s.car_id,
        irating: s.irating,
        license: &s.lic,
        license_color: hex(s.lic_color),
        club: &s.club,
        flair: &s.flair,
        country: &s.country,
        make: &s.make,
        team_color: s.team_color.map(hex).unwrap_or_default(),
        team_short: &s.team_short,
        team_logo: team_logo_url(&s.team_logo),
        team_logo_small: team_logo_url(&s.team_logo_small),
    }
}

/// Where the overlays load a team logo from ("" for none).
fn team_logo_url(file: &str) -> String {
    if file.is_empty() {
        String::new()
    } else {
        format!("/team-logos/{file}")
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
    /// Fuel put in at pit stops, where the game tells it for this car (LMU: every car;
    /// iRacing: only the player).
    pub fuel_added: Option<FuelAddedOut>,
    /// Seconds stood in the pit box at the last stop (or this one, live).
    pub pit_time: Option<PitTimeOut>,
    /// The lap the car last came into the pits on (none before its first stop).
    pub pit_lap: Option<i32>,
    /// Fuel or virtual energy left, where the game tells it for this car (LMU: every car;
    /// iRacing: only the player).
    pub fuel_left: Option<FuelLeftOut>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct FuelLeftOut {
    pub value: f32,
    /// "L" (litres) or "%" (of a full tank, or of the virtual energy allowance).
    pub unit: &'static str,
    /// Virtual energy (LMU's Hypercar, LMGT3...) rather than fuel.
    pub energy: bool,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct PitTimeOut {
    /// Being serviced (fuel, tyres, repairs).
    pub service: f32,
    /// Serving a penalty (stop-and-go).
    pub penalty: f32,
    /// The stop is still going on (in the box, or not yet out of the pit lane).
    pub live: bool,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct FuelAddedOut {
    /// At the last stop, or so far at the stop under way.
    pub last: f32,
    /// Over the session.
    pub total: f32,
    /// "L" (litres) or "%" (of a full tank).
    pub unit: &'static str,
    /// Filling up now.
    pub filling: bool,
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
        fuel_added: e.hist[idx].fuel_added().map(|(last, total)| FuelAddedOut {
            last: round(last, 1),
            total: round(total, 1),
            unit: if e.hist[idx].fuel_litres { "L" } else { "%" },
            filling: e.hist[idx].fuel_filling,
        }),
        fuel_left: fuel_left(e, idx),
        pit_time: e.hist[idx].pit_time().map(|(p, live)| PitTimeOut { service: round(p.service, 1), penalty: round(p.penalty, 1), live }),
        pit_lap: (e.hist[idx].pit_stops > 0 && e.hist[idx].last_pit_lap > 0).then_some(e.hist[idx].last_pit_lap),
    }
}

fn fuel_left(e: &Engine, idx: usize) -> Option<FuelLeftOut> {
    let c = &e.tel.cars[idx];
    let player = usize::try_from(e.tel.player_idx).ok() == Some(idx);
    // The player's own energy from telemetry when the per-car figure is missing.
    let energy = c.energy.or(if player { e.tel.virtual_energy } else { None });
    if let Some(v) = energy {
        return Some(FuelLeftOut { value: round(v, 0), unit: "%", energy: true });
    }
    if let Some(f) = c.fuel {
        return Some(FuelLeftOut { value: round(f * 100.0, 0), unit: "%", energy: false });
    }
    (player && e.tel.fuel_level > 0.0).then(|| FuelLeftOut { value: round(e.tel.fuel_level, 1), unit: "L", energy: false })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelRow {
    #[serde(flatten)]
    pub d: DriverOut,
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
pub struct RelativeOut {
    pub rows: Vec<RelRow>,
}

/// How many cars either side of you the relative needs: as many as the relative overlay showing
/// the most does (any relative overlay, as a disabled one may still be used in OBS), at most 10.
pub fn relative_side(cfg: &crate::config::Config) -> usize {
    const MAX: usize = 10;
    let wanted = |o: &crate::config::OverlayInstance, key: &str| o.settings.get(key).and_then(|v| v.as_f64()).map_or(4, |v| v.max(0.0).ceil() as usize);
    cfg.overlays.iter().filter(|o| o.kind == "relative").map(|o| wanted(o, "carsAhead").max(wanted(o, "carsBehind"))).max().unwrap_or(MAX).min(MAX)
}

pub fn relative(e: &Engine) -> RelativeOut {
    let t = &e.tel;
    let Some(focus) = t.focus_idx() else { return RelativeOut { rows: vec![] } };
    let race = e.is_race();
    let fprog = e.progress(focus);
    let alone = e.alone_on_track();
    let mut rows: Vec<RelRow> = e
        .order
        .iter()
        .filter(|r| t.cars[r.idx].in_world() && (!alone || r.idx == focus))
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
    let side = relative_side(&e.cfg);
    let start = me.saturating_sub(side);
    let end = (me + side + 1).min(rows.len());
    RelativeOut { rows: rows.drain(start..end).collect() }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandRow {
    #[serde(flatten)]
    pub d: DriverOut,
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
pub struct StandClass {
    #[serde(flatten)]
    pub class: ClassOut,
    pub best_lap: Option<f32>,
    pub rows: Vec<StandRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingsOut<'a> {
    pub is_race: bool,
    pub focus_class: Option<&'a str>,
    pub classes: Vec<StandClass>,
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
    // The class of the car followed; in the garage or the menus (no car on track) your own
    // class, so "my class only" still shows it.
    let player = usize::try_from(t.player_idx).ok();
    let focus_class = t.focus_idx().or(player).and_then(|f| e.static_of(f)).map(|s| s.class.id.as_str());
    StandingsOut { is_race: e.is_race(), focus_class, classes }
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
    /// Every lap is averaged, not only clean ones.
    pub any_laps: bool,
    /// How many of the last clean laps the average is over (Strategy → Fuel).
    pub avg_laps: usize,
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
    /// The race ahead, before it starts (practice, qualifying, the grid).
    pub race: Option<RaceEstimateOut>,
    /// This lap against the fuel per lap you can use (fuel save coach).
    pub coach: Option<FuelCoachOut>,
}

/// The lap in progress against a fuel per lap: what this point of the lap should have used
/// (by the shape of your last clean lap) and what it has.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuelCoachOut {
    /// "save" (the per lap that makes the finish on fewer stops) or "average" (your average).
    pub against: &'static str,
    pub per_lap: f32,
    pub used: f32,
    pub expected: f32,
    /// expected - used: above 0, under it (saving); below 0, over it.
    pub delta: f32,
    /// This lap's use if the rest goes like the part driven (from a tenth of the lap on).
    pub projected: Option<f32>,
}

/// How long the race will be and what it takes, worked out before the start.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RaceEstimateOut {
    pub laps: f32,
    /// Set by laps (else estimated from the race's time and your lap time).
    pub exact: bool,
    /// The race's time limit, minutes.
    pub minutes: Option<f32>,
    /// The lap time the estimate uses.
    pub lap_time: f32,
    /// Fuel for the whole race with your margin; the stops it takes starting full; what to
    /// start with (no more than a full tank).
    pub fuel: Option<f32>,
    pub stops: Option<i32>,
    pub start_fuel: Option<f32>,
    /// "session" (from the game) or "settings" (the race length you set).
    pub from: &'static str,
}

/// The race ahead and its length (laps, seconds), before it starts: the race session itself
/// before the green, else the next race of the event, else the length set in the settings.
fn race_ahead(e: &Engine) -> Option<(Option<i32>, Option<f64>, &'static str)> {
    let cur = e.session_def();
    if let Some(d) = cur.filter(|d| d.is_race()) {
        return (e.tel.session_state < crate::engine::STATE_RACING).then_some((d.laps, d.time, "session"));
    }
    let num = e.tel.session_num;
    if let Some(d) = e.info.sessions.iter().find(|s| s.is_race() && s.num > num) {
        return Some((d.laps, d.time, "session"));
    }
    let c = &e.cfg.fuel;
    (c.race_minutes > 0.0 || c.race_laps > 0)
        .then(|| ((c.race_laps > 0).then_some(c.race_laps), (c.race_minutes > 0.0).then_some(c.race_minutes as f64 * 60.0), "settings"))
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
    let mut laps_left = focus.and_then(|f| fuel::laps_remaining(&race_clock(e, f))).map(|v| v as f32);
    // Before the race: its length from its laps or time and your lap time.
    let race = race_ahead(e).and_then(|(laps, secs, from)| {
        let lap_time = focus.map(|f| race_clock(e, f).player_lap).filter(|l| *l > 0.0)?;
        let (n, exact) = fuel::race_laps(laps, secs, lap_time)?;
        let need = avg.filter(|a| *a > 0.0).map(|a| (n + cfg.margin_laps) * a);
        Some(RaceEstimateOut {
            laps: n,
            exact,
            minutes: secs.map(|t| round((t / 60.0) as f32, 1)),
            lap_time: r3(lap_time),
            fuel: need.map(|v| round(v, 2)),
            stops: need.filter(|_| capacity > 0.0).map(|v| ((v - capacity) / capacity).ceil().max(0.0) as i32),
            start_fuel: need.map(|v| round(if capacity > 0.0 { v.min(capacity) } else { v }, 2)),
            from,
        })
    });
    // On the grid the calculator plans the race now, not what is left of the session.
    if e.is_race() && t.session_state < crate::engine::STATE_RACING {
        if let Some(r) = &race {
            laps_left = Some(r.laps);
        }
    }
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
        laps_sampled: e.fuel.laps.iter().filter(|l| e.fuel.counts(l)).count(),
        any_laps: cfg.any_laps,
        avg_laps: n,
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
        coach: {
            let save = match (&plan, avg) {
                (Some(p), Some(a)) if p.stops >= 1 && p.target_per_lap > 0.0 => {
                    let tank_after_stops = t.fuel_level + capacity * (p.stops - 1) as f32;
                    let target = tank_after_stops / (laps_left.unwrap_or(0.0) + cfg.margin_laps);
                    let s = a - target;
                    (s > 0.0 && s <= a * 0.12).then_some(target)
                }
                _ => None,
            };
            let reference = save.map(|v| ("save", v)).or(avg.map(|a| ("average", a)));
            match (reference, e.fuel.lap_used()) {
                (Some((against, per_lap)), Some(used)) if t.is_on_track && !t.on_pit_road && per_lap > 0.0 && (0.02..1.0).contains(&t.lap_dist_pct) => {
                    let share = e.fuel.share_at(t.lap_dist_pct);
                    let expected = per_lap * share;
                    Some(FuelCoachOut {
                        against,
                        per_lap: r3(per_lap),
                        used: r3(used),
                        expected: r3(expected),
                        delta: r3(expected - used),
                        projected: (share > 0.1).then(|| r3(used / share)),
                    })
                }
                _ => None,
            }
        },
        window_opens_lap: plan.as_ref().filter(|p| p.stops > 0).map(|p| lap + p.window_opens_in.ceil() as i32),
        window_closes_lap: plan.as_ref().filter(|p| p.stops > 0).map(|p| lap + p.window_closes_in.floor() as i32),
        margin_laps: cfg.margin_laps,
        history: e.fuel.laps.iter().rev().filter(|l| e.fuel.counts(l)).take(10).map(|l| r3(l.used)).collect(),
        expected,
        estimate_from: if session_avg.is_some() {
            "session"
        } else if expected.is_some() {
            "history"
        } else {
            "none"
        },
        race,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleCar {
    #[serde(flatten)]
    pub d: DriverOut,
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
pub struct BattlePair {
    pub ahead: Option<BattleCar>,
    pub behind: Option<BattleCar>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleOut {
    /// Same class (position battle), overall order, and nearest on track.
    pub class: BattlePair,
    pub overall: BattlePair,
    pub track: BattlePair,
    pub my_last_lap: Option<f32>,
    pub my_best_lap: Option<f32>,
    pub max_gap: f32,
}

pub fn battle_out(e: &Engine) -> BattleOut {
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
    /// The pit lane as dashes (official maps), in the same unit box as `points`.
    pub pit: &'a [[[f32; 2]; 2]],
    pub source: Option<&'a str>,
    pub recording: bool,
    /// The outline is being built from the cars on track (Le Mans Ultimate), not recorded
    /// from the player's lap.
    pub building: bool,
    pub progress: f32,
    pub sectors: &'a [f32],
    /// Numbered as iRacing's own map numbers the turns, when the official map is known.
    pub corners: std::borrow::Cow<'a, [crate::config::Corner]>,
    /// The official map's turns that no corner covers (number only).
    pub turns: Vec<crate::config::Corner>,
    /// "custom" | "lovely" | "mock" | "none"
    pub corners_source: &'a str,
    pub straights: &'a [crate::config::Corner],
    pub pit_entry: Option<f32>,
    pub pit_exit: Option<f32>,
}

/// The corners numbered as iRacing's own map numbers them (when known; the player's own
/// corners stay as they made them), and the map's turns no corner covers.
fn numbered_corners(e: &Engine) -> (std::borrow::Cow<'_, [crate::config::Corner]>, Vec<crate::config::Corner>) {
    let official = e.map.official.as_ref().filter(|m| m.track_id == e.info.track_id && !m.turns.is_empty() && e.corners_source != "custom");
    match official {
        Some(m) => {
            let (c, t) = crate::corners::number_from_map(&e.corners, &m.turns);
            (std::borrow::Cow::Owned(c), t)
        }
        None => (std::borrow::Cow::Borrowed(e.corners.as_slice()), vec![]),
    }
}

pub fn trackmap(e: &Engine) -> TrackMapOut<'_> {
    let (corners, turns) = numbered_corners(e);
    TrackMapOut {
        track_id: e.info.track_id,
        track_name: &e.info.track_display_name,
        points: e.map.shown(e.cfg.track_outline != "recorded").map(|m| m.points.as_slice()),
        pit: e.map.shown(e.cfg.track_outline != "recorded").map(|m| m.pit.as_slice()).unwrap_or(&[]),
        source: e.map.shown(e.cfg.track_outline != "recorded").map(|m| m.source.as_str()),
        recording: e.map.recording(),
        building: e.map.building.is_some() && e.map.map.is_none(),
        progress: round(e.map.progress(), 2),
        sectors: &e.info.sectors,
        corners,
        turns,
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
    fn pit_rejoin_positions_and_neighbours() {
        let mut e = engine(5, true);
        // You (class 1) mid-lap; laps of 100 s, so 0.1 of a lap is 10 s.
        for (i, pct) in [(0, 0.5), (1, 0.45), (2, 0.30), (3, 0.20), (4, 0.70)] {
            place(&mut e, i, 5, pct);
        }
        e.tick();
        e.cfg.pit_rejoin.lane_seconds = 22.0;
        e.cfg.pit_rejoin.stop_seconds = 3.0;
        let r = pit_rejoin(&e);
        assert_eq!((r.loss, r.lane, r.stop, r.stop_from, r.lane_measured), (25.0, 22.0, 3.0, "set", 0));
        // P2 now; 25 s down you come out behind cars 1 and 2 as well: P4 (class: P2 to P3).
        assert_eq!(r.position, Some([2, 4]));
        assert_eq!(r.class_position, Some([2, 3]));
        let (a, b) = (r.ahead.unwrap(), r.behind.unwrap());
        assert_eq!((a.idx, a.gap, a.same_class), (2, 5.0, true));
        assert_eq!((b.idx, b.gap), (3, -5.0));
        assert!((r.ghost_pct.unwrap() - 0.25).abs() < 1e-4);
        // The stop from your class's stops when none is set and you have not stopped.
        e.cfg.pit_rejoin.stop_seconds = 0.0;
        assert_eq!(pit_rejoin(&e).stop_from, "none");
    }

    #[test]
    fn corner_times_add_up_to_the_lap() {
        let mut e = engine(1, false);
        let n = crate::garage61::REF_STEPS;
        let mine: Vec<f32> = (0..=n).map(|k| k as f32 / n as f32 * 100.0).collect();
        // The best lap a second quicker between 0.2 and 0.5 of the lap.
        let best: Vec<f32> = (0..=n)
            .map(|k| {
                let x = k as f32 / n as f32;
                x * 100.0 - ((x - 0.2) / 0.3).clamp(0.0, 1.0)
            })
            .collect();
        let r = |times: Vec<f32>, lap: f64| crate::garage61::Reference { driver: "Me".into(), lap_time: lap, source: "session".into(), times };
        e.laps.last = Some(crate::engine::laps::LastLap { lap_time: 100.0, valid: true, times: mine, session: Some(r(best, 99.0)), all_time: None });
        e.corners = vec![
            crate::config::Corner { name: "Turn one".into(), number: 1, last: 0, start: 0.2, end: 0.25 },
            crate::config::Corner { name: "".into(), number: 2, last: 3, start: 0.5, end: 0.55 },
        ];
        let c = corner_times(&e);
        assert_eq!(c.by, "corners");
        assert_eq!(c.segments.len(), 2);
        assert_eq!((c.segments[1].number, c.segments[1].last), (2, 3));
        let vs = &c.vs[0];
        assert_eq!(vs.source, "session");
        assert!((vs.deltas[0] - 1.0).abs() < 0.01 && vs.deltas[1].abs() < 0.01, "{:?}", vs.deltas);
        assert!((vs.deltas.iter().sum::<f32>() - vs.delta).abs() < 0.01);
    }

    #[test]
    fn official_map_survives_a_disconnect_and_a_new_session() {
        // The built-in map for track 1 arrives once (the downloader sends it once per track).
        let mut e = engine(2, false);
        let outline = crate::engine::trackmap::TrackMap {
            track_id: 1,
            name: "Test".into(),
            points: vec![[0.5, 0.5]; crate::engine::trackmap::BINS],
            source: "iracing".into(),
            version: crate::engine::trackmap::MAP_VERSION,
            pit: vec![],
            turns: vec![],
        };
        e.map.set_official(outline);
        assert_eq!(trackmap(&e).source, Some("iracing"));
        // iRacing disconnects (leaving the session), then a test session at the same track.
        e.reset();
        assert!(trackmap(&e).points.is_none());
        e.set_session(model::SessionInfo::parse(&crate::engine::tests::session_yaml(2, false)));
        e.connected = true;
        assert_eq!(trackmap(&e).source, Some("iracing"), "the map must still be there");
        // Another track: not drawn there.
        let other = crate::engine::tests::session_yaml(2, false).replace("TrackID: 1\n", "TrackID: 2\n");
        e.set_session(model::SessionInfo::parse(&other));
        assert!(trackmap(&e).points.is_none());
    }

    #[test]
    fn brake_bias_read_against_the_setup() {
        // A percentage, a fraction, hundredths of a %: the one near the setup's bias.
        assert_eq!(bias_pct(54.5, Some(55.0)), Some(54.5));
        assert_eq!(bias_pct(0.545, Some(55.0)), Some(54.5));
        assert_eq!(bias_pct(5450.0, Some(55.0)), Some(54.5));
        // An adjustment from the setup's bias follows each click in the car.
        assert_eq!(bias_pct(0.0, Some(55.0)), Some(55.0));
        assert_eq!(bias_pct(1.5, Some(55.0)), Some(56.5));
        assert_eq!(bias_pct(-2.0, Some(55.0)), Some(53.0));
        // 0.5 next to a 55% setup is half a percent more, not 50%.
        assert_eq!(bias_pct(0.5, Some(55.0)), Some(55.5));
        // Far from the setup's bias: still the reading itself, not the setup's bias.
        assert_eq!(bias_pct(39.9, Some(48.3)), Some(39.9));
        assert_eq!(bias_pct(40.3, Some(48.3)), Some(40.3));
        assert_eq!(bias_pct(3990.0, Some(48.3)), Some(39.9));
        // No setup known: only a plain percentage.
        assert_eq!(bias_pct(56.0, None), Some(56.0));
        assert_eq!(bias_pct(1.5, None), None);
    }

    #[test]
    fn estimated_laps_in_timed_sessions() {
        // A 40-minute race, 10 minutes (600 s) left, laps of 100 s (the class estimate).
        let yaml = crate::engine::tests::session_yaml(2, true).replace("   SessionLaps: 20\n", "   SessionLaps: unlimited\n   SessionTime: 2400.0000 sec\n");
        let mut e = engine(2, true);
        e.set_session(model::SessionInfo::parse(&yaml));
        e.tel.session_time_remain = 600.0;
        place(&mut e, 0, 12, 0.5);
        place(&mut e, 1, 12, 0.3);
        e.tick();
        let s = session(&e);
        assert_eq!(s.est_laps_left, Some(6.0));
        // The leader (on lap 12, halfway) covers 6 more laps: the clock runs out halfway round
        // lap 18, which they finish.
        assert_eq!(s.est_laps_total, Some(18));
        // The race's length at the leader's pace: 11.5 laps done and 6 more by the clock.
        assert_eq!(s.est_race_laps, Some(17.5));
        // A race set by laps: no estimate.
        let mut e = engine(2, true);
        e.tel.session_time_remain = 600.0;
        place(&mut e, 0, 12, 0.5);
        e.tick();
        assert_eq!(session(&e).est_laps_left, None);
        // In the garage (lap -1) there is no leader lap to show.
        e.tel.cars[0].lap = -1;
        e.tick();
        assert!(session(&e).leader_lap >= 0);
    }

    #[test]
    fn race_length_estimated_before_the_start() {
        // An iRacing event: 30 minutes of practice, then a 40-minute race.
        let yaml = crate::engine::tests::session_yaml(2, false).replace(
            "   SessionType: Practice\n   SessionLaps: 20\n",
            "   SessionType: Practice\n   SessionLaps: unlimited\n   SessionTime: 1800.0000 sec\n - SessionNum: 1\n   SessionType: Race\n   SessionLaps: unlimited\n   SessionTime: 2400.0000 sec\n",
        );
        let mut e = engine(2, false);
        e.set_session(model::SessionInfo::parse(&yaml));
        e.info.fuel_max_ltr = 50.0;
        e.tel.session_num = 0;
        e.tel.session_time_remain = 1200.0;
        e.tel.fuel_level = 20.0;
        place(&mut e, 0, 3, 0.5);
        place(&mut e, 1, 3, 0.2);
        e.tick();
        // No lap yet: the class's estimated lap (100 s) gives the laps; no fuel use known yet.
        let r = fuel_out(&e).race.unwrap();
        assert_eq!((r.laps, r.exact, r.minutes, r.from), (24.0, false, Some(40.0), "session"));
        assert!(r.fuel.is_none());
        // Laps of 1:36 using 2.5 L: 25 laps, (25 + 1 margin) x 2.5 = 65 L, one stop from a 50 L tank.
        for _ in 0..3 {
            e.fuel.laps.push_back(fuel::FuelLap { used: 2.5, time: 96.0, valid: true, start: 0.0, end: 0.0, why: "" });
        }
        let r = fuel_out(&e).race.unwrap();
        assert_eq!((r.laps, r.fuel, r.stops, r.start_fuel), (25.0, Some(65.0), Some(1), Some(50.0)));
        // On the grid the calculator plans the race itself.
        e.tel.session_num = 1;
        e.tel.session_state = 3;
        e.tick();
        let f = fuel_out(&e);
        let laps = f.race.as_ref().map(|r| r.laps);
        assert!(laps.is_some_and(|l| l == 24.0 || l == 25.0), "{laps:?}");
        assert_eq!(f.laps_remaining, laps);
        // Once racing, no estimate: the race is under way.
        e.tel.session_state = crate::engine::STATE_RACING;
        e.tick();
        assert!(fuel_out(&e).race.is_none());
        // Le Mans Ultimate in practice (no race listed): the race length from the settings.
        let mut e = engine(2, false);
        e.tel.session_num = 0;
        place(&mut e, 0, 3, 0.5);
        e.tick();
        assert!(fuel_out(&e).race.is_none());
        let mut cfg = e.cfg.clone();
        cfg.fuel.race_laps = 30;
        e.set_config(cfg);
        let r = fuel_out(&e).race.unwrap();
        assert_eq!((r.laps, r.exact, r.from), (30.0, true, "settings"));
    }

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
        // A relative overlay with its default 4 cars either side: 5 of these 6 (the focus car is first).
        e.cfg.overlays = crate::config::catalogue();
        assert_eq!(relative(&e).rows.len(), 5);
        // With no relative overlay to size it, every car (as before).
        e.cfg.overlays.clear();
        let rel = relative(&e);
        assert_eq!(rel.rows.len(), 6);
        assert!(rel.rows.windows(2).all(|w| w[0].gap >= w[1].gap));
        let st = standings(&e);
        assert_eq!(st.classes.len(), 2);
        assert_eq!(st.classes.iter().map(|c| c.rows.len()).sum::<usize>(), 6);
    }

    #[test]
    fn forecast_for_lmu_and_conditions_now_for_iracing() {
        use crate::source::lmu::extras::{Forecast, Node};
        let mut e = engine(4, true);
        e.tel.session_time = 600.0;
        e.tel.session_time_remain = 3000.0;
        e.tel.air_temp = 21.0;
        e.tel.skies = 2;
        // iRacing: the weather now, no forecast.
        e.source = "iracing";
        let f = forecast(&e);
        assert!(f.points.is_empty() && f.note.contains("iRacing"));
        assert_eq!(f.now.sky, "mostly");
        assert_eq!(f.minutes_left, Some(50.0));
        // LMU: its forecast points placed on the session clock (60 min session, 10 min in).
        e.source = "lmu";
        e.tel.sky_code = 6;
        let node = |frac: f32, sky: i32| Node { frac, sky, temp: 20.0, rain: 0.3 };
        e.lmu_forecast = Some(Forecast { race: (0..5).map(|i| node(i as f32 * 0.2, i * 2)).collect(), ..Default::default() });
        let f = forecast(&e);
        assert_eq!(f.note, "");
        assert_eq!(f.now.sky, "lightRain");
        assert_eq!(f.points.iter().map(|p| p.minutes).collect::<Vec<_>>(), vec![-10.0, 2.0, 14.0, 26.0, 38.0]);
        assert_eq!(f.points[4].sky, "rain");
        // A lap race: no clock to place it on.
        e.tel.session_time_remain = 0.0;
        let f = forecast(&e);
        assert!(f.points.is_empty() && f.minutes_left.is_none() && f.note.contains("laps"));
    }

    #[test]
    fn relative_sends_only_the_cars_its_overlays_show() {
        use crate::config::{Config, OverlayInstance};
        let rel = |settings: serde_json::Value| OverlayInstance { kind: "relative".into(), settings, ..Default::default() };
        // Default settings: 4 ahead and 4 behind.
        assert_eq!(relative_side(&Config { overlays: vec![rel(serde_json::json!({}))], ..Default::default() }), 4);
        // The relative showing the most decides; never more than 10.
        let two = Config {
            overlays: vec![rel(serde_json::json!({"carsAhead": 2, "carsBehind": 6})), rel(serde_json::json!({"carsAhead": 3}))],
            ..Default::default()
        };
        assert_eq!(relative_side(&two), 6);
        assert_eq!(relative_side(&Config { overlays: vec![rel(serde_json::json!({"carsBehind": 40}))], ..Default::default() }), 10);
        // No relative overlay (another app reading the topic): everything, as before.
        assert_eq!(relative_side(&Config { overlays: vec![], ..Default::default() }), 10);
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

/// One lap in the lap times overlay.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LapRow {
    pub lap: i32,
    /// The game's lap time (or the sectors added up when it has none).
    pub time: Option<f32>,
    pub sectors: Vec<Option<f32>>,
    pub pit: bool,
    /// Taken away for track limits (Le Mans Ultimate).
    pub invalid: bool,
    /// The driver's best lap of the session, and the fastest in the class.
    pub pb: bool,
    pub class_best: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LapTimesOut {
    /// The car shown (yours, or the one you are watching).
    pub car: Option<usize>,
    /// Lap in progress: its time so far and the sectors done.
    pub current_lap: i32,
    pub current_time: Option<f32>,
    pub current_sectors: Vec<Option<f32>>,
    pub current_invalid: bool,
    /// Newest first.
    pub laps: Vec<LapRow>,
    pub best_lap: Option<f32>,
    /// Best time in each sector this session, and their sum (best possible lap).
    pub best_sectors: Vec<Option<f32>>,
    pub theoretical: Option<f32>,
    pub class_best: Option<f32>,
}

pub fn laptimes(e: &Engine) -> LapTimesOut {
    let s = &e.sectors;
    let car = e.tel.focus_idx();
    let best = car.and_then(|i| e.best_lap(i));
    let class_best = car.and_then(|i| {
        let cid = e.static_of(i).map(|s| s.class.id.clone())?;
        e.classes.iter().find(|k| k.key.id == cid).and_then(|k| k.best_lap).map(|b| b.1)
    });
    let hist = car.map(|i| &e.hist[i].laps);
    let now = e.tel.session_time;
    let laps = s
        .laps
        .iter()
        .rev()
        .map(|l| {
            // The game's own time for the lap: the one that came in just after the line.
            let official = hist.and_then(|h| h.iter().rev().find(|r| r.at >= l.end_at - 0.5 && r.at <= l.end_at + 6.0)).map(|r| r.time);
            let summed = l.sectors.iter().all(|x| x.is_some()).then(|| l.sectors.iter().flatten().sum::<f64>() as f32);
            let time = official.or(summed).filter(|t| *t > 0.0);
            let same = |a: Option<f32>, b: Option<f32>| matches!((a, b), (Some(a), Some(b)) if (a - b).abs() < 1e-3);
            LapRow {
                lap: l.lap,
                time: time.map(r3),
                sectors: l.sectors.iter().map(|x| x.map(|v| r3(v as f32))).collect(),
                pit: l.pit,
                invalid: l.invalid,
                pb: !l.invalid && same(time, best),
                class_best: !l.invalid && same(time, class_best),
            }
        })
        .collect();
    let best_sectors: Vec<Option<f32>> = s.best.iter().map(|b| b.map(|v| r3(v as f32))).collect();
    let theoretical = (!best_sectors.is_empty() && best_sectors.iter().all(|b| b.is_some())).then(|| r3(best_sectors.iter().flatten().sum()));
    LapTimesOut {
        car,
        current_lap: car.map(|i| e.tel.cars[i].lap).unwrap_or(0),
        // Your own lap: the game's timer; another car's: since it crossed the line.
        current_time: if car.is_some() && car == usize::try_from(e.tel.player_idx).ok() && e.tel.lap_current_time > 0.0 {
            Some(round(e.tel.lap_current_time, 1))
        } else {
            s.lap_end_at.map(|at| round((now - at).max(0.0) as f32, 1))
        },
        current_sectors: s.current.iter().map(|x| x.map(|v| r3(v as f32))).collect(),
        current_invalid: e.tel.lap_invalid,
        laps,
        best_lap: best.map(r3),
        best_sectors,
        theoretical,
        class_best: class_best.map(r3),
    }
}

/// The player's settings changed from the wheel.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarSettingsOut {
    pub game: &'static str,
    /// In the car (settings are only shown for your own car).
    pub driving: bool,
    pub items: Vec<model::CarSetting>,
}

pub fn car_settings(e: &Engine) -> CarSettingsOut {
    let t = &e.tel;
    let driving = t.driving();
    let mut items = if driving { t.car_settings.clone() } else { vec![] };
    // iRacing: the live brake bias, in front %, read against the setup's bias (see
    // `bias_pct`), so it follows changes made in the car whatever scale the car uses.
    if e.source == "iracing" {
        let setup = e.info.setup_brake_bias;
        items.retain_mut(|s| {
            if !matches!(s.id, "brakeBias" | "peakBias") {
                return true;
            }
            match bias_pct(s.value, if s.id == "brakeBias" { setup } else { None }) {
                Some(v) => {
                    s.value = round(v, 2);
                    true
                }
                None => false,
            }
        });
    }
    // No live bias at all: the bias the setup states.
    if driving && e.source == "iracing" && !items.iter().any(|s| s.id == "brakeBias") {
        if let Some(b) = e.info.setup_brake_bias {
            items.insert(0, model::CarSetting { id: "brakeBias", value: b, max: None });
        }
    }
    CarSettingsOut { game: e.source, driving, items }
}

/// A brake bias in front %, from what the car gives: a percentage (54.5), a fraction (0.545),
/// hundredths of a % (5450), or an adjustment from the setup's bias (+1.5). With the setup's
/// bias known, the reading closest to it is used where a value could be read more than one way
/// (0.5: 50%, or half a percent more); a value that can only be a % is used however far the
/// driver has moved the bias from the setup's. Without the setup's bias, only a plain %.
pub fn bias_pct(raw: f32, setup: Option<f32>) -> Option<f32> {
    let as_pct = [(raw > 0.0 && raw <= 1.0).then_some(raw * 100.0), (raw > 100.0 && raw <= 10_000.0).then_some(raw / 100.0), Some(raw)];
    let pct = as_pct.into_iter().flatten().filter(|p| (20.0..=90.0).contains(p));
    match setup {
        // Whichever reading lands nearest the setup's bias: the right scale is the one that
        // puts it closest. No limit on how far: the bias in the car can be 8% or more from the
        // setup's (Audi RS 3 LMS TCR: 48.3% in the setup, 39.9% in the car).
        Some(base) => pct
            .chain((raw.abs() <= 10.0).then_some(base + raw))
            .filter(|p| (20.0..=90.0).contains(p))
            .min_by(|a, b| (a - base).abs().total_cmp(&(b - base).abs())),
        None => crate::source::iracing::brake_bias_pct(raw),
    }
}

/// The player's tyres and brakes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TyreDataOut {
    pub game: &'static str,
    pub driving: bool,
    /// Read as you drive (Le Mans Ultimate), or as measured at the last stop (iRacing).
    pub live: bool,
    /// The tyre's ideal temperature where the game says (°C).
    pub optimal: Option<f32>,
    /// Front-left, front-right, rear-left, rear-right.
    pub wheels: Option<[model::WheelTel; 4]>,
}

pub fn tyre_data(e: &Engine) -> TyreDataOut {
    let t = &e.tel;
    // Rounded to what is shown, so small wobbles are not re-sent.
    let wheels = t.wheels.map(|ws| {
        ws.map(|w| model::WheelTel {
            temps: w.temps.map(|a| a.map(|v| v.round())),
            carcass: w.carcass.map(f32::round),
            pressure: w.pressure.map(|v| round(v, 1)),
            wear: w.wear.map(|v| round(v, 1)),
            brake: w.brake.map(|v| (v / 5.0).round() * 5.0),
            flat: w.flat,
        })
    });
    TyreDataOut { game: e.source, driving: t.driving(), live: t.tyres_live, optimal: t.tyre_optimal.map(f32::round), wheels }
}

/// Le Mans Ultimate's virtual energy, planned like fuel (in % of the allowance).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnergyOut {
    /// The car races on virtual energy (Le Mans Ultimate's Hypercars, LMGT3...).
    pub available: bool,
    pub game: &'static str,
    pub level: Option<f32>,
    pub battery: Option<f32>,
    pub per_lap_avg: Option<f32>,
    pub per_lap_last: Option<f32>,
    pub laps_in_tank: Option<f32>,
    pub laps_remaining: Option<f32>,
    /// Energy needed to the flag, to add at the stop, stops, and what per lap would make it.
    pub to_finish: Option<f32>,
    pub to_add: Option<f32>,
    pub stops: Option<i32>,
    pub target_per_lap: Option<f32>,
    pub window_opens_lap: Option<i32>,
    pub window_closes_lap: Option<i32>,
    pub margin_laps: f32,
    pub history: Vec<f32>,
}

pub fn energy_out(e: &Engine) -> EnergyOut {
    let t = &e.tel;
    let cfg = &e.cfg.fuel;
    let avg = e.energy.average(cfg.avg_laps);
    let focus = usize::try_from(t.player_idx).ok().filter(|&i| i < t.cars.len());
    let laps_left = focus.and_then(|f| fuel::laps_remaining(&race_clock(e, f))).map(|v| v as f32);
    let level = t.virtual_energy;
    let plan = match (level, avg, laps_left) {
        (Some(l), Some(a), Some(left)) => Some(fuel::plan(l, 100.0, a, left, cfg.margin_laps)),
        _ => None,
    };
    let lap = t.lap.max(0);
    EnergyOut {
        available: level.is_some(),
        game: e.source,
        level: level.map(|v| round(v, 1)),
        battery: t.battery.map(|v| round(v, 0)),
        per_lap_avg: avg.map(|v| round(v, 2)),
        per_lap_last: e.energy.laps.back().map(|l| round(l.used, 2)),
        laps_in_tank: match (level, avg) {
            (Some(l), Some(a)) if a > 0.0 => Some(round(l / a, 1)),
            _ => None,
        },
        laps_remaining: laps_left.map(|v| round(v, 1)),
        to_finish: plan.as_ref().map(|p| round(p.fuel_to_finish, 1)),
        to_add: plan.as_ref().map(|p| round(p.fuel_to_add, 1)),
        stops: plan.as_ref().map(|p| p.stops),
        target_per_lap: plan.as_ref().filter(|p| p.target_per_lap > 0.0).map(|p| round(p.target_per_lap, 2)),
        window_opens_lap: plan.as_ref().filter(|p| p.stops > 0).map(|p| lap + p.window_opens_in.ceil() as i32),
        window_closes_lap: plan.as_ref().filter(|p| p.stops > 0).map(|p| lap + p.window_closes_in.floor() as i32),
        margin_laps: cfg.margin_laps,
        history: e.energy.laps.iter().rev().filter(|l| l.valid).take(10).map(|l| round(l.used, 2)).collect(),
    }
}

/// A car you would come out near after a stop.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RejoinCar {
    pub idx: usize,
    pub number: String,
    pub name: String,
    pub class_color: String,
    pub same_class: bool,
    /// Seconds from you on track after the stop (ahead positive).
    pub gap: f32,
    /// Laps it would be ahead of you (+1) or behind (-1) in the race; 0 the same lap.
    pub laps: i32,
}

/// Where you would rejoin if you came in now: positions before and after, the cars you would
/// come out between, and the spot on the map.
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PitRejoinOut {
    pub ready: bool,
    pub in_pits: bool,
    pub race: bool,
    /// Seconds a stop costs: the pit lane plus standing in the box.
    pub loss: f32,
    pub lane: f32,
    /// Stops the lane time was measured from (0: the time set in the settings).
    pub lane_measured: usize,
    pub stop: f32,
    /// "plan" (what you set in the black box, timed from the car's measured stops), "set"
    /// (the settings), "you" (your last stop), "class" (your class's stops), "none".
    pub stop_from: &'static str,
    /// The black box stop worked out: fuel and tyres and how long each takes.
    pub plan: Option<crate::engine::pitservice::StopPlan>,
    /// Where the timings come from: "you" (your stops in this car) or "community" (shared).
    pub plan_from: Option<&'static str>,
    /// Mandatory repairs still to do (s), included in the stop.
    pub repair: f32,
    /// [now, after], in the race.
    pub position: Option<[i32; 2]>,
    pub class_position: Option<[i32; 2]>,
    pub ahead: Option<RejoinCar>,
    pub behind: Option<RejoinCar>,
    /// Lap fraction you would rejoin at (where you will be when the stop is done).
    pub ghost_pct: Option<f32>,
}

/// Seconds into the lap at lap fraction `x` on a reference lap.
fn ref_time(times: &[f32], x: f32) -> f32 {
    let n = times.len() - 1;
    let f = x.clamp(0.0, 1.0) * n as f32;
    let k = (f as usize).min(n - 1);
    times[k] + (times[k + 1] - times[k]) * (f - k as f32)
}

/// The lap fraction a reference lap reaches at `secs` into it.
fn ref_pct(times: &[f32], secs: f32) -> f32 {
    let n = times.len() - 1;
    let k = times.partition_point(|&t| t < secs).clamp(1, n);
    let (a, b) = (times[k - 1], times[k]);
    let f = if b > a { (secs - a) / (b - a) } else { 0.0 };
    ((k - 1) as f32 + f.clamp(0.0, 1.0)) / n as f32
}

pub fn pit_rejoin(e: &Engine) -> PitRejoinOut {
    let Some(me) = e.tel.focus_idx() else { return PitRejoinOut::default() };
    let Some(my) = e.static_of(me) else { return PitRejoinOut::default() };
    let cfg = &e.cfg.pit_rejoin;
    let (lane, lane_measured) = e.pit.lane_loss().unwrap_or((cfg.lane_seconds, 0));
    // The stop: as set, else your last one, else the middle of your class's.
    let served = |i: usize| e.hist[i].pit_last.map(|p| p.service).filter(|s| *s >= 3.0);
    // Your own car in iRacing: what the black box is set to do, timed per car.
    let player = usize::try_from(e.tel.player_idx).ok() == Some(me) && matches!(e.source, "iracing" | "mock");
    let plan = player.then(|| e.pit_service.model(my.car_id)).flatten().and_then(|(m, from)| {
        let (fuel, tyres) = crate::engine::pitservice::planned(&e.tel, e.info.fuel_capacity());
        m.predict(fuel, tyres).map(|p| (p, from))
    });
    let repair = if player { e.tel.pit_repair_left.max(0.0) } else { 0.0 };
    let (stop, stop_from) = if cfg.stop_seconds > 0.0 {
        (cfg.stop_seconds, "set")
    } else if let Some((p, _)) = plan {
        (p.stood + repair, "plan")
    } else if let Some(s) = served(me) {
        (s, "you")
    } else {
        let mut v: Vec<f32> =
            (0..crate::model::MAX_CARS).filter(|&i| i != me && e.static_of(i).is_some_and(|s| s.class.id == my.class.id)).filter_map(served).collect();
        v.sort_by(f32::total_cmp);
        match v.get(v.len() / 2) {
            Some(&s) => (s, "class"),
            None => (0.0, "none"),
        }
    };
    let loss = lane + stop;
    let reference = e.ref_lap(me).max(1.0);
    let shift = (loss / reference) as f64;
    let now = e.progress(me);
    let after = now - shift;
    let race = e.is_race();
    let mut out = PitRejoinOut {
        ready: true,
        in_pits: e.tel.cars[me].on_pit_road,
        race,
        loss: round(loss, 1),
        lane: round(lane, 1),
        lane_measured,
        stop: round(stop, 1),
        stop_from,
        plan: plan.map(|(p, _)| crate::engine::pitservice::StopPlan {
            fuel_l: round(p.fuel_l, 1),
            fuel_secs: round(p.fuel_secs, 1),
            tyre_secs: round(p.tyre_secs, 1),
            base: round(p.base, 1),
            stood: round(p.stood, 1),
            ..p
        }),
        plan_from: plan.map(|(_, f)| f),
        repair: round(repair, 1),
        ..Default::default()
    };
    let others: Vec<usize> =
        (0..crate::model::MAX_CARS).filter(|&i| i != me && e.static_of(i).is_some_and(|s| !s.hidden) && (e.tel.cars[i].in_world() || e.hist[i].seen)).collect();
    if race {
        if let Some(r) = e.order.iter().find(|r| r.idx == me) {
            let ahead_of = |class: bool| {
                others.iter().filter(|&&i| (!class || e.static_of(i).is_some_and(|s| s.class.id == my.class.id)) && e.progress(i) > after).count() as i32 + 1
            };
            out.position = Some([r.pos, ahead_of(false)]);
            out.class_position = Some([r.class_pos, ahead_of(true)]);
        }
    }
    // The cars just ahead and behind on track when you come out (any lap), out on track.
    let mut best_ahead: Option<(f64, usize, f64)> = None;
    let mut best_behind: Option<(f64, usize, f64)> = None;
    for &i in &others {
        if !e.tel.cars[i].in_world() {
            continue;
        }
        let d = e.progress(i) - after;
        let w = d - d.round();
        if w >= 0.0 {
            if best_ahead.is_none_or(|b| w < b.0) {
                best_ahead = Some((w, i, d));
            }
        } else if best_behind.is_none_or(|b| w > b.0) {
            best_behind = Some((w, i, d));
        }
    }
    let car = |(w, i, d): (f64, usize, f64)| {
        let s = e.static_of(i)?;
        Some(RejoinCar {
            idx: i,
            number: s.number.clone(),
            name: s.abbrev.clone(),
            class_color: hex(s.class.color),
            same_class: s.class.id == my.class.id,
            gap: round((w * reference as f64) as f32, 1),
            laps: if race { d.round() as i32 } else { 0 },
        })
    };
    out.ahead = best_ahead.and_then(car);
    out.behind = best_behind.and_then(car);
    // Where on track: back along your best lap by the time the stop costs.
    let pct = e.tel.cars[me].pct;
    let lap_ref = e.laps.session.as_ref().or(e.laps.all_time.as_ref()).filter(|r| r.times.len() > 10);
    out.ghost_pct = Some(match lap_ref {
        Some(r) => {
            let lap = r.lap_time as f32;
            ref_pct(&r.times, (ref_time(&r.times, pct) - loss).rem_euclid(lap))
        }
        None => (pct - shift as f32).rem_euclid(1.0),
    });
    out
}

/// One stretch of the lap in the corner times: a corner and the straight after it.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CornerSeg {
    pub number: i32,
    pub last: i32,
    pub name: String,
}

/// Your last lap against a best lap, stretch by stretch.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CornerVs {
    /// "session" or "allTime".
    pub source: &'static str,
    pub lap_time: f32,
    pub delta: f32,
    pub deltas: Vec<f32>,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CornerTimesOut {
    pub lap_time: Option<f32>,
    pub valid: bool,
    /// "corners", "sectors" (no corner data for the track) or "lap".
    pub by: &'static str,
    pub segments: Vec<CornerSeg>,
    pub vs: Vec<CornerVs>,
}

/// Your last full lap, corner by corner (each corner with the straight after it, so the times
/// add up to the lap), against your best this session and your all-time best.
pub fn corner_times(e: &Engine) -> CornerTimesOut {
    let Some(last) = &e.laps.last else { return CornerTimesOut::default() };
    let (corners, turns) = numbered_corners(e);
    let mut cs: Vec<crate::config::Corner> = corners.iter().chain(turns.iter()).cloned().collect();
    cs.sort_by(|a, b| a.start.total_cmp(&b.start));
    cs.dedup_by(|b, a| (b.start - a.start).abs() < 0.002);
    let (by, segs): (&'static str, Vec<(f32, CornerSeg)>) = if !cs.is_empty() {
        ("corners", cs.into_iter().map(|c| (c.start, CornerSeg { number: c.number, last: c.last, name: c.name })).collect())
    } else if e.info.sectors.len() > 1 {
        ("sectors", e.info.sectors.iter().enumerate().map(|(i, &s)| (s, CornerSeg { number: 0, last: 0, name: format!("S{}", i + 1) })).collect())
    } else {
        ("lap", vec![(0.0, CornerSeg { number: 0, last: 0, name: "Lap".into() })])
    };
    let starts: Vec<f32> = segs.iter().map(|s| s.0).collect();
    // Time from one start to the next on a lap (the last stretch runs over the line).
    let spans = |times: &[f32], lap: f32| -> Vec<f32> {
        (0..starts.len())
            .map(|k| {
                let a = ref_time(times, starts[k]);
                if k + 1 < starts.len() {
                    ref_time(times, starts[k + 1]) - a
                } else {
                    lap - a + ref_time(times, starts[0])
                }
            })
            .collect()
    };
    let mine = spans(&last.times, last.lap_time);
    let vs = [("session", &last.session), ("allTime", &last.all_time)]
        .into_iter()
        .filter_map(|(source, r)| {
            let r = r.as_ref().filter(|r| r.times.len() == last.times.len())?;
            let theirs = spans(&r.times, r.lap_time as f32);
            Some(CornerVs {
                source,
                lap_time: round(r.lap_time as f32, 3),
                delta: round(last.lap_time - r.lap_time as f32, 3),
                deltas: mine.iter().zip(&theirs).map(|(m, t)| round(m - t, 3)).collect(),
            })
        })
        .collect();
    CornerTimesOut { lap_time: Some(round(last.lap_time, 3)), valid: last.valid, by, segments: segs.into_iter().map(|s| s.1).collect(), vs }
}
