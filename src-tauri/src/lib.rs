//! Thin Tauri layer: commands delegate to `heatmap-core`; this file owns windows, tray and shortcuts.
use base64::Engine;
use heatmap_core::capture::{parse_capture, Capture};
use heatmap_core::git::SyncReport;
use heatmap_core::insights::{compute_insights, correlate, Correlation, Insights};
use heatmap_core::io::ImportReport;
use heatmap_core::dates::parse_date;
use heatmap_core::{DayAgg, Db, Entry, Error, Source, Stats, Tracker, TrackerInput};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// (kind, title, width, height). The UI for each kind lives in src/widgets.
const WIDGETS: &[(&str, &str, f64, f64)] = &[
    ("ring", "Today ring", 280.0, 340.0),
    ("month", "Neon month", 340.0, 400.0),
    ("strip", "Mono strip", 430.0, 268.0),
    ("sheet", "Paper sheet", 310.0, 400.0),
];
const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;

struct AppState {
    db: Mutex<Db>,
    data_dir: PathBuf,
    pending_pos: Arc<Mutex<HashMap<String, (i32, i32)>>>,
}
type Res<T> = Result<T, Error>;

fn db<'a>(s: &'a State<AppState>) -> Res<MutexGuard<'a, Db>> {
    s.db.lock().map_err(|_| Error::Internal("The database lock was poisoned; please restart the app.".into()))
}
fn changed(app: &AppHandle) { let _ = app.emit("data-changed", ()); }
fn state(app: &AppHandle) -> State<'_, AppState> { app.state::<AppState>() }

// ---- trackers & entries ------------------------------------------------------------------
#[tauri::command] fn list_trackers(s: State<AppState>) -> Res<Vec<Tracker>> { db(&s)?.list_trackers() }
#[tauri::command] fn create_tracker(app: AppHandle, s: State<AppState>, input: TrackerInput) -> Res<Tracker> { let r = db(&s)?.create_tracker(input)?; changed(&app); Ok(r) }
#[tauri::command] fn update_tracker(app: AppHandle, s: State<AppState>, id: String, input: TrackerInput) -> Res<Tracker> { let r = db(&s)?.update_tracker(&id, input)?; changed(&app); Ok(r) }
#[tauri::command] fn delete_tracker(app: AppHandle, s: State<AppState>, id: String) -> Res<()> { db(&s)?.delete_tracker(&id)?; changed(&app); Ok(()) }
#[tauri::command] fn day_aggregates(s: State<AppState>, tracker_id: String, from: String, to: String) -> Res<Vec<DayAgg>> { db(&s)?.day_aggregates(&tracker_id, &from, &to) }
#[tauri::command] fn list_entries(s: State<AppState>, tracker_id: String, date: String) -> Res<Vec<Entry>> { db(&s)?.list_entries(&tracker_id, &date) }
#[tauri::command] fn add_entry(app: AppHandle, s: State<AppState>, tracker_id: String, date: String, value: f64, note: String) -> Res<Entry> { let r = db(&s)?.add_entry(&tracker_id, &date, value, &note)?; changed(&app); Ok(r) }
#[tauri::command] fn update_entry(app: AppHandle, s: State<AppState>, id: String, value: f64, note: String) -> Res<Entry> { let r = db(&s)?.update_entry(&id, value, &note)?; changed(&app); Ok(r) }
#[tauri::command] fn delete_entry(app: AppHandle, s: State<AppState>, id: String) -> Res<()> { db(&s)?.delete_entry(&id)?; changed(&app); Ok(()) }
#[tauri::command] fn get_stats(s: State<AppState>, tracker_id: String, today: String, week_start: u32) -> Res<Stats> { db(&s)?.stats(&tracker_id, &today, week_start) }
#[tauri::command] fn get_settings(s: State<AppState>) -> Res<BTreeMap<String, String>> { db(&s)?.get_settings() }
#[tauri::command] fn set_setting(s: State<AppState>, key: String, value: String) -> Res<()> { db(&s)?.set_setting(&key, &value) }

// ---- freezes, sources, capture, insights -------------------------------------------------
#[tauri::command] fn list_freezes(s: State<AppState>, tracker_id: String, from: String, to: String) -> Res<Vec<String>> { db(&s)?.list_freezes(&tracker_id, &from, &to) }
#[tauri::command] fn set_freeze(app: AppHandle, s: State<AppState>, tracker_id: String, date: String, on: bool) -> Res<()> { db(&s)?.set_freeze(&tracker_id, &date, on)?; changed(&app); Ok(()) }
#[tauri::command] fn list_sources(s: State<AppState>, tracker_id: String) -> Res<Vec<Source>> { db(&s)?.list_sources(&tracker_id) }
#[tauri::command] fn add_git_source(app: AppHandle, s: State<AppState>, tracker_id: String, path: String) -> Res<Source> { let r = db(&s)?.add_git_source(&tracker_id, &path)?; changed(&app); Ok(r) }
#[tauri::command] fn sync_source(app: AppHandle, s: State<AppState>, source_id: String) -> Res<SyncReport> { let r = db(&s)?.sync_source(&source_id)?; changed(&app); Ok(r) }
#[tauri::command] fn remove_source(app: AppHandle, s: State<AppState>, source_id: String, delete_entries: bool) -> Res<()> { db(&s)?.remove_source(&source_id, delete_entries)?; changed(&app); Ok(()) }

