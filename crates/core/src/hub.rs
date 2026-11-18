//! Fan-out of topic payloads to WebSocket clients and of UI requests to the
//! desktop host. Topics are only built while someone subscribes to them.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Requests the desktop host acts on (moving windows etc.).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum HostEvent {
    Drag {
        id: String,
    },
    Resize {
        id: String,
        dir: String,
    },
    EditMode {
        value: bool,
    },
    Visible {
        value: bool,
    },
    OpenSettings,
    /// Show a folder in the file manager (the log files).
    OpenFolder {
        path: std::path::PathBuf,
    },
    /// Open a web page in the browser (the app's own repository only, see market::may_open).
    OpenUrl {
        url: String,
    },
    /// Fit an overlay window to its content (CSS pixels; 0 = not automatic).
    AutoSize {
        id: String,
        width: f64,
        height: f64,
    },
    CheckUpdate,
    InstallUpdate,
    /// Close the update pop-up window.
    ClosePrompt,
    /// Answer from the "close Apex Overlays?" pop-up: quit everything, or keep the overlays
    /// running and just hide the settings window.
    CloseApp {
        quit: bool,
    },
    /// You have just driven out of the pit lane (on pit road, then off it, in the car): the
    /// desktop app can close the settings window to the tray.
    LeftPits,
    /// An overlay wants mouse clicks (true) or to be click-through again (false).
    Interactive {
        id: String,
        value: bool,
    },
    /// The overlays started (true) or stopped (false) showing in the headset: the windows move
    /// to their VR positions, or back to their screen positions.
    VrActive {
        value: bool,
    },
}

/// Commands for the telemetry thread.
#[derive(Debug, Clone)]
pub enum Command {
    ResetTrackMap,
    /// Jump the replay to `session_time` (seconds) in session `session_num`, watching the player.
    Replay {
        session_num: i32,
        session_time: f64,
    },
    SetTrackData {
        track_id: i64,
        data: crate::corners::TrackData,
        source: &'static str,
    },
    /// Outline from iRacing's official track map.
    SetOfficialMap(crate::engine::trackmap::TrackMap),
    /// Outline from the shared map library (used until the player records their own).
    SetLibraryMap(crate::engine::trackmap::TrackMap),
    /// Lovely Car Data per `CarPath` (lower case).
    SetCarData(std::collections::HashMap<String, crate::cardata::CarData>),
    /// Lovely Car Classes: iRacing class name → shorthand.
    SetClassNames(std::collections::HashMap<String, String>),
    /// Driver countries looked up from the iRacing Data API (customer ID → ISO code).
    SetCountries(std::collections::BTreeMap<i64, String>),
    /// Le Mans Ultimate's weather forecast (from its local REST API), or none.
    SetLmuForecast(Option<crate::source::lmu::extras::Forecast>),
}

pub struct Hub {
    subs: Mutex<HashMap<String, usize>>,
    latest: Mutex<HashMap<String, Arc<str>>>,
    tx: broadcast::Sender<(Arc<str>, Arc<str>)>,
    host_tx: broadcast::Sender<HostEvent>,
    cmd_tx: Mutex<Option<std::sync::mpsc::Sender<Command>>>,
    pub edit_mode: AtomicBool,
    pub visible: AtomicBool,
    pub connected: AtomicBool,
    /// In the car and out of the pit lane (driving on track) right now.
    pub on_track: AtomicBool,
    pub clients: AtomicUsize,
    pub source: Mutex<&'static str>,
    /// Set by the desktop app, which can install updates; the headless server cannot.
    pub updates_supported: AtomicBool,
    /// Latest raw session info string from the sim (downloadable for troubleshooting).
    pub session_yaml: Mutex<String>,
    /// Running totals for the Monitoring section (see `monitor`).
    pub stats: Stats,
}

/// Counters the monitor turns into rates.
#[derive(Default)]
pub struct Stats {
    /// Bytes and messages sent to overlay windows and the settings page.
    pub bytes_sent: std::sync::atomic::AtomicU64,
    pub messages_sent: std::sync::atomic::AtomicU64,
    /// Telemetry ticks processed, and the time spent on them (nanoseconds).
    pub ticks: std::sync::atomic::AtomicU64,
    pub tick_ns: std::sync::atomic::AtomicU64,
}

