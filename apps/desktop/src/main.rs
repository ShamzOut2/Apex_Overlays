//! Apex Overlays desktop host: runs the core in-process and shows every enabled overlay
//! as its own transparent, always-on-top, click-through window. In edit mode the
//! windows accept the mouse so they can be dragged and resized; their geometry
//! is written back to the config.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod vr;

use apex_core::config::{Config, ConfigStore};
use apex_core::hub::{HostEvent, Hub};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, RunEvent, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const SETTINGS: &str = "settings";
const UPDATE_PROMPT: &str = "update-prompt";
const CLOSE_PROMPT: &str = "close-prompt";
const PREFIX: &str = "ov-";

/// Pending (position, size) of an overlay window, in logical pixels.
type Geometry = (Option<(i32, i32)>, Option<(u32, u32)>);

struct Core {
    hub: Arc<Hub>,
    config: ConfigStore,
    url: String,
    port: u16,
    /// Window geometry changes waiting to be written to the config (debounced).
    pending: Mutex<HashMap<String, Geometry>>,
    /// Overlays that size their window to their content (label -> (width, height), 0 = user-set).
    auto_size: Mutex<HashMap<String, (f64, f64)>>,
    /// Overlays that take mouse clicks right now (e.g. the incident list's replay buttons).
    interactive: Mutex<HashSet<String>>,
    /// "Move all together": the overlay being dragged, its start position and every other
    /// overlay's start position (logical pixels).
    group_drag: Mutex<Option<GroupDrag>>,
    /// Update found by the last check, ready to install.
    update: tokio::sync::Mutex<Option<tauri_plugin_updater::Update>>,
    /// Overlay id -> its window's native handle, for VR capture (Windows).
    hwnds: Mutex<HashMap<String, isize>>,
}

struct GroupDrag {
    leader: String,
    start: (f64, f64),
    others: Vec<(String, f64, f64)>,
}

fn logical_pos(w: &WebviewWindow) -> Option<(f64, f64)> {
    let s = w.scale_factor().unwrap_or(1.0);
    let p: LogicalPosition<f64> = w.outer_position().ok()?.to_logical(s);
    Some((p.x, p.y))
}

fn label_for(id: &str) -> String {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    format!("{PREFIX}{safe}")
}

fn id_for(cfg: &Config, label: &str) -> Option<String> {
    cfg.overlays.iter().find(|o| label_for(&o.id) == label).map(|o| o.id.clone())
}

fn show_settings(app: &AppHandle) {
    let core = app.state::<Core>();
    if let Some(w) = app.get_webview_window(SETTINGS) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let url = core.url.parse().expect("valid url");
    let _ = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::External(url))
        .title("Apex Overlays")
        .inner_size(1240.0, 860.0)
        .min_inner_size(900.0, 600.0)
        // Tauri's file-drop handler swallows drag & drop on Windows; the page needs it to
        // reorder columns.
        .disable_drag_drop_handler()
        .build();
}

