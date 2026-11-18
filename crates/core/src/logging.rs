//! The app's log file: `<data dir>/logs/apex.log`, with the previous run kept as
//! `apex.previous.log` (so the log of a run that crashed survives the restart).
//!
//! Errors and warnings from everywhere are written, plus what the app is doing (game found,
//! session, overlays opened, updates). Crashes (panics) are written with where they happened.
//! The same line repeated is written once with a count, so a failure that repeats every tick
//! cannot fill the disk. Overlay pages report their script errors here too (see server.rs).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Past this size the file starts again (the full one kept as apex.part.log).
const MAX_BYTES: u64 = 8 * 1024 * 1024;

pub const FILE_NAME: &str = "apex.log";
pub const PREVIOUS_NAME: &str = "apex.previous.log";

/// The folder the log files are in.
pub fn dir(data_dir: &Path) -> PathBuf {
    data_dir.join("logs")
}

struct Out {
    file: Option<File>,
    path: PathBuf,
    written: u64,
    /// The last line (without its time) and how many times it came again since.
    last: String,
    repeats: u32,
}

struct Logger {
    out: Mutex<Out>,
    echo: bool,
}

static LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

/// Starts logging to the data folder (once; later calls do nothing). `echo` also prints each
/// line to stderr (the headless server). Returns the log file's path.
pub fn init(data_dir: &Path, echo: bool) -> Option<PathBuf> {
    let dir = dir(data_dir);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(FILE_NAME);
    if LOGGER.get().is_some() {
        return Some(path);
    }
    // The last run's log is kept beside this one.
    if path.exists() {
        let _ = std::fs::rename(&path, dir.join(PREVIOUS_NAME));
    }
    let file = OpenOptions::new().create(true).append(true).open(&path).ok();
    let logger = LOGGER.get_or_init(|| Logger { out: Mutex::new(Out { file, path: path.clone(), written: 0, last: String::new(), repeats: 0 }), echo });
    if log::set_logger(logger).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
    install_panic_hook();
    log::info!(
        target: "apex_overlays_desktop",
        "Apex Overlays {} on {} ({} cores, {} MB memory); data in {}",
        env!("CARGO_PKG_VERSION"),
        sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into()),
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
        {
            let mut s = sysinfo::System::new();
            s.refresh_memory();
            s.total_memory() / 1_048_576
        },
        data_dir.display()
    );
    Some(path)
}

impl log::Log for Logger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        // Ours: from information up. Libraries (the web server, TLS, the window toolkit...):
        // only their warnings and errors.
        let ours = m.target().starts_with("apex") || m.target() == "ui";
        m.level() <= if ours { log::Level::Info } else { log::Level::Warn }
    }

    fn log(&self, r: &log::Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        // Short names: "core", "app", "garage61", "source::lmu"...
        let target = match r.target() {
            "apex_core" => "core",
            "apex_overlays_desktop" | "apex_overlays_server" => "app",
            t => t.strip_prefix("apex_core::").or_else(|| t.strip_prefix("apex_overlays_desktop::")).unwrap_or(t),
        };
        let body = format!("{:<5} {target}: {}", r.level(), r.args());
        self.write(body);
    }

    fn flush(&self) {
        if let Ok(mut o) = self.out.lock() {
            if let Some(f) = o.file.as_mut() {
                let _ = f.flush();
            }
        }
    }
}

impl Logger {
    fn write(&self, body: String) {
        let Ok(mut o) = self.out.lock() else { return };
        if body == o.last {
            o.repeats += 1;
            return;
        }
        let mut text = String::new();
        if o.repeats > 0 {
            text += &format!("{} (the line above came {} more time{})\n", timestamp(), o.repeats, if o.repeats == 1 { "" } else { "s" });
        }
        text += &format!("{} {body}\n", timestamp());
        if self.echo {
            eprint!("{text}");
        }
        o.last = body;
        o.repeats = 0;
        o.written += text.len() as u64;
        if o.written > MAX_BYTES {
            // Full: keep it as apex.part.log and carry on in a fresh file.
            o.file = None;
            let part = o.path.with_file_name("apex.part.log");
            let _ = std::fs::rename(&o.path, part);
            o.file = OpenOptions::new().create(true).append(true).open(&o.path).ok();
            o.written = text.len() as u64;
        }
        if let Some(f) = o.file.as_mut() {
            let _ = f.write_all(text.as_bytes());
            let _ = f.flush();
        }
    }
}

/// Crashes are written to the log (with the thread, the place and a backtrace) before the
/// usual report.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".into());
        // The same on every system: Windows gives the file as crates\core\src\logging.rs.
        let place = info.location().map(|l| format!("{}:{}", l.file().replace('\\', "/"), l.line())).unwrap_or_default();
        let bt = std::backtrace::Backtrace::force_capture();
        log::error!(target: "apex::crash", "thread '{thread}' crashed at {place}: {msg}\n{bt}");
        previous(info);
    }));
}

/// "2026-11-16 09:31:31.123Z" (UTC).
fn timestamp() -> String {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, dd) = civil(days);
    format!("{y:04}-{m:02}-{dd:02} {:02}:{:02}:{:02}.{:03}Z", rem / 3600, rem % 3600 / 60, rem % 60, d.subsec_millis())
}

/// Days since 1970-01-01 to (year, month, day) (Howard Hinnant's civil_from_days).
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + (m <= 2) as i64, m, d)
}

/// The end of the log file (at most `max` bytes, from a line start), for showing in Settings.
pub fn tail(data_dir: &Path, max: usize) -> String {
    let Ok(text) = std::fs::read(dir(data_dir).join(FILE_NAME)) else { return String::new() };
    let start = text.len().saturating_sub(max);
    let s = String::from_utf8_lossy(&text[start..]).into_owned();
    if start > 0 {
        s.split_once('\n').map(|(_, rest)| rest.to_string()).unwrap_or(s)
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_773), (2026, 11, 16));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert_eq!(timestamp().len(), 24);
    }

    #[test]
    fn writes_errors_keeps_the_last_run_and_folds_repeats() {
        let dir = std::env::temp_dir().join(format!("apex-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(super::dir(&dir)).unwrap();
        std::fs::write(super::dir(&dir).join(FILE_NAME), "last run\n").unwrap();
        let path = init(&dir, false).unwrap();
        assert_eq!(std::fs::read_to_string(super::dir(&dir).join(PREVIOUS_NAME)).unwrap(), "last run\n", "previous run kept");
        log::error!("garage61: fetch failed: timed out");
        for _ in 0..500 {
            log::warn!(target: "ui", "relative: TypeError: x is undefined");
        }
        log::info!("game: lmu");
        // Libraries' chatter is left out; their warnings are not.
        log::info!(target: "hyper::proto", "noise");
        log::warn!(target: "tao::platform", "window problem");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Apex Overlays "), "header: {text}");
        assert!(text.contains("ERROR logging::tests: garage61: fetch failed: timed out"), "{text}");
        assert_eq!(text.matches("TypeError").count(), 1, "repeats folded: {text}");
        assert!(text.contains("came 499 more times"), "{text}");
        assert!(!text.contains("noise") && text.contains("window problem"), "{text}");
        assert!(tail(&dir, 300).lines().count() >= 2, "the end of the log, from a line start");
        // A crash in any thread is written with where it happened.
        let _ = std::thread::Builder::new().name("telemetry".into()).spawn(|| panic!("index out of range")).unwrap().join();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("ERROR apex::crash: thread 'telemetry' crashed at crates/core/src/logging.rs:"), "{text}");
        assert!(text.contains("index out of range"), "{text}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