impl Default for Hub {
    fn default() -> Self {
        Self {
            subs: Default::default(),
            latest: Default::default(),
            tx: broadcast::channel(512).0,
            host_tx: broadcast::channel(64).0,
            cmd_tx: Mutex::new(None),
            stats: Stats::default(),
            edit_mode: AtomicBool::new(false),
            visible: AtomicBool::new(true),
            connected: AtomicBool::new(false),
            on_track: AtomicBool::new(false),
            clients: AtomicUsize::new(0),
            source: Mutex::new("none"),
            updates_supported: AtomicBool::new(false),
            session_yaml: Mutex::new(String::new()),
        }
    }
}

impl Hub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn has_subscribers(&self, topic: &str) -> bool {
        self.subs.lock().unwrap().get(topic).is_some_and(|&n| n > 0)
    }

    pub fn subscribe_topic(&self, topic: &str) {
        *self.subs.lock().unwrap().entry(topic.to_string()).or_default() += 1;
    }

    pub fn unsubscribe_topic(&self, topic: &str) {
        if let Some(n) = self.subs.lock().unwrap().get_mut(topic) {
            *n = n.saturating_sub(1);
        }
    }

    /// Publish a JSON payload. Returns false (and sends nothing) if it is unchanged.
    pub fn publish(&self, topic: &str, json: &str) -> bool {
        let msg: Arc<str> = crate::topics::Topic::envelope(topic, json).into();
        {
            let mut latest = self.latest.lock().unwrap();
            if latest.get(topic).is_some_and(|m| **m == *msg) {
                return false;
            }
            latest.insert(topic.to_string(), msg.clone());
        }
        let _ = self.tx.send((topic.into(), msg));
        true
    }

    pub fn latest(&self, topic: &str) -> Option<Arc<str>> {
        self.latest.lock().unwrap().get(topic).cloned()
    }

    pub fn receiver(&self) -> broadcast::Receiver<(Arc<str>, Arc<str>)> {
        self.tx.subscribe()
    }

    pub fn host_events(&self) -> broadcast::Receiver<HostEvent> {
        self.host_tx.subscribe()
    }

    pub fn host(&self, e: HostEvent) {
        let _ = self.host_tx.send(e);
    }

    pub fn set_command_sender(&self, tx: std::sync::mpsc::Sender<Command>) {
        *self.cmd_tx.lock().unwrap() = Some(tx);
    }

    pub fn command(&self, c: Command) {
        if let Some(tx) = self.cmd_tx.lock().unwrap().as_ref() {
            let _ = tx.send(c);
        }
    }

    pub fn set_edit_mode(&self, v: bool) {
        if self.edit_mode.swap(v, Ordering::Relaxed) != v {
            log::info!("overlays: edit mode {}", if v { "on" } else { "off" });
        }
        self.host(HostEvent::EditMode { value: v });
        self.publish_status();
    }

    pub fn set_visible(&self, v: bool) {
        if self.visible.swap(v, Ordering::Relaxed) != v {
            log::info!("overlays: all {}", if v { "shown" } else { "hidden (show / hide overlays)" });
        }
        self.host(HostEvent::Visible { value: v });
        self.publish_status();
    }

    pub fn publish_status(&self) {
        let s = serde_json::json!({
            "connected": self.connected.load(Ordering::Relaxed),
            "source": *self.source.lock().unwrap(),
            "editMode": self.edit_mode.load(Ordering::Relaxed),
            "visible": self.visible.load(Ordering::Relaxed),
            "version": env!("CARGO_PKG_VERSION"),
        });
        self.publish("status", &s.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupes_and_counts() {
        let h = Hub::default();
        let mut rx = h.receiver();
        assert!(h.publish("session", "{\"a\":1}"));
        assert!(!h.publish("session", "{\"a\":1}"));
        assert!(h.publish("session", "{\"a\":2}"));
        assert_eq!(&*rx.try_recv().unwrap().1, "{\"t\":\"session\",\"d\":{\"a\":1}}");
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
        assert!(!h.has_subscribers("relative"));
        h.subscribe_topic("relative");
        assert!(h.has_subscribers("relative"));
        h.unsubscribe_topic("relative");
        assert!(!h.has_subscribers("relative"));
    }
}