#[tauri::command]
fn capture_preview(s: State<AppState>, text: String, today: String) -> Res<Capture> {
    let d = db(&s)?;
    parse_capture(&text, parse_date(&today)?, &d.list_trackers()?)
}
#[tauri::command]
fn quick_capture(app: AppHandle, s: State<AppState>, text: String, today: String) -> Res<Capture> {
    let mut d = db(&s)?;
    let c = parse_capture(&text, parse_date(&today)?, &d.list_trackers()?)?;
    d.add_entry(&c.tracker_id, &c.date, c.value, &c.note)?;
    changed(&app);
    Ok(c)
}
#[tauri::command]
fn get_insights(s: State<AppState>, tracker_id: String, today: String) -> Res<Insights> {
    let d = db(&s)?;
    Ok(compute_insights(&d.daily_values(&tracker_id)?, parse_date(&today)?))
}
#[tauri::command]
fn get_correlation(s: State<AppState>, a: String, b: String, lag: i64, days: i64, today: String) -> Res<Option<Correlation>> {
    let d = db(&s)?;
    let to = parse_date(&today)?;
    let from = to - chrono::Duration::days(days.clamp(14, 1500));
    Ok(correlate(&d.daily_values(&a)?, &d.daily_values(&b)?, lag.clamp(-14, 14), from, to))
}

// ---- files -------------------------------------------------------------------------------
#[tauri::command]
fn export_to_file(s: State<AppState>, kind: String, path: String, tracker_id: Option<String>) -> Res<()> {
    let d = db(&s)?;
    let text = match kind.as_str() {
        "json" => d.export_json()?,
        "csv" => d.export_csv(tracker_id.as_deref())?,
        _ => return Err(Error::Validation("Unknown export type.".into())),
    };
    std::fs::write(path, text)?;
    Ok(())
}
#[tauri::command]
fn import_from_file(app: AppHandle, s: State<AppState>, path: String) -> Res<ImportReport> {
    if std::fs::metadata(&path)?.len() > MAX_IMPORT_BYTES { return Err(Error::Validation("That file is too large to import (limit 50 MB).".into())); }
    let text = std::fs::read_to_string(&path)?;
    let mut d = db(&s)?;
    let r = if path.to_lowercase().ends_with(".csv") {
        d.import_csv(&text)?
    } else {
        // JSON import replaces everything, so keep an automatic safety copy first.
        let dir = s.data_dir.join("backups");
        std::fs::create_dir_all(&dir)?;
        d.backup_to(&dir.join(format!("pre-import-{}.db", chrono::Local::now().format("%Y%m%d-%H%M%S"))))?;
        d.import_json(&text)?
    };
    changed(&app);
    Ok(r)
}
#[tauri::command] fn backup_db(s: State<AppState>, path: String) -> Res<()> { db(&s)?.backup_to(Path::new(&path)) }

/// Image/SVG exports: only .svg and .png, size-limited, written exactly where the user chose in the save dialog.
#[tauri::command]
fn write_export(path: String, text: Option<String>, data_url: Option<String>) -> Res<()> {
    let lower = path.to_lowercase();
    if let (true, Some(t)) = (lower.ends_with(".svg"), text) {
        if t.len() > 20_000_000 { return Err(Error::Validation("Export is too large.".into())); }
        std::fs::write(path, t)?;
    } else if let (true, Some(u)) = (lower.ends_with(".png"), data_url) {
        let b64 = u.strip_prefix("data:image/png;base64,").ok_or_else(|| Error::Validation("Not a PNG image.".into()))?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| Error::Validation("Corrupt image data.".into()))?;
        if bytes.len() > 40_000_000 { return Err(Error::Validation("Export is too large.".into())); }
        std::fs::write(path, bytes)?;
    } else {
        return Err(Error::Validation("Unsupported export format.".into()));
    }
    Ok(())
}

