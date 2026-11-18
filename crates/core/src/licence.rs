//! Apex Labs licence key: optional during early access.
//!
//! People get a free key at <https://shamzout2.github.io/Apex-Labs/key/> (Discord sign-in) and
//! paste it under General → Licence. The app sends it to the Apex Labs key service, which answers
//! with a licence signed with Ed25519. The licence is kept in `licence.json` in the data folder and
//! checked here against [`PUBLIC_KEY`], so it works offline until it expires (30 days after the last
//! check) and can't be forged by editing the file. It is checked again at start and once a day.
//!
//! Nothing is locked: without a key, or offline, everything works as before.
//!
//! The PC id is a random id shared with Apex Commentator (`%APPDATA%\Apex Labs\pc-id`), so one PC
//! running both apps counts once against the key's limit of 5 PCs.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SERVICE: &str = "https://apex-keys.shamzout.workers.dev";
pub const PUBLIC_KEY: &str = "0oLW-0hzSRd5aBYMf78Bgdb1yJ_xq91Qnrf-6n2P9Lk";
pub const GET_KEY_URL: &str = "https://shamzout2.github.io/Apex-Labs/key/";
const APP: &str = "overlays";
const CHECK_EVERY: Duration = Duration::from_secs(24 * 3600);
const OFFLINE: &str = "Couldn't reach the Apex Labs key service. Your licence keeps working offline; it will check again later.";

/// What `licence.json` holds.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Saved {
    key: String,
    token: String,
    checked_at: String,
    error: String,
    manage_url: String,
    /// Only when the shared PC id file can't be written.
    pc_id: String,
}

/// The signed licence, as the key service writes it.
#[derive(Debug, Default, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Claims {
    pub key: String,
    pub user: String,
    pub tier: String,
    pub tier_name: String,
    pub features: Vec<String>,
    pub pc_id: String,
    pub app: String,
    pub issued_at: String,
    pub expires_at: String,
}

/// What the settings page shows.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub has_key: bool,
    pub key: String,
    pub valid: bool,
    pub user: String,
    pub tier: String,
    pub tier_name: String,
    pub features: Vec<String>,
    pub expires_at: String,
    pub checked_at: String,
    pub error: String,
    pub manage_url: String,
    pub get_key_url: String,
}

/// `APEX-XXXX-XXXX-XXXX-XXXX` from whatever was pasted (any case, spaces, O/I/L for 0/1/1).
pub fn normalise_key(text: &str) -> Option<String> {
    let raw: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .collect();
    let body = raw.strip_prefix("APEX")?;
    let ok = body.len() == 16 && body.chars().all(|c| c.is_ascii_digit() || (c.is_ascii_uppercase() && !matches!(c, 'I' | 'L' | 'O' | 'U')));
    ok.then(|| format!("APEX-{}-{}-{}-{}", &body[0..4], &body[4..8], &body[8..12], &body[12..16]))
}

fn public_key_bytes(b64: &str) -> Option<[u8; 32]> {
    URL_SAFE_NO_PAD.decode(b64).ok()?.try_into().ok()
}

/// The licence inside a signed token, or `None` if the signature is wrong.
pub fn read_token(token: &str, public_key: &[u8; 32]) -> Option<Claims> {
    let (body, sig) = token.split_once('.')?;
    let sig: [u8; 64] = URL_SAFE_NO_PAD.decode(sig).ok()?.try_into().ok()?;
    let key = ed25519_dalek::VerifyingKey::from_bytes(public_key).ok()?;
    key.verify_strict(body.as_bytes(), &ed25519_dalek::Signature::from_bytes(&sig)).ok()?;
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(body).ok()?).ok()
}

/// Seconds since 1970 for `YYYY-MM-DDTHH:MM:SS…` (UTC), or 0.
fn parse_time(s: &str) -> i64 {
    let n = |r: std::ops::Range<usize>| s.get(r).and_then(|t| t.parse::<i64>().ok());
    let (Some(y), Some(m), Some(d), Some(hh), Some(mm), Some(ss)) = (n(0..4), n(5..7), n(8..10), n(11..13), n(14..16), n(17..19)) else { return 0 };
    // Days from civil (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) * 86400 + hh * 3600 + mm * 60 + ss
}

