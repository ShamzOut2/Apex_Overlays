//! Headless server. Every overlay is available at `http://127.0.0.1:<port>/o/<id>`,
//! which can be added to OBS as a browser source; settings live at `/`.

use std::path::PathBuf;

const HELP: &str = "apex-overlays-server [--mock] [--port N] [--data-dir DIR] [--ui-dir DIR]

  --mock          run the built-in race simulator instead of reading iRacing
  --port N        override the configured port (default 8182)
  --data-dir DIR  where config.json and recorded track maps are stored
  --ui-dir DIR    serve UI files from DIR first (development)";

fn main() {
    let mut opts = apex_core::Options { data_dir: apex_core::config::default_data_dir(), mock: false, ui_dir: None, port: None };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--mock" => opts.mock = true,
            "--port" => opts.port = args.next().and_then(|v| v.parse().ok()),
            "--data-dir" => opts.data_dir = args.next().map(PathBuf::from).unwrap_or(opts.data_dir),
            "--ui-dir" => opts.ui_dir = args.next().map(PathBuf::from),
            "-h" | "--help" => {
                println!("{HELP}");
                return;
            }
            other => {
                eprintln!("unknown argument {other}\n\n{HELP}");
                std::process::exit(2);
            }
        }
    }
    apex_core::logging::init(&opts.data_dir, true);
    let app = match apex_core::start(opts) {
        Ok(a) => a,
        Err(e) => {
            log::error!("failed to start: {e}");
            std::process::exit(1);
        }
    };
    log::info!("Apex Overlays {} — settings: {}", env!("CARGO_PKG_VERSION"), app.url());
    log::info!("overlay URLs (OBS browser sources): {}", app.overlay_url("<id>"));
    log::info!("data directory: {}", app.data_dir.display());
    loop {
        std::thread::park();
    }
}
