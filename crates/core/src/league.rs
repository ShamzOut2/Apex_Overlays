//! League builder: user-defined classes that override iRacing's car classes,
//! plus per-driver display overrides.

use crate::config::{League, LeagueClass, LeagueTeam};
use crate::model::Driver;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassKey {
    pub id: String,
    pub name: String,
    pub color: u32,
    /// Lower sorts first (faster classes first for iRacing classes).
    pub order: f32,
}

#[derive(Default, Clone)]
pub struct Resolver {
    league: Option<League>,
}

impl Resolver {
    pub fn new(league: Option<League>) -> Self {
        Self { league }
    }

    pub fn league(&self) -> Option<&League> {
        self.league.as_ref()
    }

    pub fn league_name(&self) -> Option<&str> {
        self.league.as_ref().map(|l| l.name.as_str())
    }

    pub fn class_of(&self, d: &Driver) -> ClassKey {
        let iracing = || ClassKey {
            id: format!("ir{}", d.class_id),
            name: if d.class_name.is_empty() { d.car_short.clone() } else { d.class_name.clone() },
            color: d.class_color,
            order: 1000.0 + if d.class_est_lap > 0.0 { d.class_est_lap } else { 999.0 },
        };
        let Some(league) = &self.league else { return iracing() };
        let key = |i: usize, c: &LeagueClass| ClassKey {
            id: format!("lg{}", c.id),
            name: c.name.clone(),
            color: parse_color(&c.color).unwrap_or(0xffffff),
            order: i as f32,
        };
        // A class picked for the driver on the league roster wins over the rules.
        // Secondary classes are never anyone's main class.
        if let Some(o) = league.driver_overrides.iter().find(|o| o.customer_id == d.user_id && !o.class_id.is_empty()) {
            if let Some((i, c)) = league.classes.iter().enumerate().find(|(_, c)| c.id == o.class_id && !c.is_secondary()) {
                return key(i, c);
            }
        }
        for (i, c) in league.classes.iter().enumerate() {
            if !c.is_secondary() && matches(c, d) {
                return key(i, c);
            }
        }
        if league.fallback == "other" {
            ClassKey { id: "other".into(), name: "Other".into(), color: 0x888888, order: 900.0 }
        } else {
            iracing()
        }
    }

    /// The driver's secondary class (picked for them on the roster), if they have one.
    pub fn secondary_of(&self, d: &Driver) -> Option<ClassKey> {
        let league = self.league.as_ref()?;
        let o = league.driver_overrides.iter().find(|o| o.customer_id == d.user_id && !o.class2_id.is_empty())?;
        let (i, c) = league.classes.iter().enumerate().find(|(_, c)| c.id == o.class2_id && c.is_secondary())?;
        Some(ClassKey { id: format!("lg{}", c.id), name: c.name.clone(), color: parse_color(&c.color).unwrap_or(0xffffff), order: i as f32 })
    }

    /// League car number for a driver, if the roster sets one.
    pub fn car_number(&self, d: &Driver) -> String {
        self.league
            .as_ref()
            .and_then(|l| l.driver_overrides.iter().find(|o| o.customer_id == d.user_id && !o.car_number.trim().is_empty()))
            .map(|o| o.car_number.trim().to_string())
            .unwrap_or_else(|| d.car_number.clone())
    }

    /// The league team a driver races for: by the team name shown for them (their roster
    /// row's, else iRacing's), ignoring case and spaces at the ends.
    pub fn team_of(&self, team_name: &str) -> Option<&LeagueTeam> {
        let name = team_name.trim();
        if name.is_empty() {
            return None;
        }
        self.league.as_ref()?.teams.iter().find(|t| t.name.trim().eq_ignore_ascii_case(name))
    }

    /// (display name, team name, hidden)
    pub fn display(&self, d: &Driver) -> (String, String, bool) {
        let strip = self.league.as_ref().is_some_and(|l| l.strip_name_class);
        let clean = |n: &str| if strip { strip_name_class(n) } else { n.to_string() };
        if let Some(o) = self.league.as_ref().and_then(|l| l.driver_overrides.iter().find(|o| o.customer_id == d.user_id)) {
            let name = if o.display_name.is_empty() { clean(&d.name) } else { o.display_name.clone() };
            let team = if o.team_name.is_empty() { d.team_name.clone() } else { o.team_name.clone() };
            return (name, team, o.hidden);
        }
        (clean(&d.name), d.team_name.clone(), false)
    }
}

/// A name without the class tags leagues add in brackets: "(P) Max Power", "(PA)Max Power"
/// and "Max Power (P)" all become "Max Power". A name that is only a tag is kept.
pub fn strip_name_class(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut depth = 0;
    for ch in name.chars() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.is_empty() {
        name.trim().to_string()
    } else {
        out
    }
}

fn matches(c: &LeagueClass, d: &Driver) -> bool {
    let r = &c.rules;
    r.car_ids.contains(&d.car_id)
        || r.customer_ids.contains(&d.user_id)
        || r.iracing_class_ids.contains(&d.class_id)
        || r.car_numbers.iter().any(|n| n.trim() == d.car_number)
        || r.car_names.iter().any(|p| glob(p, &d.car_name) || glob(p, &d.car_short) || glob(p, &d.car_path))
        || r.team_names.iter().any(|p| glob(p, &d.team_name))
        || r.driver_names.iter().any(|p| glob(p, &d.name))
}

