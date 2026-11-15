//! Le Mans Ultimate extras, fetched in the background while LMU is the game being read:
//!
//! - the weather forecast, from the game's local REST API (`/rest/sessions/weather`): for each
//!   session (practice, qualifying, race) five points at the start and 20, 40, 60 and 80% of
//!   the way through, each with the sky (LMU's 0-10 cloud scale), air temperature (°C) and
//!   chance of rain (%);
//! - Lovely Sim Racing's LMU track data (corners, straights, pit entry and exit), as for iRacing;
//! - an outline from the shared map library, so a track mapped once is there for everyone.

use crate::engine::trackmap::TrackMap;
use crate::hub::{Command, Hub};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// The game's local REST API.
const REST: &str = "http://127.0.0.1:6397";

/// One forecast point: from `frac` of the session on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Node {
    pub frac: f32,
    /// LMU's sky: 0 clear, 1 light clouds, 2 partly cloudy, 3 mostly cloudy, 4 overcast,
    /// 5 cloudy & drizzle, 6 cloudy & light rain, 7 overcast & light rain, 8 overcast & rain,
    /// 9 overcast & heavy rain, 10 overcast & storm.
    pub sky: i32,
    /// Air temperature, °C.
    pub temp: f32,
    /// Chance of rain, 0-1.
    pub rain: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Forecast {
    pub practice: Vec<Node>,
    pub qualify: Vec<Node>,
    pub race: Vec<Node>,
}

impl Forecast {
    /// The forecast for a session type (iRacing-style SessionType: "Race", "Lone Qualify"...).
    pub fn for_session(&self, kind: &str) -> &[Node] {
        let k = kind.to_lowercase();
        if k.contains("race") || k.contains("warm") {
            &self.race
        } else if k.contains("qual") {
            &self.qualify
        } else {
            &self.practice
        }
    }
}

const NODES: [(f32, &str); 5] = [(0.0, "START"), (0.2, "NODE_25"), (0.4, "NODE_50"), (0.6, "NODE_75"), (0.8, "FINISH")];

/// Reads the REST API's weather answer: `{"RACE": {"START": {"WNV_SKY": {"currentValue": 2}, ...}}}`.
pub fn parse_forecast(v: &Value) -> Forecast {
    let num = |n: &Value, field: &str| {
        let c = n.get(field)?.get("currentValue")?;
        c.as_f64().or_else(|| c.as_str()?.trim().parse().ok())
    };
    let session = |key: &str| -> Vec<Node> {
        let Some(s) = v.get(key) else { return vec![] };
        NODES
            .iter()
            .filter_map(|&(frac, node)| {
                let n = s.get(node)?;
                Some(Node {
                    frac,
                    sky: num(n, "WNV_SKY")?.round().clamp(0.0, 10.0) as i32,
                    temp: num(n, "WNV_TEMPERATURE")? as f32,
                    rain: (num(n, "WNV_RAIN_CHANCE")? * 0.01).clamp(0.0, 1.0) as f32,
                })
            })
            .collect()
    };
    Forecast { practice: session("PRACTICE"), qualify: session("QUALIFY"), race: session("RACE") }
}

/// Lovely's file for an LMU track name ("Circuit de la Sarthe" → "lmu/circuit-de-la-sarthe.json"),
/// from its manifest. Names are compared without case or punctuation; failing an exact match,
/// the Lovely track whose name the game's starts with (LMU adds layout words: "... Mulsanne").
pub fn find_lovely(manifest: &Value, name: &str) -> Option<String> {
    let list = manifest.pointer("/tracks/lmu")?.as_array()?;
    let clean = |s: &str| s.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>();
    let want = clean(name);
    if want.is_empty() {
        return None;
    }
    let keys = |t: &Value| ["trackId", "name"].iter().filter_map(|k| t.get(*k)?.as_str().map(clean)).collect::<Vec<_>>();
    let path = |t: &Value| t.get("path").and_then(|p| p.as_str()).map(String::from);
    if let Some(t) = list.iter().find(|t| keys(t).contains(&want)) {
        return path(t);
    }
    list.iter()
        .filter_map(|t| keys(t).into_iter().filter(|k| k.len() >= 6 && want.starts_with(k.as_str())).map(|k| k.len()).max().map(|n| (n, t)))
        .max_by_key(|(n, _)| *n)
        .and_then(|(_, t)| path(t))
}