fn iso(t: i64) -> String {
    let (days, secs) = (t.div_euclid(86400), t.rem_euclid(86400));
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", secs / 3600, secs % 3600 / 60, secs % 60)
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// A failed call to the key service.
#[derive(Debug, PartialEq)]
pub enum CallError {
    /// The service answered with a message for the user (unknown key, turned off, too many PCs…).
    Refused(String),
    /// No answer (offline, DNS, timeout).
    Offline,
}

/// How the licence talks to the key service (replaced in tests).
pub trait Transport: Send + Sync {
    fn post(&self, url: &str, body: &serde_json::Value) -> Result<serde_json::Value, CallError>;
}

struct Http;

impl Transport for Http {
    fn post(&self, url: &str, body: &serde_json::Value) -> Result<serde_json::Value, CallError> {
        match crate::net::agent().post(url).send_json(body.clone()) {
            Ok(r) => r.into_json().map_err(|_| CallError::Offline),
            Err(ureq::Error::Status(code, r)) => {
                let msg = r.into_json::<serde_json::Value>().ok().and_then(|v| v["message"].as_str().map(String::from));
                Err(CallError::Refused(msg.unwrap_or_else(|| format!("The key service answered {code}."))))
            }
            Err(_) => Err(CallError::Offline),
        }
    }
}

fn default_pc_id_file() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Apex Labs").join("pc-id")
}