pub fn parse_color(s: &str) -> Option<u32> {
    let h = s.trim().trim_start_matches('#').trim_start_matches("0x");
    match h.len() {
        6 => u32::from_str_radix(h, 16).ok(),
        3 => {
            let v = u32::from_str_radix(h, 16).ok()?;
            let (r, g, b) = ((v >> 8) & 0xf, (v >> 4) & 0xf, v & 0xf);
            Some(((r * 17) << 16) | ((g * 17) << 8) | (b * 17))
        }
        _ => None,
    }
}

/// Case-insensitive glob with `*` and `?`. A pattern without wildcards matches as a substring.
pub fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.trim().to_lowercase().chars().collect();
    if p.is_empty() {
        return false;
    }
    let t: Vec<char> = text.to_lowercase().chars().collect();
    if !p.iter().any(|&c| c == '*' || c == '?') {
        let ps: String = p.iter().collect();
        return text.to_lowercase().contains(&ps);
    }
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_class_tags_from_names() {
        assert_eq!(strip_name_class("(P) Max Power"), "Max Power");
        assert_eq!(strip_name_class("(PA)Max  Power"), "Max Power");
        assert_eq!(strip_name_class("Max Power (Am)"), "Max Power");
        assert_eq!(strip_name_class("Max Power"), "Max Power");
        assert_eq!(strip_name_class("(P)"), "(P)");
    }
    use crate::config::{ClassRules, DriverOverride};

    #[test]
    fn globbing() {
        assert!(glob("porsche*", "Porsche 911 GT3 R"));
        assert!(glob("*gt3*", "BMW M4 GT3"));
        assert!(glob("gt3", "BMW M4 GT3"));
        assert!(!glob("gt4*", "BMW M4 GT3"));
        assert!(glob("car ?", "Car 7"));
    }

    #[test]
    fn secondary_classes_run_alongside_the_main_ones() {
        let rookie = LeagueClass {
            id: "rookie".into(),
            name: "Rookie Cup".into(),
            color: "#00ff00".into(),
            role: "secondary".into(),
            rules: ClassRules { car_numbers: vec!["1".into()], ..Default::default() },
        };
        let pro = LeagueClass {
            id: "pro".into(),
            name: "Pro".into(),
            color: "#ff0000".into(),
            rules: ClassRules { car_numbers: vec!["1".into()], ..Default::default() },
            ..Default::default()
        };
        let league = League {
            classes: vec![rookie, pro],
            driver_overrides: vec![
                DriverOverride { customer_id: 7, class2_id: "rookie".into(), ..Default::default() },
                // A secondary class picked as the main one by mistake: not used as such.
                DriverOverride { customer_id: 8, class_id: "rookie".into(), class2_id: "pro".into(), ..Default::default() },
            ],
            ..Default::default()
        };
        let r = Resolver::new(Some(league));
        let d = |id: i64| Driver { user_id: id, car_number: "1".into(), class_id: 3, ..Default::default() };
        // The rules of a secondary class pick nobody: the main class is Pro.
        assert_eq!(r.class_of(&d(7)).name, "Pro");
        assert_eq!(r.secondary_of(&d(7)).map(|c| (c.name, c.color)), Some(("Rookie Cup".into(), 0x00ff00)));
        assert_eq!(r.class_of(&d(8)).name, "Pro");
        assert!(r.secondary_of(&d(8)).is_none(), "a main class is not a secondary one");
        assert!(r.secondary_of(&d(9)).is_none());
    }

    #[test]
    fn league_classes_override_iracing() {
        let league = League {
            id: "l".into(),
            name: "Endurance".into(),
            classes: vec![
                LeagueClass {
                    id: "pro".into(),
                    name: "Pro".into(),
                    color: "#f00".into(),
                    rules: ClassRules { car_numbers: vec!["1".into()], ..Default::default() },
                    ..Default::default()
                },
                LeagueClass {
                    id: "am".into(),
                    name: "Am".into(),
                    color: "#00ff00".into(),
                    rules: ClassRules { team_names: vec!["*amateur*".into()], ..Default::default() },
                    ..Default::default()
                },
            ],
            driver_overrides: vec![DriverOverride { customer_id: 42, display_name: "Ace".into(), ..Default::default() }],
            fallback: "other".into(),
            ..Default::default()
        };
        let r = Resolver::new(Some(league));
        let d = |num: &str, team: &str| Driver { car_number: num.into(), team_name: team.into(), class_id: 7, ..Default::default() };
        assert_eq!(r.class_of(&d("1", "x")).name, "Pro");
        assert_eq!(r.class_of(&d("1", "x")).color, 0xff0000);
        assert_eq!(r.class_of(&d("5", "Amateur Hour")).name, "Am");
        assert_eq!(r.class_of(&d("5", "x")).id, "other");
        let me = Driver { user_id: 42, name: "Real".into(), ..Default::default() };
        assert_eq!(r.display(&me).0, "Ace");
        assert_eq!(Resolver::default().class_of(&d("1", "x")).id, "ir7");
    }
}