/// The small "update available" window (the settings page in pop-up mode).
fn show_update_prompt(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(UPDATE_PROMPT) {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let url = format!("{}?prompt=update", app.state::<Core>().url).parse().expect("valid url");
    let _ = WebviewWindowBuilder::new(app, UPDATE_PROMPT, WebviewUrl::External(url))
        .title("Apex Overlays – update available")
        .inner_size(560.0, 560.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .always_on_top(true)
        .center()
        .focused(true)
        .build();
}

/// "Close the overlays too, or keep them running?" — asked when the settings window closes.
fn show_close_prompt(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(CLOSE_PROMPT) {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let url = format!("{}?prompt=close", app.state::<Core>().url).parse().expect("valid url");
    let _ = WebviewWindowBuilder::new(app, CLOSE_PROMPT, WebviewUrl::External(url))
        .title("Close Apex Overlays")
        .inner_size(460.0, 250.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .always_on_top(true)
        .center()
        .focused(true)
        .build();
}

fn quit_all(app: &AppHandle) {
    flush_geometry(app);
    app.exit(0);
}

/// Make the set of overlay windows match the config.
fn reconcile(app: &AppHandle) {
    let core = app.state::<Core>();
    let cfg = core.config.get();
    let edit = core.hub.edit_mode.load(Ordering::Relaxed);
    let visible = core.hub.visible.load(Ordering::Relaxed);
    let mut wanted = Vec::new();
    let mut hwnds = HashMap::new();
    for o in cfg.overlays.iter().filter(|o| o.enabled) {
        let label = label_for(&o.id);
        wanted.push(label.clone());
        let win = match app.get_webview_window(&label) {
            Some(w) => {
                // A position typed in settings ends a "move all together" drag of this overlay.
                let mut group = core.group_drag.lock().unwrap();
                if group.as_ref().is_some_and(|g| g.leader == label)
                    && logical_pos(&w).is_some_and(|(x, y)| (x - o.x as f64).abs() > 1.0 || (y - o.y as f64).abs() > 1.0)
                {
                    *group = None;
                }
                drop(group);
                // Auto-sized overlays keep the size their content asked for.
                let (aw, ah) = core.auto_size.lock().unwrap().get(&label).copied().unwrap_or_default();
                let pick = |auto: f64, set: u32| if auto > 0.0 { auto.round() as u32 } else { set };
                sync_geometry(&w, o.x, o.y, pick(aw, o.width), pick(ah, o.height));
                w
            }
            None => {
                let url = format!("http://127.0.0.1:{}/o/{}", core.port, o.id).parse().expect("valid url");
                match WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
                    .title(if o.name.is_empty() { &o.kind } else { &o.name })
                    .transparent(true)
                    .decorations(false)
                    .shadow(false)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .resizable(true)
                    .focused(false)
                    .visible(visible || edit)
                    .position(o.x as f64, o.y as f64)
                    .inner_size(o.width.max(40) as f64, o.height.max(20) as f64)
                    .build()
                {
                    Ok(w) => w,
                    Err(e) => {
                        log::error!("could not open overlay {}: {e}", o.id);
                        continue;
                    }
                }
            }
        };
        #[cfg(windows)]
        if let Ok(h) = win.hwnd() {
            hwnds.insert(o.id.clone(), h.0 as isize);
        }
        let clickable = edit || core.interactive.lock().unwrap().contains(&label);
        let _ = win.set_ignore_cursor_events(!clickable);
        let _ = if visible || edit { win.show() } else { win.hide() };
    }
    for (label, w) in app.webview_windows() {
        if label.starts_with(PREFIX) && !wanted.contains(&label) {
            let _ = w.close();
        }
    }
    *core.hwnds.lock().unwrap() = hwnds;
}

fn sync_geometry(w: &WebviewWindow, x: i32, y: i32, width: u32, height: u32) {
    let scale = w.scale_factor().unwrap_or(1.0);
    if let Ok(p) = w.outer_position() {
        let p: LogicalPosition<f64> = p.to_logical(scale);
        if (p.x - x as f64).abs() > 1.0 || (p.y - y as f64).abs() > 1.0 {
            let _ = w.set_position(LogicalPosition::new(x as f64, y as f64));
        }
    }
    if let Ok(s) = w.inner_size() {
        let s: LogicalSize<f64> = s.to_logical(scale);
        if (s.width - width as f64).abs() > 1.0 || (s.height - height as f64).abs() > 1.0 {
            let _ = w.set_size(LogicalSize::new(width as f64, height as f64));
        }
    }
}

fn handle_host_event(app: &AppHandle, ev: HostEvent) {
    match ev {
        HostEvent::Drag { id } => {
            let label = label_for(&id);
            if let Some(w) = app.get_webview_window(&label) {
                let core = app.state::<Core>();
                let group = core.config.read(|c| c.move_all_together);
                *core.group_drag.lock().unwrap() = if group {
                    logical_pos(&w).map(|start| GroupDrag {
                        leader: label.clone(),
                        start,
                        others: app
                            .webview_windows()
                            .into_iter()
                            .filter(|(l, _)| l.starts_with(PREFIX) && *l != label)
                            .filter_map(|(l, ow)| logical_pos(&ow).map(|(x, y)| (l, x, y)))
                            .collect(),
                    })
                } else {
                    None
                };
                let _ = w.start_dragging();
            }
        }
        HostEvent::Resize { id, dir } => {
            if let Some(w) = app.get_webview_window(&label_for(&id)) {
                use tauri_runtime::ResizeDirection as D;
                let d = match dir.as_str() {
                    "East" => D::East,
                    "South" => D::South,
                    "SouthWest" => D::SouthWest,
                    "West" => D::West,
                    "North" => D::North,
                    _ => D::SouthEast,
                };
                let _ = w.as_ref().window().start_resize_dragging(d);
            }
        }
        HostEvent::EditMode { .. } | HostEvent::Visible { .. } => {
            *app.state::<Core>().group_drag.lock().unwrap() = None;
            reconcile(app);
        }
        HostEvent::OpenSettings => show_settings(app),
        HostEvent::AutoSize { id, width, height } => {
            let label = label_for(&id);
            let core = app.state::<Core>();
            if width <= 0.0 && height <= 0.0 {
                core.auto_size.lock().unwrap().remove(&label);
                return;
            }
            core.auto_size.lock().unwrap().insert(label.clone(), (width, height));
            if let Some(w) = app.get_webview_window(&label) {
                let scale = w.scale_factor().unwrap_or(1.0);
                if let Ok(size) = w.inner_size() {
                    let size: LogicalSize<f64> = size.to_logical(scale);
                    let nw = if width > 0.0 { width.max(40.0) } else { size.width };
                    let nh = if height > 0.0 { height.max(20.0) } else { size.height };
                    if (size.width - nw).abs() > 1.0 || (size.height - nh).abs() > 1.0 {
                        let _ = w.set_size(LogicalSize::new(nw, nh));
                    }
                }
            }
        }
        HostEvent::Interactive { id, value } => {
            let label = label_for(&id);
            let core = app.state::<Core>();
            let changed = if value { core.interactive.lock().unwrap().insert(label.clone()) } else { core.interactive.lock().unwrap().remove(&label) };
            if changed {
                if let Some(w) = app.get_webview_window(&label) {
                    let edit = core.hub.edit_mode.load(Ordering::Relaxed);
                    let _ = w.set_ignore_cursor_events(!(edit || value));
                }
            }
        }
        HostEvent::CheckUpdate => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move { check_update(app, false).await });
        }
        HostEvent::ClosePrompt => {
            for label in [UPDATE_PROMPT, CLOSE_PROMPT] {
                if let Some(w) = app.get_webview_window(label) {
                    let _ = w.close();
                }
            }
        }
        HostEvent::CloseApp { quit: everything } => {
            if let Some(w) = app.get_webview_window(CLOSE_PROMPT) {
                let _ = w.close();
            }
            if everything {
                quit_all(app);
            } else if let Some(w) = app.get_webview_window(SETTINGS) {
                let _ = w.hide();
            }
        }
        HostEvent::InstallUpdate => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move { install_update(app).await });
        }
    }
}