fn valid_pc_id(s: &str) -> bool {
    (16..=64).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A random id for this PC (not a secret, only needs to be unique).
fn new_pc_id() -> String {
    let mut h = Sha256::new();
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    h.update(t.as_nanos().to_le_bytes());
    h.update(std::process::id().to_le_bytes());
    h.update((&h as *const _ as usize).to_le_bytes());
    h.update(format!("{:?}", std::thread::current().id()).as_bytes());
    URL_SAFE_NO_PAD.encode(&h.finalize()[..18])
}

pub struct Licence {
    path: Option<PathBuf>,
    pc_id_file: PathBuf,
    service: String,
    public_key: [u8; 32],
    transport: Box<dyn Transport>,
    clock: Box<dyn Fn() -> i64 + Send + Sync>,
    saved: Mutex<Saved>,
    pc_id: Mutex<Option<String>>,
}

impl Licence {
    /// The app's licence, kept in `data_dir/licence.json`.
    pub fn new(data_dir: &Path) -> Self {
        let mut l = Self::with(Some(data_dir.join("licence.json")), default_pc_id_file(), Box::new(Http), PUBLIC_KEY, Box::new(now));
        // Development builds can talk to another copy of the key service (e.g. a local test one).
        #[cfg(debug_assertions)]
        if let Ok(url) = std::env::var("APEX_KEYS_SERVICE") {
            l.service = url.trim_end_matches('/').to_string();
        }
        l
    }

    pub fn with(
        path: Option<PathBuf>,
        pc_id_file: PathBuf,
        transport: Box<dyn Transport>,
        public_key: &str,
        clock: Box<dyn Fn() -> i64 + Send + Sync>,
    ) -> Self {
        let saved = path.as_ref().and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self {
            path,
            pc_id_file,
            service: SERVICE.to_string(),
            public_key: public_key_bytes(public_key).expect("licence public key"),
            transport,
            clock,
            saved: Mutex::new(saved),
            pc_id: Mutex::new(None),
        }
    }

    fn pc_id(&self) -> String {
        let mut cached = self.pc_id.lock().unwrap();
        if let Some(id) = cached.as_ref() {
            return id.clone();
        }
        let from_file = std::fs::read_to_string(&self.pc_id_file).ok().map(|s| s.trim().to_string()).filter(|s| valid_pc_id(s));
        let id = from_file.unwrap_or_else(|| {
            let mut saved = self.saved.lock().unwrap();
            let id = if valid_pc_id(&saved.pc_id) { saved.pc_id.clone() } else { new_pc_id() };
            let shared = self.pc_id_file.parent().map(std::fs::create_dir_all).transpose().is_ok() && std::fs::write(&self.pc_id_file, &id).is_ok();
            if !shared {
                saved.pc_id = id.clone(); // can't share it: keep our own
                self.write(&saved);
            }
            id
        });
        *cached = Some(id.clone());
        id
    }

    fn write(&self, saved: &Saved) {
        if let Some(path) = &self.path {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(path, serde_json::to_vec_pretty(saved).unwrap_or_default()) {
                log::warn!("licence: could not save {}: {e}", path.display());
            }
        }
    }

    fn update(&self, f: impl FnOnce(&mut Saved)) {
        let mut saved = self.saved.lock().unwrap();
        f(&mut saved);
        self.write(&saved);
    }

    /// The verified licence for this key and PC, or `None` (no key, expired, tampered).
    pub fn current(&self) -> Option<Claims> {
        let (key, token) = {
            let s = self.saved.lock().unwrap();
            (s.key.clone(), s.token.clone())
        };
        if key.is_empty() || token.is_empty() {
            return None;
        }
        let claims = read_token(&token, &self.public_key)?;
        let ok = claims.key == key && claims.pc_id == self.pc_id() && claims.app == APP && parse_time(&claims.expires_at) > (self.clock)();
        ok.then_some(claims)
    }

    /// Whether the licence unlocks `feature` ("all" unlocks everything).
    pub fn has_feature(&self, feature: &str) -> bool {
        self.current().is_some_and(|c| c.features.iter().any(|f| f == "all" || f == feature))
    }

    pub fn status(&self) -> Status {
        let claims = self.current();
        let s = self.saved.lock().unwrap().clone();
        let c = claims.clone().unwrap_or_default();
        Status {
            has_key: !s.key.is_empty(),
            key: s.key,
            valid: claims.is_some(),
            user: c.user,
            tier: c.tier,
            tier_name: c.tier_name,
            features: c.features,
            expires_at: c.expires_at,
            checked_at: s.checked_at,
            error: s.error,
            manage_url: if s.manage_url.is_empty() { format!("{}/", self.service) } else { s.manage_url },
            get_key_url: GET_KEY_URL.to_string(),
        }
    }

    fn activate(&self, key: &str) -> Result<(String, String), CallError> {
        let body = serde_json::json!({ "key": key, "pc_id": self.pc_id(), "app": APP, "version": env!("CARGO_PKG_VERSION") });
        let r = self.transport.post(&format!("{}/v1/activate", self.service), &body)?;
        let token = r["token"].as_str().unwrap_or_default().to_string();
        match read_token(&token, &self.public_key) {
            Some(c) if c.key == key && c.pc_id == self.pc_id() => Ok((token, r["manage_url"].as_str().unwrap_or_default().to_string())),
            _ => Err(CallError::Refused("The key service sent a licence this app couldn't verify. Please update Apex Overlays.".into())),
        }
    }

    /// Add a key (checked with the service first).
    pub fn set_key(&self, text: &str) -> Result<Status, String> {
        let key = normalise_key(text).ok_or("That doesn't look like an Apex Labs key. It looks like APEX-XXXX-XXXX-XXXX-XXXX.")?;
        match self.activate(&key) {
            Ok((token, manage_url)) => {
                let checked = iso((self.clock)());
                self.update(|s| {
                    *s = Saved { key, token, checked_at: checked, error: String::new(), manage_url, pc_id: std::mem::take(&mut s.pc_id) };
                });
                Ok(self.status())
            }
            Err(CallError::Refused(msg)) => Err(msg),
            Err(CallError::Offline) => Err("Couldn't reach the Apex Labs key service. Check your internet connection and try again.".into()),
        }
    }

    /// Forget the key on this PC and free its place (if the service can be reached).
    pub fn remove(&self) -> Status {
        let key = self.saved.lock().unwrap().key.clone();
        if !key.is_empty() {
            let body = serde_json::json!({ "key": key, "pc_id": self.pc_id() });
            let _ = self.transport.post(&format!("{}/v1/deactivate", self.service), &body);
        }
        self.update(|s| *s = Saved { pc_id: std::mem::take(&mut s.pc_id), ..Saved::default() });
        self.status()
    }

    /// Check the key again. Offline keeps the saved licence; a turned-off key loses it.
    pub fn refresh(&self) -> Status {
        let key = self.saved.lock().unwrap().key.clone();
        if key.is_empty() {
            return self.status();
        }
        match self.activate(&key) {
            Ok((token, manage_url)) => {
                let checked = iso((self.clock)());
                self.update(|s| {
                    (s.token, s.manage_url, s.checked_at, s.error) = (token, manage_url, checked, String::new());
                });
            }
            Err(CallError::Offline) => self.update(|s| s.error = OFFLINE.into()),
            Err(CallError::Refused(msg)) => self.update(|s| {
                s.error = msg;
                s.token.clear();
            }),
        }
        self.status()
    }
}

/// Check in the background a few seconds after start and then once a day.
pub fn spawn(licence: Arc<Licence>) {
    let _ = std::thread::Builder::new().name("licence".into()).spawn(move || {
        std::thread::sleep(Duration::from_secs(5));
        loop {
            if licence.status().has_key {
                let st = licence.refresh();
                log::info!("licence: {}", if st.valid { format!("{} ({})", st.tier_name, st.user) } else { format!("not confirmed: {}", st.error) });
            }
            std::thread::sleep(CHECK_EVERY);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::sync::atomic::{AtomicI64, Ordering};

    const KEY: &str = "APEX-7K3Q-M9TD-2XWR-HB4P";
    const NOW: i64 = 1_800_000_000;

    /// A tiny key service.
    struct Fake {
        signer: SigningKey,
        keys: Vec<String>,
        offline: Mutex<bool>,
        revoked: Mutex<bool>,
        calls: Mutex<Vec<(String, serde_json::Value)>>,
    }

    impl Transport for Arc<Fake> {
        fn post(&self, url: &str, body: &serde_json::Value) -> Result<serde_json::Value, CallError> {
            self.calls.lock().unwrap().push((url.rsplit('/').next().unwrap().to_string(), body.clone()));
            if *self.offline.lock().unwrap() {
                return Err(CallError::Offline);
            }
            if url.ends_with("/deactivate") {
                return Ok(serde_json::json!({ "ok": true }));
            }
            let key = body["key"].as_str().unwrap();
            if *self.revoked.lock().unwrap() {
                return Err(CallError::Refused("This key has been turned off.".into()));
            }
            if !self.keys.iter().any(|k| k == key) {
                return Err(CallError::Refused("This key doesn't exist. Check it on the key page.".into()));
            }
            let payload = serde_json::json!({
                "v": 1, "key": key, "user": "shamz2", "tier": "early_access", "tier_name": "Early Access",
                "features": ["all"], "pc_id": body["pc_id"], "app": body["app"],
                "issued_at": iso(NOW), "expires_at": format!("{}.000Z", iso(NOW + 30 * 86400).trim_end_matches('Z')),
            });
            let part = URL_SAFE_NO_PAD.encode(payload.to_string());
            let sig = self.signer.sign(part.as_bytes());
            Ok(serde_json::json!({ "token": format!("{part}.{}", URL_SAFE_NO_PAD.encode(sig.to_bytes())), "manage_url": "https://keys.test/" }))
        }
    }

    struct Setup {
        dir: PathBuf,
        fake: Arc<Fake>,
        clock: Arc<AtomicI64>,
        public: String,
    }

    fn setup(name: &str) -> Setup {
        let dir = std::env::temp_dir().join(format!("apex-licence-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let signer = SigningKey::from_bytes(&[7u8; 32]);
        let public = URL_SAFE_NO_PAD.encode(signer.verifying_key().to_bytes());
        let fake = Arc::new(Fake { signer, keys: vec![KEY.into()], offline: Mutex::new(false), revoked: Mutex::new(false), calls: Mutex::new(vec![]) });
        Setup { dir, fake, clock: Arc::new(AtomicI64::new(NOW)), public }
    }

    fn licence(s: &Setup) -> Licence {
        let clock = s.clock.clone();
        Licence::with(
            Some(s.dir.join("data/licence.json")),
            s.dir.join("Apex Labs/pc-id"),
            Box::new(s.fake.clone()),
            &s.public,
            Box::new(move || clock.load(Ordering::SeqCst)),
        )
    }

    #[test]
    fn keys_are_tidied() {
        assert_eq!(normalise_key(" apex 7k3q m9td 2xwr hb4p ").as_deref(), Some(KEY));
        assert_eq!(normalise_key("APEX-7K3Q-M9TD-2XWR-HB4O").as_deref(), Some("APEX-7K3Q-M9TD-2XWR-HB40"));
        assert_eq!(normalise_key("APEX-UUUU-M9TD-2XWR-HB4P"), None);
        assert_eq!(normalise_key("APEX-7K3Q"), None);
        assert_eq!(normalise_key(""), None);
    }

    #[test]
    fn times() {
        assert_eq!(parse_time("1970-01-01T00:00:00Z"), 0);
        assert_eq!(parse_time("2027-01-15T08:00:00.000Z"), NOW);
        assert_eq!(iso(NOW), "2027-01-15T08:00:00Z");
        assert_eq!(parse_time(&iso(1_234_567_890)), 1_234_567_890);
        assert_eq!(parse_time("garbage"), 0);
    }

    #[test]
    fn signed_by_the_live_service() {
        // A test key's licence, signed by the real Apex Labs key service on 18 Nov 2026.
        let token = include_str!("../testdata/live-licence.txt").trim();
        let claims = read_token(token, &public_key_bytes(PUBLIC_KEY).unwrap()).expect("the app's public key must match the key service");
        assert_eq!(claims.key, "APEX-TEST-0000-0000-0001");
        assert_eq!(claims.features, ["all"]);
        let (body, sig) = token.split_once('.').unwrap();
        assert!(read_token(&format!("{}AA.{sig}", &body[..body.len() - 2]), &public_key_bytes(PUBLIC_KEY).unwrap()).is_none());
    }

    #[test]
    fn no_key_is_fine() {
        let s = setup("none");
        let st = licence(&s).status();
        assert!(!st.has_key && !st.valid);
        assert_eq!(st.get_key_url, GET_KEY_URL);
    }

    #[test]
    fn add_key_check_and_keep_offline() {
        let s = setup("add");
        let l = licence(&s);
        let st = l.set_key("apex 7k3q m9td 2xwr hb4p").unwrap();
        assert!(st.valid);
        assert_eq!((st.key.as_str(), st.user.as_str(), st.tier_name.as_str()), (KEY, "shamz2", "Early Access"));
        assert!(l.has_feature("anything"));
        let (_, sent) = s.fake.calls.lock().unwrap()[0].clone();
        assert_eq!(sent["app"], "overlays");
        assert_eq!(sent["pc_id"].as_str().unwrap(), std::fs::read_to_string(s.dir.join("Apex Labs/pc-id")).unwrap());
        // A new start reads it back and trusts it, offline.
        *s.fake.offline.lock().unwrap() = true;
        let again = licence(&s);
        assert!(again.status().valid);
        let st = again.refresh();
        assert!(st.valid && st.error.contains("offline"));
        *s.fake.offline.lock().unwrap() = false;
        assert!(again.refresh().error.is_empty());
    }

    #[test]
    fn shares_the_pc_id_with_commentator() {
        let s = setup("shared");
        std::fs::create_dir_all(s.dir.join("Apex Labs")).unwrap();
        std::fs::write(s.dir.join("Apex Labs/pc-id"), "made-by-commentator-0123").unwrap();
        licence(&s).set_key(KEY).unwrap();
        assert_eq!(s.fake.calls.lock().unwrap()[0].1["pc_id"], "made-by-commentator-0123");
    }

    #[test]
    fn bad_unknown_and_offline_keys() {
        let s = setup("bad");
        let l = licence(&s);
        assert!(l.set_key("hello").unwrap_err().contains("doesn't look like"));
        assert!(l.set_key("APEX-0000-0000-0000-0000").unwrap_err().contains("doesn't exist"));
        *s.fake.offline.lock().unwrap() = true;
        assert!(l.set_key(KEY).unwrap_err().contains("internet"));
        assert!(!l.status().has_key);
    }

    #[test]
    fn licences_signed_by_anyone_else_are_refused() {
        let s = setup("foreign");
        let clock = s.clock.clone();
        let other = URL_SAFE_NO_PAD.encode(SigningKey::from_bytes(&[9u8; 32]).verifying_key().to_bytes());
        let l = Licence::with(
            Some(s.dir.join("data/licence.json")),
            s.dir.join("Apex Labs/pc-id"),
            Box::new(s.fake.clone()),
            &other,
            Box::new(move || clock.load(Ordering::SeqCst)),
        );
        assert!(l.set_key(KEY).unwrap_err().contains("couldn't verify"));
    }

    #[test]
    fn editing_the_saved_file_breaks_it() {
        let s = setup("edit");
        licence(&s).set_key(KEY).unwrap();
        let path = s.dir.join("data/licence.json");
        let mut saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let token = saved["token"].as_str().unwrap().to_string();
        let (body, sig) = token.split_once('.').unwrap();
        let mut claims: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(body).unwrap()).unwrap();
        claims["features"] = serde_json::json!(["everything"]);
        saved["token"] = format!("{}.{sig}", URL_SAFE_NO_PAD.encode(claims.to_string())).into();
        std::fs::write(&path, saved.to_string()).unwrap();
        assert!(!licence(&s).status().valid);
    }

    #[test]
    fn expires_after_30_days_without_a_check() {
        let s = setup("expiry");
        let l = licence(&s);
        l.set_key(KEY).unwrap();
        s.clock.store(NOW + 29 * 86400, Ordering::SeqCst);
        assert!(l.status().valid);
        s.clock.store(NOW + 31 * 86400, Ordering::SeqCst);
        let st = l.status();
        assert!(!st.valid && st.has_key);
    }

    #[test]
    fn a_turned_off_key_loses_its_licence() {
        let s = setup("revoked");
        let l = licence(&s);
        l.set_key(KEY).unwrap();
        *s.fake.revoked.lock().unwrap() = true;
        let st = l.refresh();
        assert!(!st.valid && st.has_key && st.error.contains("turned off"));
    }

    /// One HTTP answer from a local server, for the real `Http` transport.
    fn serve_once(status: &str, body: &str) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let reply = format!("HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
        std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = sock.read(&mut buf);
            let _ = sock.write_all(reply.as_bytes());
        });
        format!("http://{addr}/v1/activate")
    }

    #[test]
    fn http_transport_reads_answers_and_messages() {
        let ok = serve_once("200 OK", r#"{"token":"a.b"}"#);
        assert_eq!(Http.post(&ok, &serde_json::json!({})).unwrap()["token"], "a.b");
        let refused = serve_once("409 Conflict", r#"{"error":"pc_limit","message":"This key is already used on 5 PCs."}"#);
        assert_eq!(Http.post(&refused, &serde_json::json!({})), Err(CallError::Refused("This key is already used on 5 PCs.".into())));
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        assert_eq!(Http.post(&format!("http://{closed}/v1/activate"), &serde_json::json!({})), Err(CallError::Offline));
    }

    #[test]
    fn remove_frees_the_pc_even_offline() {
        let s = setup("remove");
        let l = licence(&s);
        l.set_key(KEY).unwrap();
        assert!(!l.remove().has_key);
        assert_eq!(s.fake.calls.lock().unwrap().last().unwrap().0, "deactivate");
        l.set_key(KEY).unwrap();
        *s.fake.offline.lock().unwrap() = true;
        assert!(!l.remove().has_key);
        assert!(!licence(&s).status().has_key, "saved");
    }
}
