//! iRacing Data API client (https://members-ng.iracing.com/data), used to import
//! leagues: name, roster and nicknames, car classes, points system and season standings.
//!
//! iRacing requires OAuth. This uses the "password limited" grant, which needs a
//! client ID/secret registered with iRacing plus the user's iRacing login. Both
//! secrets are sent (and stored) masked as base64(sha256(secret + lowercase(id))).

use crate::config::{ClassRules, DriverOverride, IracingApiConfig, League, LeagueClass, PointsConfig, SeasonEntry};
use crate::engine::ratings::parse_points_table;
use base64::Engine as _;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

const TOKEN_URL: &str = "https://oauth.iracing.com/oauth2/token";
const DATA_URL: &str = "https://members-ng.iracing.com/data/";
const PALETTE: [&str; 8] = ["#ff4d6d", "#3fa7ff", "#ffd23f", "#3ddc84", "#b26bff", "#ff8c42", "#2ee6d6", "#f2f4f8"];

/// iRacing's secret masking: base64(sha256(secret + identifier.trim().to_lowercase())).
pub fn mask(secret: &str, identifier: &str) -> String {
    let mut h = Sha256::new();
    h.update(secret.as_bytes());
    h.update(identifier.trim().to_lowercase().as_bytes());
    base64::engine::general_purpose::STANDARD.encode(h.finalize())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonSummary {
    pub season_id: i64,
    pub name: String,
    pub active: bool,
}

pub struct Client {
    agent: ureq::Agent,
    cfg: IracingApiConfig,
    token: Option<(String, Instant)>,
}

impl Client {
    pub fn new(cfg: IracingApiConfig) -> Self {
        Self { agent: crate::net::agent(), cfg, token: None }
    }

    fn token(&mut self) -> Result<String, String> {
        if let Some((t, until)) = &self.token {
            if Instant::now() < *until {
                return Ok(t.clone());
            }
        }
        if !self.cfg.configured() {
            return Err("Add your iRacing API credentials first".into());
        }
        let resp = self.agent.post(TOKEN_URL).send_form(&[
            ("grant_type", "password_limited"),
            ("client_id", &self.cfg.client_id),
            ("client_secret", &self.cfg.client_secret_masked),
            ("username", &self.cfg.username),
            ("password", &self.cfg.password_masked),
            ("scope", "iracing.auth"),
        ]);
        let v: Value = match resp {
            Ok(r) => r.into_json().map_err(|e| e.to_string())?,
            Err(ureq::Error::Status(code, r)) => {
                let body: Value = r.into_json().unwrap_or_default();
                let msg = body.get("error_description").or(body.get("error")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                return Err(format!("iRacing login failed ({code}) {msg}"));
            }
            Err(e) => return Err(e.to_string()),
        };
        let token = v.get("access_token").and_then(|t| t.as_str()).ok_or("iRacing login returned no token")?.to_string();
        let ttl = v.get("expires_in").and_then(|t| t.as_u64()).unwrap_or(600).saturating_sub(30);
        self.token = Some((token.clone(), Instant::now() + Duration::from_secs(ttl)));
        Ok(token)
    }

    /// GET a /data endpoint, following the S3 `link` indirection most endpoints use.
    pub fn get(&mut self, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
        let token = self.token()?;
        let mut req = self.agent.get(&format!("{DATA_URL}{path}")).set("Authorization", &format!("Bearer {token}"));
        for (k, v) in query {
            req = req.query(k, v);
        }
        let v: Value = match req.call() {
            Ok(r) => r.into_json().map_err(|e| e.to_string())?,
            Err(ureq::Error::Status(401, _)) => {
                self.token = None;
                return Err("iRacing rejected the login (401)".into());
            }
            Err(ureq::Error::Status(code, _)) => return Err(format!("iRacing API error {code} on {path}")),
            Err(e) => return Err(e.to_string()),
        };
        match v.get("link").and_then(|l| l.as_str()) {
            Some(link) => self.agent.get(link).call().map_err(|e| e.to_string())?.into_json().map_err(|e| e.to_string()),
            None => Ok(v),
        }
    }

    pub fn seasons(&mut self, league_id: i64) -> Result<Vec<Value>, String> {
        let v = self.get("league/seasons", &[("league_id", league_id.to_string()), ("retired", "0".into())])?;
        Ok(v.get("seasons").and_then(|s| s.as_array()).cloned().unwrap_or_default())
    }
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn i(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64()).unwrap_or(0)
}
fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(|x| x.as_array()).map(|a| a.as_slice()).unwrap_or(&[])
}

pub fn summarize_seasons(seasons: &[Value]) -> Vec<SeasonSummary> {
    seasons
        .iter()
        .map(|x| SeasonSummary { season_id: i(x, "season_id"), name: s(x, "season_name"), active: x.get("active").and_then(|a| a.as_bool()).unwrap_or(false) })
        .collect()
}

/// Build a league definition from the raw API responses (kept separate from HTTP for testing).
pub fn build_league(league: &Value, season: Option<&Value>, sessions: &[Value], standings: Option<&Value>) -> League {
    // Classes: every car class raced in the season's sessions, else the classes that score points.
    let mut classes: Vec<LeagueClass> = vec![];
    let mut add = |class_id: i64, name: String, car_id: i64, car_name: String| {
        let id = if class_id > 0 { format!("ir{class_id}") } else { format!("car{car_id}") };
        let n = classes.len();
        let c = match classes.iter_mut().find(|c| c.id == id) {
            Some(c) => c,
            None => {
                classes.push(LeagueClass {
                    id: id.clone(),
                    name: if name.is_empty() { car_name.clone() } else { name },
                    color: PALETTE[n % PALETTE.len()].into(),
                    rules: ClassRules { iracing_class_ids: if class_id > 0 { vec![class_id] } else { vec![] }, ..Default::default() },
                    role: String::new(),
                });
                classes.last_mut().unwrap()
            }
        };
        if car_id > 0 && !c.rules.car_ids.contains(&car_id) {
            c.rules.car_ids.push(car_id);
        }
    };
    for sess in sessions {
        for car in arr(sess, "cars") {
            add(i(car, "car_class_id"), s(car, "car_class_name"), i(car, "car_id"), s(car, "car_name"));
        }
    }
    if let Some(season) = season.filter(|_| sessions.iter().all(|x| arr(x, "cars").is_empty())) {
        for cc in arr(season, "driver_points_car_classes") {
            for car in arr(cc, "cars_in_class") {
                add(i(cc, "car_class_id"), s(cc, "name"), i(car, "car_id"), s(car, "car_name"));
            }
        }
    }
    // The whole roster: names and league car numbers, nicknames as display names.
    let driver_overrides = arr(league, "roster")
        .iter()
        .map(|m| DriverOverride {
            customer_id: i(m, "cust_id"),
            name: s(m, "display_name"),
            car_number: m.get("car_number").map(|n| n.as_str().map(String::from).unwrap_or_else(|| n.to_string())).filter(|n| n != "null").unwrap_or_default(),
            display_name: s(m, "nick_name").trim().to_string(),
            ..Default::default()
        })
        .collect();
    let (system_name, description) = season.map(|x| (s(x, "points_system_name"), s(x, "points_system_desc"))).unwrap_or_default();
    let mut table = parse_points_table(&description);
    if table.is_empty() {
        table = parse_points_table(&system_name);
    }
    let season_points = standings
        .and_then(|v| v.pointer("/standings/driver_standings"))
        .and_then(|d| d.as_array())
        .map(|a| {
            a.iter()
                .map(|e| SeasonEntry {
                    customer_id: e.pointer("/driver/cust_id").and_then(|c| c.as_i64()).unwrap_or(0),
                    name: e.pointer("/driver/display_name").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                    points: e.get("total_points").and_then(|p| p.as_f64()).unwrap_or(0.0) as f32,
                    position: e.get("position").and_then(|p| p.as_i64()).unwrap_or(0) as i32,
                })
                .collect()
        })
        .unwrap_or_default();
    League {
        id: format!("irl-{}", i(league, "league_id")),
        name: s(league, "league_name"),
        fallback: "iracing".into(),
        iracing_league_id: i(league, "league_id"),
        season_id: season.map(|x| i(x, "season_id")).unwrap_or(0),
        season_name: season.map(|x| s(x, "season_name")).unwrap_or_default(),
        points: PointsConfig {
            basis: if classes.len() > 1 { "class".into() } else { "overall".into() },
            table,
            system_name,
            description,
            ..Default::default()
        },
        classes,
        driver_overrides,
        season_points,
        imported_at: String::new(),
        strip_name_class: false,
        teams: vec![],
    }
}

/// Fetch everything for a league (and season: the given one, else the active one).
pub fn import(client: &mut Client, league_id: i64, season_id: Option<i64>) -> Result<(League, Vec<SeasonSummary>), String> {
    let league = client.get("league/get", &[("league_id", league_id.to_string()), ("include_licenses", "false".into())])?;
    if i(&league, "league_id") == 0 {
        return Err(format!("League {league_id} not found or not visible to your account"));
    }
    let seasons = client.seasons(league_id)?;
    let season = match season_id {
        Some(id) => seasons.iter().find(|x| i(x, "season_id") == id),
        None => seasons.iter().find(|x| x.get("active").and_then(|a| a.as_bool()).unwrap_or(false)).or(seasons.last()),
    };
    let (mut sessions, mut standings) = (vec![], None);
    if let Some(season) = season {
        let q = [("league_id", league_id.to_string()), ("season_id", i(season, "season_id").to_string())];
        sessions = client.get("league/season_sessions", &q).map(|v| arr(&v, "sessions").to_vec()).unwrap_or_default();
        standings = client.get("league/season_standings", &q).ok();
    }
    Ok((build_league(&league, season, &sessions, standings.as_ref()), summarize_seasons(&seasons)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn masks_like_iracing() {
        // base64(sha256("MyPassWordjohn.smith@example.com"))
        let m = mask("MyPassWord", "  John.Smith@Example.com ");
        let mut h = Sha256::new();
        h.update(b"MyPassWordjohn.smith@example.com");
        assert_eq!(m, base64::engine::general_purpose::STANDARD.encode(h.finalize()));
        assert_eq!(m.len(), 44);
    }

    #[test]
    fn builds_league() {
        let league = json!({"league_id": 42, "league_name": "Sunday Endurance", "roster": [
            {"cust_id": 1, "display_name": "Max Power", "nick_name": "Maxi"},
            {"cust_id": 2, "display_name": "Jo Bloggs", "nick_name": ""}
        ]});
        let season =
            json!({"season_id": 7, "season_name": "S7", "active": true, "points_system_name": "League points", "points_system_desc": "Points: 30-25-21-18-16"});
        let sessions = vec![json!({"cars": [
            {"car_id": 10, "car_name": "Porsche 963", "car_class_id": 4029, "car_class_name": "GTP"},
            {"car_id": 11, "car_name": "BMW M4 GT3", "car_class_id": 2708, "car_class_name": "GT3"},
            {"car_id": 12, "car_name": "Ferrari 296", "car_class_id": 2708, "car_class_name": "GT3"}
        ]})];
        let standings =
            json!({"standings": {"driver_standings": [{"position": 1, "total_points": 120.0, "driver": {"cust_id": 1, "display_name": "Max Power"}}]}});
        let l = build_league(&league, Some(&season), &sessions, Some(&standings));
        assert_eq!(l.name, "Sunday Endurance");
        assert_eq!(l.classes.len(), 2);
        assert_eq!(l.classes[1].rules.car_ids, vec![11, 12]);
        assert_eq!(l.classes[1].rules.iracing_class_ids, vec![2708]);
        assert_eq!(l.driver_overrides.len(), 2);
        assert_eq!(l.driver_overrides[0].display_name, "Maxi");
        assert_eq!(l.driver_overrides[1].name, "Jo Bloggs");
        assert_eq!(l.points.table, vec![30.0, 25.0, 21.0, 18.0, 16.0]);
        assert_eq!(l.points.basis, "class");
        assert_eq!(l.season_points[0].points, 120.0);
    }
}