fn publish_update(app: &AppHandle, v: serde_json::Value) {
    let mut v = v;
    v["current"] = env!("CARGO_PKG_VERSION").into();
    app.state::<Core>().hub.publish("update", &v.to_string());
}

/// Ask GitHub Releases (via the signed `latest.json`) whether a newer version exists.
/// `prompt`: pop up the update window when one is found, unless the user postponed that version.
async fn check_update(app: AppHandle, prompt: bool) {
    use tauri_plugin_updater::UpdaterExt;
    publish_update(&app, serde_json::json!({ "status": "checking" }));
    let result = match app.updater() {
        Ok(u) => u.check().await.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    match result {
        Ok(Some(update)) => {
            publish_update(&app, serde_json::json!({ "status": "available", "version": update.version, "notes": update.body.clone().unwrap_or_default() }));
            let version = update.version.clone();
            *app.state::<Core>().update.lock().await = Some(update);
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
            let postponed = app.state::<Core>().config.read(|c| c.update_postpone.as_ref().is_some_and(|p| p.holds(&version, now)));
            if prompt && !postponed {
                show_update_prompt(&app);
            }
        }
        Ok(None) => publish_update(&app, serde_json::json!({ "status": "none" })),
        Err(e) => publish_update(&app, serde_json::json!({ "status": "error", "message": e })),
    }
}

/// Download the update, verify its signature and run the installer (the app closes and restarts).
async fn install_update(app: AppHandle) {
    let Some(update) = app.state::<Core>().update.lock().await.take() else {
        return check_update(app, false).await;
    };
    let version = update.version.clone();
    let mut done: usize = 0;
    let progress_app = app.clone();
    let mut last = 0u64;
    let result = update
        .download_and_install(
            move |chunk, total| {
                done += chunk;
                let pct = total.map(|t| done as f64 / t as f64 * 100.0).unwrap_or(0.0);
                if pct as u64 != last {
                    last = pct as u64;
                    publish_update(&progress_app, serde_json::json!({ "status": "downloading", "version": version, "progress": pct.round() }));
                }
            },
            || {},
        )
        .await;
    match result {
        Ok(()) => {
            publish_update(&app, serde_json::json!({ "status": "installing" }));
            flush_geometry(&app);
            app.restart();
        }
        Err(e) => publish_update(&app, serde_json::json!({ "status": "error", "message": e.to_string() })),
    }
}

/// Remember where the user put an overlay (flushed to disk by `flush_geometry`).
fn record_geometry(app: &AppHandle, label: &str, pos: Option<(i32, i32)>, size: Option<(u32, u32)>) {
    let core = app.state::<Core>();
    let Some(id) = core.config.read(|c| id_for(c, label)) else { return };
    let mut pending = core.pending.lock().unwrap();
    let e = pending.entry(id).or_default();
    if pos.is_some() {
        e.0 = pos;
    }
    if size.is_some() {
        e.1 = size;
    }
}

fn flush_geometry(app: &AppHandle) {
    let core = app.state::<Core>();
    let pending: Vec<_> = core.pending.lock().unwrap().drain().collect();
    if pending.is_empty() {
        return;
    }
    core.config.update(|c| {
        for (id, (pos, size)) in pending {
            if let Some(o) = c.overlays.iter_mut().find(|o| o.id == id) {
                if let Some((x, y)) = pos {
                    o.x = x;
                    o.y = y;
                }
                if let Some((w, h)) = size {
                    o.width = w;
                    o.height = h;
                }
            }
        }
    });
}

/// What a hotkey does.
#[derive(Clone, Debug)]
enum HotkeyAction {
    ToggleEdit,
    ToggleVisible,
    NextLayout,
    Layout(String),
    Overlay(String),
    VrRecenter,
}

/// The hotkeys in use and what they do (blank ones and ones for missing layouts / overlays left out).
fn hotkey_bindings(c: &Config) -> Vec<(String, HotkeyAction)> {
    let k = &c.hotkeys;
    let mut out = vec![
        (k.toggle_edit.clone(), HotkeyAction::ToggleEdit),
        (k.toggle_visibility.clone(), HotkeyAction::ToggleVisible),
        (k.next_layout.clone(), HotkeyAction::NextLayout),
        (k.vr_recenter.clone(), HotkeyAction::VrRecenter),
    ];
    for l in &c.layouts {
        if let Some(key) = k.layouts.get(&l.id) {
            out.push((key.clone(), HotkeyAction::Layout(l.id.clone())));
        }
    }
    for o in &c.overlays {
        if let Some(key) = k.overlays.get(&o.id) {
            out.push((key.clone(), HotkeyAction::Overlay(o.id.clone())));
        }
    }
    out.retain(|(key, _)| !key.trim().is_empty());
    out
}

/// (Re-)register every global hotkey; called at start-up and whenever the bindings change.
fn register_hotkeys(app: &AppHandle) {
    let core = app.state::<Core>();
    let bindings = core.config.read(hotkey_bindings);
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for (key, action) in bindings {
        let (hub, config) = (core.hub.clone(), core.config.clone());
        let result = gs.on_shortcut(key.trim(), move |_, _, ev| {
            if ev.state != ShortcutState::Pressed {
                return;
            }
            match &action {
                HotkeyAction::ToggleEdit => hub.set_edit_mode(!hub.edit_mode.load(Ordering::Relaxed)),
                HotkeyAction::ToggleVisible => hub.set_visible(!hub.visible.load(Ordering::Relaxed)),
                HotkeyAction::NextLayout => config.update(|c| {
                    if let Some(id) = c.next_layout() {
                        c.apply_layout(&id);
                    }
                }),
                HotkeyAction::Layout(id) => config.update(|c| {
                    c.apply_layout(id);
                }),
                HotkeyAction::Overlay(id) => config.update(|c| c.toggle_overlay(id)),
                HotkeyAction::VrRecenter => vr::RECENTER.store(true, Ordering::Relaxed),
            }
        });
        if let Err(e) = result {
            log::warn!("hotkey {key}: {e}");
        }
    }
}

fn tray(app: &AppHandle) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let edit = MenuItem::with_id(app, "edit", "Toggle edit mode", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "visible", "Show / hide overlays", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &edit, &hide, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main").tooltip("Apex Overlays").menu(&menu).on_menu_event(|app, ev| {
        let hub = app.state::<Core>().hub.clone();
        match ev.id.as_ref() {
            "settings" => show_settings(app),
            "edit" => hub.set_edit_mode(!hub.edit_mode.load(Ordering::Relaxed)),
            "visible" => hub.set_visible(!hub.visible.load(Ordering::Relaxed)),
            "quit" => quit_all(app),
            _ => {}
        }
    });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    let mock = std::env::args().any(|a| a == "--mock");
    let data_dir = apex_core::config::default_data_dir();
    let opts = |port| apex_core::Options { data_dir: data_dir.clone(), mock, ui_dir: None, port };
    // If the configured port is taken, fall back to any free port so the app still works.
    let core = apex_core::start(opts(None)).or_else(|_| apex_core::start(opts(Some(0)))).expect("failed to start Apex Overlays");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_settings(app)))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Core {
            hub: core.hub.clone(),
            config: core.config.clone(),
            url: core.url(),
            port: core.addr.port(),
            pending: Mutex::new(HashMap::new()),
            auto_size: Mutex::new(HashMap::new()),
            interactive: Mutex::new(HashSet::new()),
            group_drag: Mutex::new(None),
            update: tokio::sync::Mutex::new(None),
            hwnds: Mutex::new(HashMap::new()),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            tray(&handle)?;
            register_hotkeys(&handle);
            handle.state::<Core>().hub.updates_supported.store(true, Ordering::Relaxed);
            publish_update(&handle, serde_json::json!({ "status": "idle" }));
            if handle.state::<Core>().config.read(|c| c.auto_check_updates) {
                let h = handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(8)).await;
                    check_update(h, true).await;
                });
            }
            reconcile(&handle);
            vr::spawn(handle.clone());
            // Start with the settings window unless launched to the tray (e.g. at Windows start-up).
            if !std::env::args().any(|a| a == "--tray") {
                show_settings(&handle);
            }
            // React to config edits and requests coming from the overlay/settings pages.
            let (hub, config) = {
                let c = handle.state::<Core>();
                (c.hub.clone(), c.config.clone())
            };
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut host = hub.host_events();
                let mut cfg = config.subscribe();
                let mut tick = tokio_interval();
                // Hotkeys are re-registered only when their bindings change.
                let binding_key = |c: &Config| format!("{:?}", hotkey_bindings(c));
                let mut bound = config.read(binding_key);
                loop {
                    tokio::select! {
                        ev = host.recv() => match ev {
                            Ok(ev) => handle_host_event(&h, ev),
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                            Err(_) => break,
                        },
                        r = cfg.changed() => {
                            if r.is_err() { break; }
                            reconcile(&h);
                            let now = config.read(binding_key);
                            if now != bound {
                                bound = now;
                                register_hotkeys(&h);
                            }
                        }
                        _ = tick.tick() => flush_geometry(&h),
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            let label = window.label().to_string();
            match event {
                WindowEvent::CloseRequested { api, .. } if label == SETTINGS => {
                    // Ask whether the overlays should close too (unless the answer was remembered).
                    api.prevent_close();
                    match app.state::<Core>().config.read(|c| c.close_action.clone()).as_str() {
                        "quit" => quit_all(app),
                        "keep" => {
                            let _ = window.hide();
                        }
                        _ => show_close_prompt(app),
                    }
                }
                WindowEvent::Moved(p) if label.starts_with(PREFIX) => {
                    let s = window.scale_factor().unwrap_or(1.0);
                    let l: LogicalPosition<f64> = p.to_logical(s);
                    record_geometry(app, &label, Some((l.x.round() as i32, l.y.round() as i32)), None);
                    // Move all together: the others follow the overlay being dragged.
                    let core = app.state::<Core>();
                    let group = core.group_drag.lock().unwrap();
                    if let Some(g) = group.as_ref().filter(|g| g.leader == label) {
                        let (dx, dy) = (l.x - g.start.0, l.y - g.start.1);
                        for (other, x, y) in &g.others {
                            if let Some(w) = app.get_webview_window(other) {
                                let _ = w.set_position(LogicalPosition::new(x + dx, y + dy));
                            }
                        }
                    }
                }
                WindowEvent::Resized(sz) if label.starts_with(PREFIX) => {
                    let s = window.scale_factor().unwrap_or(1.0);
                    let l: LogicalSize<f64> = sz.to_logical(s);
                    // Auto-sized overlays only remember the dimensions the user sets.
                    let (aw, ah) = app.state::<Core>().auto_size.lock().unwrap().get(&label).copied().unwrap_or_default();
                    let set = app
                        .state::<Core>()
                        .config
                        .read(|c| id_for(c, &label).and_then(|id| c.overlays.iter().find(|o| o.id == id).map(|o| (o.width, o.height))))
                        .unwrap_or((0, 0));
                    let w = if aw > 0.0 { set.0 } else { l.width.round() as u32 };
                    let h = if ah > 0.0 { set.1 } else { l.height.round() as u32 };
                    record_geometry(app, &label, None, Some((w, h)));
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Apex Overlays")
        .run(|_, event| {
            if let RunEvent::ExitRequested { api, code, .. } = event {
                // Keep running in the tray when the last window closes; only "Quit" exits.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

fn tokio_interval() -> tokio::time::Interval {
    let mut i = tokio::time::interval(Duration::from_millis(400));
    i.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    i
}
