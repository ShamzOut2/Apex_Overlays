//! Layout Market: overlay layouts people share, kept on the `market` branch of the app's
//! GitHub repository. A GitHub Action (.github/workflows/market.yml) checks each one shared
//! through the repository's "Share a layout" form and publishes it there: `index.json` lists
//! them, `layouts/<id>.json` holds each. Read here so the settings window gets them from the
//! app itself (cached for a few minutes).

use std::io::Read as _;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const REPO_URL: &str = "https://github.com/ShamzOut2/Apex_Overlays";
const RAW: &str = "https://raw.githubusercontent.com/ShamzOut2/Apex_Overlays/market";

/// Where the market is read from: the repository, or a copy for testing (APEX_MARKET_URL).
fn raw() -> String {
    std::env::var("APEX_MARKET_URL").unwrap_or_else(|_| RAW.to_string())
}
const CACHE: Duration = Duration::from_secs(120);
/// A layout bigger than this is not one the Action would have published.
const MAX_BYTES: u64 = 512 * 1024;

static INDEX: Mutex<Option<(Instant, String)>> = Mutex::new(None);

/// A layout id as the Action writes them (the issue number it was shared in).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn get(url: &str) -> Result<Option<String>, String> {
    match crate::net::agent().get(url).call() {
        Ok(r) => {
            let mut body = String::new();
            std::io::Read::take(r.into_reader(), MAX_BYTES).read_to_string(&mut body).map_err(|e| e.to_string())?;
            Ok(Some(body))
        }
        // No market branch yet (nothing shared so far), or no such layout.
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// The list of shared layouts (`{"layouts": [...]}`); `refresh` skips the cache.
pub fn index(refresh: bool) -> Result<String, String> {
    if !refresh {
        if let Some((at, body)) = INDEX.lock().unwrap().as_ref() {
            if at.elapsed() < CACHE {
                return Ok(body.clone());
            }
        }
    }
    let body = get(&format!("{}/index.json", raw()))?.unwrap_or_else(|| r#"{"layouts":[]}"#.to_string());
    serde_json::from_str::<serde_json::Value>(&body).map_err(|e| format!("the market list could not be read: {e}"))?;
    *INDEX.lock().unwrap() = Some((Instant::now(), body.clone()));
    Ok(body)
}

/// One shared layout.
pub fn layout(id: &str) -> Result<Option<String>, String> {
    if !valid_id(id) {
        return Ok(None);
    }
    get(&format!("{}/layouts/{id}.json", raw()))
}

/// Web pages the settings window may open: the app's own repository (the share form, a
/// layout's page).
pub fn may_open(url: &str) -> bool {
    url.starts_with(&format!("{REPO_URL}/")) && !url.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '\\' | '<' | '>'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_links() {
        assert!(valid_id("42") && valid_id("abc-1"));
        assert!(!valid_id("") && !valid_id("../x") && !valid_id("a/b"));
        assert!(may_open("https://github.com/ShamzOut2/Apex_Overlays/issues/new?template=share-layout.yml&title=Layout"));
        assert!(!may_open("https://github.com/ShamzOut2/Apex_Overlays"), "the repository's own pages only");
        assert!(!may_open("https://example.com/ShamzOut2/Apex_Overlays/"));
        assert!(!may_open("https://github.com/ShamzOut2/Apex_Overlays/\" & calc"));
        assert!(may_open("https://github.com/ShamzOut2/Apex_Overlays/issues/new?title=Layout%3A%20Race"));
    }
}