// ---- widgets, quick add, autostart --------------------------------------------------------
fn widget_cfg(app: &AppHandle) -> serde_json::Value {
    state(app).db.lock().ok().and_then(|d| d.get_settings().ok()).and_then(|m| m.get("widgetCfg").cloned())
        .and_then(|s| serde_json::from_str(&s).ok()).filter(|v: &serde_json::Value| v.is_object()).unwrap_or_else(|| serde_json::json!({}))
}
fn widget_cfg_update(app: &AppHandle, kind: &str, f: impl FnOnce(&mut serde_json::Value)) {
    let mut cfg = widget_cfg(app);
    let entry = cfg.as_object_mut().unwrap().entry(kind.to_string()).or_insert_with(|| serde_json::json!({}));
    f(entry);
    if let Ok(d) = state(app).db.lock().as_deref_mut() { let _ = d.set_setting("widgetCfg", &cfg.to_string()); }
}
fn open_list(app: &AppHandle) -> Vec<String> {
    app.webview_windows().keys().filter_map(|l| l.strip_prefix("w-").map(String::from)).collect()
}
fn persist_open_list(app: &AppHandle, kinds: &[String]) {
    if let Ok(d) = state(app).db.lock().as_deref_mut() { let _ = d.set_setting("widgetsOpen", &kinds.join(",")); }
}

fn open_widget_inner(app: &AppHandle, kind: &str) -> Res<()> {
    let &(_, title, w, h) = WIDGETS.iter().find(|x| x.0 == kind).ok_or_else(|| Error::Validation("Unknown widget.".into()))?;
    let label = format!("w-{kind}");
    if let Some(win) = app.get_webview_window(&label) { let _ = win.show(); let _ = win.set_focus(); return Ok(()); }
    let cfg = widget_cfg(app);
    let mode = cfg[kind]["mode"].as_str().unwrap_or("top").to_string();
    let mut b = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(format!("index.html#/w/{kind}").into()))
        .title(format!("Heatmap – {title}")).inner_size(w, h).decorations(false).transparent(true).shadow(false)
        .resizable(false).skip_taskbar(true).visible_on_all_workspaces(true)
        .always_on_top(mode == "top").always_on_bottom(mode == "bottom");
    if let (Some(x), Some(y)) = (cfg[kind]["x"].as_f64(), cfg[kind]["y"].as_f64()) { b = b.position(x, y); }
    b.build().map_err(|e| Error::Internal(e.to_string()))?;
    let mut list = open_list(app);
    list.sort();
    persist_open_list(app, &list);
    changed(app);
    Ok(())
}

fn open_quick_inner(app: &AppHandle) -> Res<()> {
    if let Some(w) = app.get_webview_window("quick") { let _ = w.show(); let _ = w.set_focus(); return Ok(()); }
    WebviewWindowBuilder::new(app, "quick", WebviewUrl::App("index.html#/quick".into()))
        .title("Quick add").inner_size(600.0, 150.0).decorations(false).transparent(true).shadow(false)
        .always_on_top(true).resizable(false).skip_taskbar(true).center().focused(true)
        .build().map_err(|e| Error::Internal(e.to_string()))?;
    Ok(())
}

#[tauri::command] fn open_widget(app: AppHandle, kind: String) -> Res<()> { open_widget_inner(&app, &kind) }
#[tauri::command]
fn close_widget(app: AppHandle, kind: String) -> Res<()> {
    if let Some(w) = app.get_webview_window(&format!("w-{kind}")) { let _ = w.close(); }
    let list: Vec<String> = open_list(&app).into_iter().filter(|k| *k != kind).collect();
    persist_open_list(&app, &list);
    changed(&app);
    Ok(())
}
#[tauri::command]
fn set_widget_mode(app: AppHandle, kind: String, mode: String) -> Res<()> {
    if !["top", "normal", "bottom"].contains(&mode.as_str()) { return Err(Error::Validation("Unknown widget mode.".into())); }
    if let Some(w) = app.get_webview_window(&format!("w-{kind}")) {
        let _ = w.set_always_on_top(mode == "top");
        let _ = w.set_always_on_bottom(mode == "bottom");
    }
    widget_cfg_update(&app, &kind, |c| c["mode"] = serde_json::json!(mode));
    Ok(())
}
#[tauri::command] fn list_open_widgets(app: AppHandle) -> Vec<String> { open_list(&app) }
#[tauri::command] fn open_quick(app: AppHandle) -> Res<()> { open_quick_inner(&app) }

fn autostart_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("autostart").join("heatmap.desktop"))
}
#[tauri::command] fn get_autostart() -> bool { autostart_file().map_or(false, |p| p.exists()) }
#[tauri::command]
fn set_autostart(on: bool) -> Res<()> {
    let p = autostart_file().ok_or_else(|| Error::Internal("Could not find your config folder.".into()))?;
    if on {
        std::fs::create_dir_all(p.parent().unwrap())?;
        std::fs::write(&p, "[Desktop Entry]\nType=Application\nName=Heatmap\nExec=heatmap-app --background\nX-GNOME-Autostart-enabled=true\nTerminal=false\n")?;
    } else if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.unminimize(); let _ = w.set_focus(); }
}

