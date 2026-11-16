//! Garage 61 integration (https://garage61.net/developer). For the current
//! car/track combination it fetches your personal best and your teams' best
//! laps, downloads the telemetry of a reference lap (your best or the team's)
//! for a live delta, and publishes it all on the `garage61` topic.
//!
//! Auth is a personal access token sent as `Authorization: Bearer <token>`.

use crate::config::{Config, ConfigStore, Garage61Config};
use crate::hub::Hub;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const API: &str = "https://garage61.net/api/v1";
/// Reference lap time is sampled at this many equal steps of lap distance.
pub const REF_STEPS: usize = 500;
/// Reference laps kept on disk (each is a few kB).
const CACHE_KEEP: usize = 60;

/// The car/track currently driven, published by the telemetry thread.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Combo {
    pub car_id: i64,
    pub car_name: String,
    pub track_id: i64,
    pub track_name: String,
    pub track_config: String,
    /// Track temperature (°C, whole degrees) and whether the track is wet, for the
    /// "similar conditions" filter.
    pub track_temp: i32,
    pub wet: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LapInfo {
    #[serde(skip)]
    pub id: String,
    pub driver: String,
    pub lap_time: f64,
    pub date: String,
    pub is_me: bool,
}

/// A reference lap: its time at each step of lap distance.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub driver: String,
    pub lap_time: f64,
    /// "personal" or "team".
    pub source: String,
    /// Seconds into the lap at lap distance k / REF_STEPS, k = 0..=REF_STEPS.
    pub times: Vec<f32>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct G61State {
    /// "disabled" | "noToken" | "waiting" | "loading" | "ok" | "unmapped" | "error"
    pub status: String,
    pub message: String,
    pub user: String,
    pub car: String,
    pub track: String,
    /// Which laps were searched, e.g. "Current season · 26–38 °C · dry".
    pub filters: String,
    pub personal_best: Option<LapInfo>,
    pub team_best: Option<LapInfo>,
    pub leaderboard: Vec<LapInfo>,
    pub reference: Option<Reference>,
    /// Why there is no reference lap (or why a fallback was used).
    pub reference_note: String,
    /// Your Garage 61 teams.
    pub teams: Vec<TeamOut>,
    /// Reference laps the delta bars asked for: "personal", "teams" (fastest in all your
    /// teams), "team:<slug>" and "world" (fastest you can see on Garage 61).
    pub references: BTreeMap<String, Reference>,
    /// Why one of those has no reference lap.
    pub reference_notes: BTreeMap<String, String>,
    /// Fastest lap you can see on Garage 61, telemetry or not.
    pub world_best: Option<LapInfo>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct TeamOut {
    pub slug: String,
    pub name: String,
}

/// The Garage 61 reference laps the enabled delta bars use (see [`G61State::references`]).
pub fn wanted_refs(c: &Config) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for o in c.overlays.iter().filter(|o| o.kind == "delta" && o.enabled) {
        let get = |k: &str| o.settings.get(k).and_then(|v| v.as_str()).unwrap_or("");
        match get("reference") {
            "" | "allTime" | "g61Personal" => out.insert("personal".to_string()),
            "g61Teams" => out.insert("teams".to_string()),
            "g61Team" if !get("team").is_empty() => out.insert(format!("team:{}", get("team"))),
            "g61World" => out.insert("world".to_string()),
            _ => false,
        };
    }
    out
}

struct Client {
    agent: ureq::Agent,
    token: String,
    cars: Option<Vec<Value>>,
    tracks: Option<Vec<Value>>,
    me: Option<Value>,
}

impl Client {
    fn new(token: &str) -> Self {
        let agent = crate::net::agent();
        Self { agent, token: token.to_string(), cars: None, tracks: None, me: None }
    }

