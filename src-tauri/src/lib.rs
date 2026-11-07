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
use tauri::{AppHandle, Emitter, Manager, State};

const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;

#[cfg(desktop)]
#[path = "desktop.rs"]
mod platform;
#[cfg(mobile)]
#[path = "mobile.rs"]
mod platform;

#[allow(dead_code)]
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
#[allow(dead_code)]
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    platform::init(tauri::Builder::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let file = std::env::var_os("HEATMAP_DB").map(PathBuf::from).unwrap_or_else(|| data_dir.join("heatmap.db"));
            let db = Db::open(&file)?;
            app.manage(AppState { db: Mutex::new(db), data_dir, pending_pos: Arc::new(Mutex::new(HashMap::new())) });
            platform::setup(app)
        })
        .invoke_handler(tauri::generate_handler![
            list_trackers, create_tracker, update_tracker, delete_tracker, day_aggregates, list_entries,
            add_entry, update_entry, delete_entry, get_stats, get_settings, set_setting,
            list_freezes, set_freeze, list_sources, add_git_source, sync_source, remove_source,
            capture_preview, quick_capture, get_insights, get_correlation,
            export_to_file, import_from_file, backup_db, write_export,
            platform::open_widget, platform::close_widget, platform::set_widget_mode, platform::list_open_widgets,
            platform::open_quick, platform::get_autostart, platform::set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Heatmap application");
}
