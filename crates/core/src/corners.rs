//! Track data for the track map — corner names / turn numbers, straights, pit entry and
//! exit — downloaded from the community Lovely Track Data project
//! (https://github.com/Lovely-Sim-Racing/lovely-track-data, CC BY-NC-SA 4.0) and cached per
//! TrackID. Users can override the corners in Settings → Track map.

use crate::config::Corner;
use crate::hub::{Command, Hub};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(crate) const BASE: &str = "https://raw.githubusercontent.com/Lovely-Sim-Racing/lovely-track-data/main/data/";

/// Everything the track map uses from a Lovely track file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TrackData {
    pub corners: Vec<Corner>,
    /// Named straights (number unused).
    pub straights: Vec<Corner>,
    pub pit_entry: Option<f32>,
    pub pit_exit: Option<f32>,
}

pub fn parse_track(v: &Value) -> TrackData {
    let frac = |k: &str| v.get(k).and_then(|x| x.as_f64()).map(|x| x as f32).filter(|x| (0.0..=1.0).contains(x));
    let straights = v
        .get("straight")
        .and_then(|t| t.as_array())
        .into_iter()
        .flatten()
        .filter_map(|t| {
            let start = t.get("start")?.as_f64()? as f32;
            let end = t.get("end").and_then(|e| e.as_f64()).map(|e| e as f32).unwrap_or(start);
            let name = t.get("name").and_then(|n| n.as_str()).unwrap_or("").trim().to_string();
            (!name.is_empty()).then_some(Corner { name, number: 0, start: start.clamp(0.0, 1.0), end: end.clamp(0.0, 1.0) })
        })
        .collect();
    TrackData { corners: parse_lovely(v), straights, pit_entry: frac("pitentry"), pit_exit: frac("pitexit") }
}

/// Turn list from a Lovely track file.
pub fn parse_lovely(v: &Value) -> Vec<Corner> {
    let turns = v.get("turn").and_then(|t| t.as_array()).cloned().unwrap_or_default();
    turns
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            let start = t.get("start")?.as_f64()? as f32;
            let end = t.get("end").and_then(|e| e.as_f64()).map(|e| e as f32).unwrap_or(start);
            let name = t.get("name").and_then(|n| n.as_str()).unwrap_or("").trim().to_string();
            // "Turn 3" / "T3" are just numbers.
            let lower = name.to_ascii_lowercase();
            let bare = lower
                .strip_prefix("turn")
                .or_else(|| lower.strip_prefix('t'))
                .map(str::trim)
                .filter(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()));
            let name = bare.map(String::from).unwrap_or(name);
            let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
            let number = t
                .get("number")
                .and_then(|n| n.as_i64())
                .map(|n| n as i32)
                .or_else(|| (name.len() <= 4 && !digits.is_empty()).then(|| digits.parse().ok()).flatten());
            let name = if name.len() <= 4 && !digits.is_empty() { String::new() } else { name };
            // A named corner without a number of its own gets none (0): the list is of named
            // corners, not of every turn, so counting them gives the wrong turn numbers (Spa's
            // Bus Stop would be "T15", not 18–19). The map shows its name instead. Only a corner
            // with neither is numbered by its place in the list.
            let number = number.unwrap_or(if name.is_empty() { i as i32 + 1 } else { 0 });
            Some(Corner { name, number, start: start.clamp(0.0, 1.0), end: end.clamp(0.0, 1.0) })
        })
        .collect()
}

/// Words and numbers of a track name ("charlotte 2025 roval2019" → charlotte, roval / 2025, 2019).
fn tokens(s: &str) -> (Vec<String>, Vec<String>) {
    let (mut words, mut nums) = (vec![], vec![]);
    let mut cur = String::new();
    let flush = |cur: &mut String, words: &mut Vec<String>, nums: &mut Vec<String>| {
        if !cur.is_empty() {
            if cur.chars().all(|c| c.is_ascii_digit()) {
                nums.push(std::mem::take(cur))
            } else {
                words.push(std::mem::take(cur))
            }
        }
    };
    for c in s.to_lowercase().chars() {
        if !c.is_ascii_alphanumeric() || cur.chars().last().is_some_and(|p| p.is_ascii_digit() != c.is_ascii_digit()) {
            flush(&mut cur, &mut words, &mut nums);
        }
        if c.is_ascii_alphanumeric() {
            cur.push(c);
        }
    }
    flush(&mut cur, &mut words, &mut nums);
    (words, nums)
}