/// Starts the background fetches. `track` is the LMU track being driven ((0, "") otherwise).
pub fn spawn(hub: Arc<Hub>, data_dir: PathBuf, track: Arc<Mutex<(i64, String)>>) {
    let _ = std::thread::Builder::new().name("lmu-extras".into()).spawn(move || {
        let web = crate::net::agent();
        // The game is on this PC: short timeouts, no proxy.
        let local = ureq::AgentBuilder::new().timeout(Duration::from_secs(2)).try_proxy_from_env(false).build();
        let corners_dir = data_dir.join("corners");
        let library_dir = data_dir.join("trackmaps").join("library");
        let read = |p: &std::path::Path| std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok());
        let mut manifest: Option<Value> = None;
        let mut done: i64 = 0;
        let mut forecast_at: Option<Instant> = None;
        let mut forecast_id: i64 = 0;
        let mut last: Option<Forecast> = None;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let (id, name) = track.lock().unwrap().clone();
            if id == 0 {
                if last.take().is_some() {
                    hub.command(Command::SetLmuForecast(None));
                }
                continue;
            }
            // The forecast: every 30 s (it changes as the game rolls its weather), and straight
            // away on a new track.
            if forecast_id != id || forecast_at.is_none_or(|t| t.elapsed() > Duration::from_secs(30)) {
                forecast_id = id;
                forecast_at = Some(Instant::now());
                let f = local.get(&format!("{REST}/rest/sessions/weather")).call().ok().and_then(|r| r.into_json::<Value>().ok()).map(|v| parse_forecast(&v));
                if let Some(f) = f.filter(|f| !(f.practice.is_empty() && f.qualify.is_empty() && f.race.is_empty())) {
                    // Sent every time, so the engine has it again soon after it starts afresh.
                    last = Some(f.clone());
                    hub.command(Command::SetLmuForecast(Some(f)));
                }
            }
            if done == id {
                continue;
            }
            done = id;
            // Lovely track data (cached like iRacing's).
            let cache = corners_dir.join(format!("{id}.v2.json"));
            if let Some(data) = std::fs::read_to_string(&cache).ok().and_then(|s| serde_json::from_str::<crate::corners::TrackData>(&s).ok()) {
                hub.command(Command::SetTrackData { track_id: id, data, source: "lovely" });
            } else {
                if manifest.is_none() {
                    manifest = web.get(&format!("{}manifest.json", crate::corners::BASE)).call().ok().and_then(|r| r.into_json().ok());
                }
                if let Some(path) = manifest.as_ref().and_then(|m| find_lovely(m, &name)) {
                    if let Some(v) = web.get(&format!("{}{path}", crate::corners::BASE)).call().ok().and_then(|r| r.into_json::<Value>().ok()) {
                        let data = crate::corners::parse_track(&v);
                        let _ = std::fs::create_dir_all(&corners_dir);
                        let _ = std::fs::write(&cache, serde_json::to_string(&data).unwrap_or_default());
                        hub.command(Command::SetTrackData { track_id: id, data, source: "lovely" });
                    }
                } else {
                    log::info!("no Lovely track data for LMU track {name}");
                }
            }
            // Shared map library, unless this PC already has the track mapped.
            if !data_dir.join("trackmaps").join(format!("{id}.json")).exists() {
                let cached = library_dir.join(format!("{id}.json"));
                let map = read(&cached).and_then(|v| serde_json::from_value::<TrackMap>(v).ok()).or_else(|| {
                    let index_path = library_dir.join("index.json");
                    let fresh = std::fs::metadata(&index_path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| SystemTime::now().duration_since(t).ok())
                        .is_some_and(|age| age < Duration::from_secs(86_400));
                    let index = read(&index_path).filter(|_| fresh).or_else(|| {
                        let v = web.get(&format!("{}/index.json", crate::svgmap::LIBRARY_URL)).call().ok()?.into_json::<Value>().ok()?;
                        let _ = std::fs::create_dir_all(&library_dir);
                        let _ = std::fs::write(&index_path, v.to_string());
                        Some(v)
                    })?;
                    index.pointer(&format!("/tracks/{id}"))?;
                    let m = web.get(&format!("{}/{id}.json", crate::svgmap::LIBRARY_URL)).call().ok()?.into_json::<TrackMap>().ok()?;
                    (m.track_id == id && m.points.len() >= 50).then(|| {
                        let _ = std::fs::write(&cached, serde_json::to_string(&m).unwrap_or_default());
                        m
                    })
                });
                if let Some(m) = map {
                    log::info!("LMU track map for {name} from the shared library");
                    hub.command(Command::SetLibraryMap(TrackMap { source: "library".into(), ..m }));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_forecast() {
        let node = |sky: i64, temp: f64, rain: &str| json!({ "WNV_SKY": { "currentValue": sky }, "WNV_TEMPERATURE": { "currentValue": temp }, "WNV_RAIN_CHANCE": { "currentValue": rain } });
        let v = json!({
            "RACE": { "START": node(1, 22.5, "0"), "NODE_25": node(3, 21.0, "20"), "NODE_50": node(8, 18.0, "80"), "NODE_75": node(6, 18.5, "60"), "FINISH": node(2, 20.0, "10") },
            "QUALIFY": { "START": node(0, 25.0, "0") },
        });
        let f = parse_forecast(&v);
        assert_eq!(f.race.len(), 5);
        assert_eq!(f.race[2], Node { frac: 0.4, sky: 8, temp: 18.0, rain: 0.8 });
        assert_eq!(f.qualify.len(), 1);
        assert!(f.practice.is_empty());
        assert_eq!(f.for_session("Race").len(), 5);
        assert_eq!(f.for_session("Warmup").len(), 5);
        assert_eq!(f.for_session("Lone Qualify").len(), 1);
        assert!(f.for_session("Practice").is_empty());
    }

    #[test]
    fn finds_lovely_lmu_tracks() {
        let m = json!({ "tracks": { "lmu": [
            { "trackId": "circuit-de-la-sarthe", "name": "Circuit de la Sarthe", "path": "lmu/circuit-de-la-sarthe.json" },
            { "trackId": "fuji-speedway", "name": "Fuji Speedway", "path": "lmu/fuji-speedway.json" },
        ] } });
        assert_eq!(find_lovely(&m, "Circuit de la Sarthe").as_deref(), Some("lmu/circuit-de-la-sarthe.json"));
        assert_eq!(find_lovely(&m, "FUJI SPEEDWAY").as_deref(), Some("lmu/fuji-speedway.json"));
        // A layout name that adds words to the track's.
        assert_eq!(find_lovely(&m, "Circuit de la Sarthe Mulsanne").as_deref(), Some("lmu/circuit-de-la-sarthe.json"));
        assert_eq!(find_lovely(&m, "Monza"), None);
    }
}