/// Handles `--widget <kind>`, `--quick`, `--show`, `--background`; used for the first launch and for later launches (single instance).
fn handle_args(app: &AppHandle, args: &[String], first: bool) {
    let mut shown_something = false;
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--widget" => if let Some(k) = it.next() { let _ = open_widget_inner(app, k); shown_something = true; },
            "--quick" => { let _ = open_quick_inner(app); shown_something = true; }
            "--background" => if first { if let Some(w) = app.get_webview_window("main") { let _ = w.hide(); } shown_something = true; },
            "--show" => { show_main(app); shown_something = true; }
            _ => {}
        }
    }
    if !first && !shown_something { show_main(app); }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open Heatmap", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "Quick add…", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let mut items: Vec<MenuItem<tauri::Wry>> = vec![];
    for (kind, title, _, _) in WIDGETS { items.push(MenuItem::with_id(app, format!("widget:{kind}"), format!("Widget: {title}"), true, None::<&str>)?); }
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let mut refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = vec![&show, &quick, &sep];
    for i in &items { refs.push(i); }
    refs.push(&sep2);
    refs.push(&quit);
    let menu = Menu::with_items(app, &refs)?;
    let mut b = TrayIconBuilder::with_id("main").menu(&menu).tooltip("Heatmap").show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id.as_ref() {
            "show" => show_main(app),
            "quick" => { let _ = open_quick_inner(app); }
            "quit" => app.exit(0),
            id => if let Some(k) = id.strip_prefix("widget:") { let _ = open_widget_inner(app, k); },
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev { show_main(tray.app_handle()); }
        });
    if let Some(icon) = app.default_window_icon() { b = b.icon(icon.clone()); }
    b.build(app)?;
    Ok(())
}

/// Remember where each widget was dropped (debounced so dragging does not hammer the database).
fn track_widget_move(app: &AppHandle, kind: String, x: i32, y: i32) {
    let pending = state(app).pending_pos.clone();
    let first = { let mut m = pending.lock().unwrap(); let was_empty = !m.contains_key(&kind); m.insert(kind.clone(), (x, y)); was_empty };
    if first {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(900));
            let pos = state(&app).pending_pos.lock().unwrap().remove(&kind);
            if let Some((x, y)) = pos { widget_cfg_update(&app, &kind, |c| { c["x"] = serde_json::json!(x); c["y"] = serde_json::json!(y); }); }
        });
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| handle_args(app, &argv, false)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, _sc, ev| {
            if ev.state() == ShortcutState::Pressed { let _ = open_quick_inner(app); }
        }).build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let file = std::env::var_os("HEATMAP_DB").map(PathBuf::from).unwrap_or_else(|| data_dir.join("heatmap.db"));
            let db = Db::open(&file)?;
            app.manage(AppState { db: Mutex::new(db), data_dir, pending_pos: Arc::new(Mutex::new(HashMap::new())) });
            let h = app.handle().clone();
            if let Err(e) = build_tray(&h) { eprintln!("tray unavailable: {e}"); }
            // Ctrl+Alt+H opens quick add. Wayland blocks global shortcuts; `heatmap-app --quick` works everywhere.
            if let Err(e) = app.global_shortcut().register(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyH)) {
                eprintln!("global shortcut unavailable: {e}");
            }
            let saved = state(&h).db.lock().ok().and_then(|d| d.get_settings().ok()).and_then(|m| m.get("widgetsOpen").cloned()).unwrap_or_default();
            for k in saved.split(',').filter(|k| !k.is_empty()) { let _ = open_widget_inner(&h, k); }
            let args: Vec<String> = std::env::args().collect();
            handle_args(&h, &args, true);
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();
            match event {
                WindowEvent::Moved(p) => if let Some(k) = label.strip_prefix("w-") { track_widget_move(window.app_handle(), k.to_string(), p.x, p.y); },
                WindowEvent::CloseRequested { api, .. } if label == "main" => {
                    // Keep running in the background while widgets are on the desktop.
                    if !open_list(window.app_handle()).is_empty() { api.prevent_close(); let _ = window.hide(); }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_trackers, create_tracker, update_tracker, delete_tracker, day_aggregates, list_entries,
            add_entry, update_entry, delete_entry, get_stats, get_settings, set_setting,
            list_freezes, set_freeze, list_sources, add_git_source, sync_source, remove_source,
            capture_preview, quick_capture, get_insights, get_correlation,
            export_to_file, import_from_file, backup_db, write_export,
            open_widget, close_widget, set_widget_mode, list_open_widgets, open_quick, get_autostart, set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Heatmap application");
}