/// Lovely file path for an iRacing track (its `TrackName`, e.g. "spa combined"). iRacing adds
/// years to names as tracks are updated ("charlotte 2025 roval2019" vs Lovely's
/// "charlotte 2018 2019 roval"), so when there is no exact match the entry with the same words
/// and the most years in common is used.
fn find_path(manifest: &Value, track_name: &str) -> Option<String> {
    let list = manifest.pointer("/tracks/iracing")?.as_array()?;
    let clean = |s: &str| s.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>();
    let id = |t: &Value| t.get("trackId").and_then(|i| i.as_str()).unwrap_or("").to_string();
    let path = |t: &Value| t.get("path").and_then(|p| p.as_str()).map(String::from);
    let want = clean(track_name);
    if let Some(t) = list.iter().find(|t| clean(&id(t)) == want) {
        return path(t);
    }
    let (words, nums) = tokens(track_name);
    let mut sorted = words.clone();
    sorted.sort();
    list.iter()
        .filter_map(|t| {
            let (w, n) = tokens(&id(t));
            let mut w2 = w.clone();
            w2.sort();
            (w2 == sorted && !w.is_empty()).then(|| (n.iter().filter(|x| nums.contains(x)).count() as i32 * 10 - n.len() as i32, t))
        })
        .max_by_key(|(score, _)| *score)
        .and_then(|(_, t)| path(t))
}

pub fn spawn(hub: Arc<Hub>, data_dir: PathBuf, track: Arc<Mutex<(i64, String)>>) {
    std::thread::Builder::new()
        .name("corners".into())
        .spawn(move || {
            let dir = data_dir.join("corners");
            let agent = crate::net::agent();
            let mut manifest: Option<Value> = None;
            let mut done: Option<i64> = None;
            let mut retry_at = Instant::now();
            loop {
                std::thread::sleep(Duration::from_secs(2));
                let (id, name) = track.lock().unwrap().clone();
                if id == 0 || done == Some(id) {
                    continue;
                }
                // v3: corners known only by name no longer get numbers from their place in the list.
                let cache = dir.join(format!("{id}.v3.json"));
                if let Some(data) = std::fs::read_to_string(&cache).ok().and_then(|s| serde_json::from_str::<TrackData>(&s).ok()) {
                    hub.command(Command::SetTrackData { track_id: id, data, source: "lovely" });
                    done = Some(id);
                    continue;
                }
                if Instant::now() < retry_at {
                    continue;
                }
                if manifest.is_none() {
                    manifest = agent.get(&format!("{BASE}manifest.json")).call().ok().and_then(|r| r.into_json().ok());
                }
                let Some(m) = &manifest else {
                    retry_at = Instant::now() + Duration::from_secs(600);
                    continue;
                };
                let Some(path) = find_path(m, &name) else {
                    log::info!("no Lovely track data for {name}");
                    done = Some(id);
                    continue;
                };
                match agent.get(&format!("{BASE}{path}")).call().ok().and_then(|r| r.into_json::<Value>().ok()) {
                    Some(v) => {
                        let data = parse_track(&v);
                        let _ = std::fs::create_dir_all(&dir);
                        let _ = std::fs::write(&cache, serde_json::to_string(&data).unwrap_or_default());
                        hub.command(Command::SetTrackData { track_id: id, data, source: "lovely" });
                        done = Some(id);
                    }
                    None => retry_at = Instant::now() + Duration::from_secs(600),
                }
            }
        })
        .expect("spawn corners thread");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_turns() {
        let v = json!({"turn": [
            {"start": 0.05, "end": 0.08, "name": "T1"},
            {"start": 0.1, "end": 0.12, "name": "Eau Rouge", "number": 2},
            {"start": 0.2, "end": 0.25, "name": "Kemmel"}
        ]});
        let c = parse_lovely(&v);
        assert_eq!(c[0], Corner { name: "".into(), number: 1, start: 0.05, end: 0.08 });
        assert_eq!(c[1].name, "Eau Rouge");
        assert_eq!(c[1].number, 2);
        // Named without a number: no number made up from its place in the list.
        assert_eq!((c[2].number, c[2].name.as_str()), (0, "Kemmel"));
    }

    #[test]
    fn finds_track_in_manifest() {
        let m = json!({"tracks": {"iracing": [{"trackId": "spa combined", "path": "iracing/spa-combined.json"}]}});
        assert_eq!(find_path(&m, "spa combined").as_deref(), Some("iracing/spa-combined.json"));
        assert_eq!(find_path(&m, "monza full"), None);
        let m = json!({"tracks": {"iracing": [
            {"trackId": "charlotte 2018 2019 roval", "path": "iracing/charlotte-2018-2019-roval.json"},
            {"trackId": "charlotte oval", "path": "x"},
            {"trackId": "lemans full", "path": "lm"}
        ]}});
        assert_eq!(find_path(&m, "charlotte 2025 roval2019").as_deref(), Some("iracing/charlotte-2018-2019-roval.json"));
        assert_eq!(find_path(&m, "lemans nochicane"), None);
    }

    #[test]
    fn parses_straights_and_pits() {
        let v = json!({"pitentry": 0.91, "pitexit": 0.11, "turn": [{"start": 0.03, "end": 0.08, "name": "Turn 1"}],
            "straight": [{"start": 0.5, "end": 0.6, "name": "Mulsanne"}]});
        let d = parse_track(&v);
        assert_eq!((d.pit_entry, d.pit_exit), (Some(0.91), Some(0.11)));
        assert_eq!(d.straights[0].name, "Mulsanne");
        assert_eq!((d.corners[0].number, d.corners[0].name.as_str()), (1, ""));
    }
}