    fn call(&self, path: &str, query: &[(&str, String)]) -> Result<ureq::Response, String> {
        let mut req = self.agent.get(&format!("{API}{path}")).set("Authorization", &format!("Bearer {}", self.token));
        for (k, v) in query {
            req = req.query(k, v);
        }
        match req.call() {
            Ok(r) => Ok(r),
            Err(ureq::Error::Status(401 | 403, _)) => Err("Token rejected — check it has the driving data permission".into()),
            Err(ureq::Error::Status(404, _)) => Err("Not found on Garage 61".into()),
            Err(ureq::Error::Status(429, _)) => Err("Rate limited by Garage 61, will retry".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
        self.call(path, query)?.into_json::<Value>().map_err(|e| e.to_string())
    }

    fn text(&self, path: &str) -> Result<String, String> {
        self.call(path, &[])?.into_string().map_err(|e| e.to_string())
    }

    fn items(v: Value) -> Vec<Value> {
        match v.get("items") {
            Some(Value::Array(a)) => a.clone(),
            _ => vec![],
        }
    }
}

fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

fn platform_id(v: &Value) -> Option<i64> {
    ["platform_id", "platformId"].iter().find_map(|k| match v.get(*k)? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

fn is_iracing(v: &Value) -> bool {
    v.get("platform").and_then(|p| p.as_str()).is_none_or(|p| p.eq_ignore_ascii_case("iracing"))
}

/// Find the Garage 61 car matching an iRacing car (by platform id, then by name).
pub fn match_car(cars: &[Value], car_id: i64, car_name: &str) -> Option<i64> {
    let ir: Vec<&Value> = cars.iter().filter(|c| is_iracing(c)).collect();
    if let Some(c) = ir.iter().find(|c| platform_id(c) == Some(car_id)) {
        return c.get("id")?.as_i64();
    }
    let n = norm(car_name);
    ir.iter().find(|c| c.get("name").and_then(|v| v.as_str()).is_some_and(|s| norm(s) == n)).and_then(|c| c.get("id")?.as_i64())
}

/// Find the Garage 61 track layout matching an iRacing track.
pub fn match_track(tracks: &[Value], track_id: i64, name: &str, config: &str) -> Option<i64> {
    let ir: Vec<&Value> = tracks.iter().filter(|t| is_iracing(t)).collect();
    if let Some(t) = ir.iter().find(|t| platform_id(t) == Some(track_id)) {
        return t.get("id")?.as_i64();
    }
    let (n, c) = (norm(name), norm(config));
    let field = |t: &Value, k: &str| norm(t.get(k).and_then(|v| v.as_str()).unwrap_or(""));
    ir.iter()
        .filter(|t| field(t, "name") == n || n.starts_with(&field(t, "name")) && !field(t, "name").is_empty())
        .max_by_key(|t| (field(t, "variant") == c) as i32 * 2 + (field(t, "name") == n) as i32)
        .and_then(|t| t.get("id")?.as_i64())
}

fn lap_info(v: &Value, me_slug: &str) -> Option<LapInfo> {
    let d = v.get("driver");
    let first = d.and_then(|d| d.get("firstName")).and_then(|v| v.as_str()).unwrap_or("");
    let last = d.and_then(|d| d.get("lastName")).and_then(|v| v.as_str()).unwrap_or("");
    let slug = d.and_then(|d| d.get("slug")).and_then(|v| v.as_str()).unwrap_or("");
    Some(LapInfo {
        id: match v.get("id") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        },
        driver: format!("{first} {last}").trim().to_string(),
        lap_time: v.get("lapTime")?.as_f64()?,
        date: v.get("startTime").and_then(|v| v.as_str()).unwrap_or("").chars().take(10).collect(),
        is_me: !slug.is_empty() && slug == me_slug,
    })
}

/// Lap search filters for the configured period and conditions, plus a short description.
pub fn lap_filters(cfg: &Garage61Config, combo: &Combo) -> (Vec<(&'static str, String)>, String) {
    let mut q: Vec<(&'static str, String)> = vec![];
    let mut desc = vec![];
    match cfg.period.as_str() {
        "season" => {
            q.push(("age", "-1".into()));
            desc.push("Current season".to_string());
        }
        "seasons2" => {
            q.push(("age", "-2".into()));
            desc.push("This and last season".to_string());
        }
        p if p.parse::<u32>().is_ok_and(|d| d > 0) => {
            q.push(("age", p.to_string()));
            desc.push(format!("Last {p} days"));
        }
        // Without a date filter Garage 61 may only search recent laps.
        _ => {
            q.push(("after", "2008-01-01T00:00:00Z".into()));
            desc.push("All time".to_string());
        }
    }
    if cfg.match_conditions {
        let r = cfg.temp_range.max(0.5);
        let (lo, hi) = (combo.track_temp as f32 - r, combo.track_temp as f32 + r);
        q.push(("minConditionsTrackTemp", format!("{lo}")));
        q.push(("maxConditionsTrackTemp", format!("{hi}")));
        // iRacing's TrackWetness: 1 dry, 2 mostly dry, 3+ wet.
        if combo.wet {
            q.push(("minConditionsTrackWetness", "3".into()));
        } else {
            q.push(("maxConditionsTrackWetness", "2".into()));
        }
        desc.push(format!("{lo:.0}–{hi:.0} °C"));
        desc.push(if combo.wet { "wet" } else { "dry" }.to_string());
    }
    (q, desc.join(" · "))
}

/// Find a column by any of its names (case-insensitive).
fn column(header: &[&str], names: &[&str]) -> Option<usize> {
    names.iter().find_map(|n| header.iter().position(|h| h.trim().trim_matches('"').eq_ignore_ascii_case(n)))
}

/// Time into the lap at each step of lap distance, from a Garage 61 telemetry CSV.
/// Uses the CSV's lap time column when it has one, otherwise integrates distance over
/// speed; either way the result is scaled to the official `lap_time`.
pub fn reference_times(csv: &str, lap_time: f64) -> Option<Vec<f32>> {
    let mut lines = csv.lines().filter(|l| !l.trim().is_empty());
    let header: Vec<&str> = lines.next()?.split(',').collect();
    let pct_col = column(&header, &["LapDistPct"])?;
    let speed_col = column(&header, &["Speed"]);
    let time_col = column(&header, &["LapCurrentLapTime", "LapTime", "Time", "SessionTime"]);
    if speed_col.is_none() && time_col.is_none() {
        return None;
    }
    let num = |cells: &[&str], i: Option<usize>| i.and_then(|i| cells.get(i)?.trim().trim_matches('"').parse::<f64>().ok());
    // (lap distance, seconds or None, speed)
    let mut rows: Vec<(f64, Option<f64>, f64)> = vec![];
    for line in lines {
        let cells: Vec<&str> = line.split(',').collect();
        let Some(p) = num(&cells, Some(pct_col)) else { continue };
        let (t, v) = (num(&cells, time_col), num(&cells, speed_col).unwrap_or(0.0));
        match rows.last() {
            // Samples from before the line at the start of the lap.
            None if p > 0.5 => continue,
            // Across the line at the end of the lap.
            Some(&(last, _, _)) if p < last - 0.5 => break,
            Some(&(last, _, _)) if p <= last => continue,
            _ => rows.push((p, t, v)),
        }
    }
    if rows.len() < 20 || rows.last()?.0 < 0.9 {
        return None;
    }
    let use_time = time_col.is_some() && rows.iter().all(|r| r.1.is_some());
    let edge = |p: f64, v: f64| p / v.max(1.0);
    // Cumulative time at each sample, from p = 0.
    let mut cum = Vec::with_capacity(rows.len());
    let first = rows[0];
    let mut acc = if use_time { 0.0 } else { edge(first.0, first.2) };
    cum.push(acc);
    for w in rows.windows(2) {
        let (a, b) = (w[0], w[1]);
        acc += if use_time { (b.1.unwrap() - a.1.unwrap()).max(0.0) } else { (b.0 - a.0) / ((a.2 + b.2) / 2.0).max(1.0) };
        cum.push(acc);
    }
    if use_time {
        // Seconds from the line to the first sample, at the pace of the first stretch.
        let k = rows.len().min(10) - 1;
        let rate = cum[k] / (rows[k].0 - first.0).max(1e-6);
        let lead = first.0 * rate;
        cum.iter_mut().for_each(|c| *c += lead);
    }
    let last = *rows.last()?;
    let total = acc
        + if use_time {
            let k = rows.len() - rows.len().min(10);
            let rate = (acc - cum[k] + cum[0]) / (last.0 - rows[k].0).max(1e-6);
            (1.0 - last.0) * rate
        } else {
            edge(1.0 - last.0, last.2)
        };
    let scale = lap_time / total;
    let mut out = Vec::with_capacity(REF_STEPS + 1);
    let mut j = 0;
    for k in 0..=REF_STEPS {
        let p = k as f64 / REF_STEPS as f64;
        let t = if p <= rows[0].0 {
            cum[0] * p / rows[0].0.max(1e-9)
        } else if p >= last.0 {
            let c = *cum.last()?;
            c + (total - c) * (p - last.0) / (1.0 - last.0).max(1e-9)
        } else {
            while j + 1 < rows.len() && rows[j + 1].0 < p {
                j += 1;
            }
            let (a, b) = (rows[j], rows[j + 1]);
            cum[j] + (cum[j + 1] - cum[j]) * (p - a.0) / (b.0 - a.0)
        };
        out.push((t * scale) as f32);
    }
    Some(out)
}

fn cache_path(dir: &Path, lap_id: &str) -> PathBuf {
    let safe: String = lap_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    dir.join(format!("{safe}.json"))
}

/// Keep only the most recently used reference laps.
fn prune(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .filter(|(_, p)| p.extension().is_some_and(|x| x == "json"))
        .collect();
    if files.len() <= CACHE_KEEP {
        return;
    }
    files.sort();
    for (_, p) in &files[..files.len() - CACHE_KEEP] {
        let _ = std::fs::remove_file(p);
    }
}

/// The reference lap's timing: from the cache, else downloaded from Garage 61.
fn reference_lap(client: &Client, lap: &LapInfo, source: &str, cache: &Path) -> Result<Reference, String> {
    if lap.id.is_empty() {
        return Err("Garage 61 gave no lap id".into());
    }
    let file = cache_path(cache, &lap.id);
    if let Some(r) = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str::<Reference>(&s).ok()) {
        // Touch it so pruning keeps laps in use.
        let _ = std::fs::write(&file, serde_json::to_string(&r).unwrap_or_default());
        return Ok(Reference { driver: lap.driver.clone(), source: source.into(), ..r });
    }
    let csv = client.text(&format!("/laps/{}/csv", lap.id))?;
    let times = reference_times(&csv, lap.lap_time).ok_or("this lap has no usable telemetry on Garage 61")?;
    let r = Reference { driver: lap.driver.clone(), lap_time: lap.lap_time, source: source.into(), times };
    let _ = std::fs::create_dir_all(cache);
    let _ = std::fs::write(&file, serde_json::to_string(&r).unwrap_or_default());
    prune(cache);
    Ok(r)
}

fn fetch(client: &mut Client, combo: &Combo, cfg: &Garage61Config, refs: &BTreeSet<String>, cache: &Path) -> Result<G61State, String> {
    if client.me.is_none() {
        client.me = Some(client.get("/me", &[])?);
    }
    if client.cars.is_none() {
        client.cars = Some(Client::items(client.get("/cars", &[])?));
    }
    if client.tracks.is_none() {
        client.tracks = Some(Client::items(client.get("/tracks", &[])?));
    }
    let me = client.me.clone().unwrap_or_default();
    let perms: Vec<&str> = me.get("apiPermissions").and_then(|p| p.as_array()).map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
    if !perms.is_empty() && !perms.iter().any(|p| p.contains("driving")) {
        return Err("This token has no “driving data” permission — create one that has it on garage61.net".into());
    }
    let slug = me.get("slug").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let user = format!("{} {}", me.get("firstName").and_then(|v| v.as_str()).unwrap_or(""), me.get("lastName").and_then(|v| v.as_str()).unwrap_or(""))
        .trim()
        .to_string();
    let (filters, desc) = lap_filters(cfg, combo);
    let mut st = G61State { status: "ok".into(), user, car: combo.car_name.clone(), track: combo.track_name.clone(), filters: desc, ..Default::default() };
    let car = match_car(client.cars.as_deref().unwrap_or(&[]), combo.car_id, &combo.car_name);
    let track = match_track(client.tracks.as_deref().unwrap_or(&[]), combo.track_id, &combo.track_name, &combo.track_config);
    let (Some(car), Some(track)) = (car, track) else {
        st.status = "unmapped".into();
        st.message = "This car/track combination was not found on Garage 61".into();
        return Ok(st);
    };
    let base = [vec![("cars", car.to_string()), ("tracks", track.to_string()), ("group", "driver".to_string())], filters].concat();
    let mine = client.get("/laps", &[base.as_slice(), &[("drivers", "me".into()), ("limit", "1".into())]].concat())?;
    st.personal_best = Client::items(mine).first().and_then(|l| lap_info(l, &slug)).map(|mut l| {
        l.is_me = true;
        l
    });
    st.teams = me
        .get("teams")
        .and_then(|t| t.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    let slug = t.get("slug").and_then(|s| s.as_str())?.to_string();
                    let name = t.get("name").and_then(|s| s.as_str()).unwrap_or(&slug).to_string();
                    Some(TeamOut { slug, name })
                })
                .collect()
        })
        .unwrap_or_default();
    let teams: Vec<String> = st.teams.iter().map(|t| t.slug.clone()).collect();
    if cfg.include_team && !teams.is_empty() {
        let team = client.get("/laps", &[base.as_slice(), &[("teams", teams.join(",")), ("limit", "10".into())]].concat())?;
        st.leaderboard = Client::items(team).iter().filter_map(|l| lap_info(l, &slug)).collect();
        if let Some(pb) = &st.personal_best {
            if !st.leaderboard.iter().any(|l| l.is_me) {
                st.leaderboard.push(pb.clone());
            }
        }
        st.leaderboard.sort_by(|a, b| a.lap_time.total_cmp(&b.lap_time));
        st.team_best = st.leaderboard.first().cloned();
    } else if let Some(pb) = &st.personal_best {
        st.leaderboard = vec![pb.clone()];
    }

    // Reference lap for the live delta: the team's best falls back to yours when its
    // telemetry is not shared.
    let wanted: Vec<(&LapInfo, &str)> = match cfg.reference.as_str() {
        "off" => vec![],
        "team" => st.team_best.iter().map(|l| (l, "team")).chain(st.personal_best.iter().map(|l| (l, "personal"))).collect(),
        _ => st.personal_best.iter().map(|l| (l, "personal")).collect(),
    };
    let mut notes = vec![];
    let mut tried = vec![];
    for (lap, source) in wanted {
        if tried.contains(&lap.id) {
            continue;
        }
        tried.push(lap.id.clone());
        match reference_lap(client, lap, source, cache) {
            Ok(r) => {
                st.reference = Some(r);
                break;
            }
            Err(e) => notes.push(format!("{}’s lap: {e}", lap.driver)),
        }
    }
    if cfg.reference != "off" && st.reference.is_none() && notes.is_empty() {
        notes.push("No lap to compare with yet".into());
    }
    st.reference_note = notes.join("; ");

    // References for the delta bars. Only laps whose telemetry you may see can be compared
    // with, so the searches ask for those.
    let fastest = |client: &Client, extra: &[(&'static str, String)]| -> Result<Option<LapInfo>, String> {
        let q = [base.as_slice(), extra, &[("limit", "1".into())]].concat();
        Ok(Client::items(client.get("/laps", &q)?).first().and_then(|l| lap_info(l, &slug)))
    };
    let telemetry = ("seeTelemetry", "true".to_string());
    for key in refs {
        let lap = match key.as_str() {
            "personal" => Ok(st.personal_best.clone()),
            "teams" if teams.is_empty() => Err("you are not in a Garage 61 team".to_string()),
            "teams" => fastest(client, &[("teams", teams.join(",")), telemetry.clone()]),
            "world" => {
                st.world_best = fastest(client, &[]).ok().flatten();
                fastest(client, std::slice::from_ref(&telemetry))
            }
            k => match k.strip_prefix("team:") {
                Some(t) if teams.iter().any(|x| x == t) => fastest(client, &[("teams", t.to_string()), telemetry.clone()]),
                Some(_) => Err("you are no longer in that team".to_string()),
                None => continue,
            },
        };
        let result = match lap {
            Ok(Some(l)) => reference_lap(client, &l, key, cache),
            Ok(None) => Err("no lap with shared telemetry for this car and track yet".to_string()),
            Err(e) => Err(e),
        };
        match result {
            Ok(r) => {
                st.references.insert(key.clone(), r);
            }
            Err(e) => {
                log::warn!("Garage 61 reference lap ({key}): {e}");
                st.reference_notes.insert(key.clone(), e);
            }
        }
    }
    Ok(st)
}

/// Background worker: refetches when the combo, settings or refresh interval change.
pub fn spawn(hub: Arc<Hub>, config: ConfigStore, combo: Arc<Mutex<Combo>>, data_dir: PathBuf) {
    std::thread::Builder::new()
        .name("garage61".into())
        .spawn(move || {
            let cache = data_dir.join("garage61");
            let mut last: Option<(Combo, Garage61Config, BTreeSet<String>)> = None;
            let mut last_fetch = Instant::now() - Duration::from_secs(3600);
            let mut client: Option<Client> = None;
            let publish = |s: &G61State| {
                hub.publish("garage61", &serde_json::to_string(s).unwrap_or_default());
            };
            loop {
                let (cfg, wanted) = config.read(|c| (c.garage61.clone(), wanted_refs(c)));
                let cur = combo.lock().unwrap().clone();
                // What changes the search. Conditions only count when filtering on them, and a
                // small drift in temperature is not worth a new search.
                let changed = last.as_ref().is_none_or(|(c, k, w)| {
                    *w != wanted
                        || c.car_id != cur.car_id
                        || c.track_id != cur.track_id
                        || k.token != cfg.token
                        || k.include_team != cfg.include_team
                        || k.period != cfg.period
                        || k.reference != cfg.reference
                        || k.match_conditions != cfg.match_conditions
                        || k.temp_range != cfg.temp_range
                        || cfg.match_conditions && (c.wet != cur.wet || (c.track_temp - cur.track_temp).abs() as f32 > cfg.temp_range / 2.0)
                });
                if !cfg.enabled {
                    publish(&G61State { status: "disabled".into(), message: "Enable Garage 61 in Settings → Garage 61".into(), ..Default::default() });
                    last = None;
                } else if cfg.token.trim().is_empty() {
                    publish(&G61State { status: "noToken".into(), message: "Add your Garage 61 token in Settings".into(), ..Default::default() });
                    last = None;
                } else if cur.car_id == 0 || cur.track_id == 0 {
                    publish(&G61State { status: "waiting".into(), message: "Waiting for a car and track".into(), ..Default::default() });
                } else if changed || last_fetch.elapsed() > Duration::from_secs(cfg.refresh_minutes.max(1) as u64 * 60) {
                    if client.as_ref().is_none_or(|c| c.token != cfg.token) {
                        client = Some(Client::new(&cfg.token));
                    }
                    publish(&G61State { status: "loading".into(), car: cur.car_name.clone(), track: cur.track_name.clone(), ..Default::default() });
                    let st = fetch(client.as_mut().unwrap(), &cur, &cfg, &wanted, &cache).unwrap_or_else(|e| {
                        log::warn!("Garage 61 ({} at {}): {e}", cur.car_name, cur.track_name);
                        G61State { status: "error".into(), message: e, ..Default::default() }
                    });
                    publish(&st);
                    last = Some((cur, cfg, wanted));
                    last_fetch = Instant::now();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
        .expect("spawn garage61 thread");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn delta_bars_ask_for_their_reference_laps() {
        use crate::config::OverlayInstance;
        let bar = |enabled: bool, settings: Value| OverlayInstance { kind: "delta".into(), enabled, settings, ..Default::default() };
        let c = Config {
            overlays: vec![
                bar(true, json!({})),
                bar(true, json!({ "reference": "g61Team", "team": "fast-crew" })),
                bar(true, json!({ "reference": "g61World" })),
                bar(true, json!({ "reference": "session" })),
                bar(false, json!({ "reference": "g61Teams" })),
            ],
            ..Default::default()
        };
        let w: Vec<String> = wanted_refs(&c).into_iter().collect();
        assert_eq!(w, vec!["personal", "team:fast-crew", "world"]);
    }

    #[test]
    fn matches_cars_and_tracks() {
        let cars = vec![
            json!({"id": 10, "name": "Porsche 911 GT3 R (992)", "platform": "iracing", "platform_id": "169"}),
            json!({"id": 11, "name": "BMW M4 GT3", "platform": "iracing"}),
        ];
        assert_eq!(match_car(&cars, 169, "whatever"), Some(10));
        assert_eq!(match_car(&cars, 1, "BMW M4 GT3"), Some(11));
        assert_eq!(match_car(&cars, 1, "Nope"), None);
        let tracks = vec![
            json!({"id": 1, "name": "Spa-Francorchamps", "variant": "Grand Prix Pits", "platform": "iracing"}),
            json!({"id": 2, "name": "Spa-Francorchamps", "variant": "Endurance", "platform": "iracing"}),
        ];
        assert_eq!(match_track(&tracks, 0, "Spa-Francorchamps", "Endurance"), Some(2));
        assert_eq!(match_track(&tracks, 0, "Circuit de Spa-Francorchamps", "x"), None);
    }

    #[test]
    fn parses_laps() {
        let l = json!({"id": "abc", "lapTime": 137.512, "startTime": "2026-05-01T10:00:00Z", "driver": {"firstName": "Max", "lastName": "V", "slug": "max-v"}});
        let i = lap_info(&l, "max-v").unwrap();
        assert_eq!(i.driver, "Max V");
        assert_eq!(i.date, "2026-05-01");
        assert_eq!(i.id, "abc");
        assert!(i.is_me);
    }

    #[test]
    fn builds_filters() {
        let mut cfg = Garage61Config::default();
        let combo = Combo { track_temp: 30, wet: false, ..Default::default() };
        let (q, d) = lap_filters(&cfg, &combo);
        assert_eq!(q, vec![("after", "2008-01-01T00:00:00Z".to_string())]);
        assert_eq!(d, "All time");
        cfg.period = "season".into();
        cfg.match_conditions = true;
        cfg.temp_range = 5.0;
        let (q, d) = lap_filters(&cfg, &combo);
        assert!(
            q.contains(&("age", "-1".into())) && q.contains(&("minConditionsTrackTemp", "25".into())) && q.contains(&("maxConditionsTrackWetness", "2".into()))
        );
        assert_eq!(d, "Current season · 25–35 °C · dry");
        cfg.period = "30".into();
        let (q, _) = lap_filters(&cfg, &Combo { wet: true, ..combo });
        assert!(q.contains(&("age", "30".into())) && q.contains(&("minConditionsTrackWetness", "3".into())));
    }

    /// A lap at 50 m/s for the first half and 25 m/s for the second, sampled at 60 Hz.
    fn csv(with_time: bool) -> (String, f64) {
        let len = 3000.0;
        let lap = 1500.0 / 50.0 + 1500.0 / 25.0;
        let mut s = String::from(if with_time { "Speed,Throttle,LapDistPct,LapCurrentLapTime\n" } else { "Speed,Throttle,LapDistPct\n" });
        // A couple of samples before the line, then the lap, then past the line.
        s += "50,1,0.9995,89.9\n";
        let mut t = 0.004;
        loop {
            let d = if t < 30.0 { t * 50.0 } else { 1500.0 + (t - 30.0) * 25.0 };
            let p = d / len;
            if p >= 1.0 {
                s += &format!("25,1,{:.6},0.01\n", p - 1.0);
                break;
            }
            let v = if t < 30.0 { 50 } else { 25 };
            if with_time {
                s += &format!("{v},1,{p:.6},{t:.4}\n");
            } else {
                s += &format!("{v},1,{p:.6}\n");
            }
            t += 1.0 / 60.0;
        }
        (s, lap)
    }

    #[test]
    fn reference_from_csv() {
        for with_time in [false, true] {
            let (s, lap) = csv(with_time);
            let r = reference_times(&s, lap).unwrap();
            assert_eq!(r.len(), REF_STEPS + 1);
            assert!(r[0].abs() < 0.05, "{with_time}: {}", r[0]);
            assert!((r[REF_STEPS / 2] - 30.0).abs() < 0.1, "{with_time}: {}", r[REF_STEPS / 2]);
            assert!((r[REF_STEPS] as f64 - lap).abs() < 0.01);
            assert!(r.windows(2).all(|w| w[1] >= w[0]));
        }
        assert!(reference_times("Throttle,Brake\n1,0\n", 90.0).is_none());
    }
}
