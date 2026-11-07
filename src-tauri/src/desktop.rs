//! Desktop-only features: widgets, quick-add window, tray, global shortcut, single instance, autostart.
use crate::{changed, state, Res};
use heatmap_core::Error;
use std::path::PathBuf;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// (kind, title, width, height). The UI for each kind lives in src/widgets.
const WIDGETS: &[(&str, &str, f64, f64)] = &[
    ("ring", "Today ring", 280.0, 340.0),
    ("month", "Neon month", 340.0, 400.0),
    ("strip", "Mono strip", 430.0, 268.0),
    ("sheet", "Paper sheet", 310.0, 400.0),
];

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

#[tauri::command] pub fn open_widget(app: AppHandle, kind: String) -> Res<()> { open_widget_inner(&app, &kind) }
#[tauri::command]
pub fn close_widget(app: AppHandle, kind: String) -> Res<()> {
    if let Some(w) = app.get_webview_window(&format!("w-{kind}")) { let _ = w.close(); }
    let list: Vec<String> = open_list(&app).into_iter().filter(|k| *k != kind).collect();
    persist_open_list(&app, &list);
    changed(&app);
    Ok(())
}
#[tauri::command]
pub fn set_widget_mode(app: AppHandle, kind: String, mode: String) -> Res<()> {
    if !["top", "normal", "bottom"].contains(&mode.as_str()) { return Err(Error::Validation("Unknown widget mode.".into())); }
    if let Some(w) = app.get_webview_window(&format!("w-{kind}")) {
        let _ = w.set_always_on_top(mode == "top");
        let _ = w.set_always_on_bottom(mode == "bottom");
    }
    widget_cfg_update(&app, &kind, |c| c["mode"] = serde_json::json!(mode));
    Ok(())
}
#[tauri::command] pub fn list_open_widgets(app: AppHandle) -> Vec<String> { open_list(&app) }
#[tauri::command] pub fn open_quick(app: AppHandle) -> Res<()> { open_quick_inner(&app) }

fn autostart_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("autostart").join("heatmap.desktop"))
}
#[tauri::command] pub fn get_autostart() -> bool { autostart_file().map_or(false, |p| p.exists()) }
#[tauri::command]
pub fn set_autostart(on: bool) -> Res<()> {
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


/// Plugins and window hooks that only exist on desktop. The single-instance plugin must be registered first.
pub fn init(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    use tauri_plugin_global_shortcut::ShortcutState;
    builder
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| handle_args(app, &argv, false)))
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, _sc, ev| {
            if ev.state() == ShortcutState::Pressed { let _ = open_quick_inner(app); }
        }).build())
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
}

/// Called once the database is ready: tray, global shortcut, remembered widgets, command-line flags.
pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
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
}
