//! Android/iOS: desktop-only features are unavailable. The commands exist so the UI can call one handler list everywhere.
use crate::Res;
use heatmap_core::Error;

fn unsupported() -> Error { Error::Validation("This feature is only available in the desktop app.".into()) }

#[tauri::command] pub fn open_widget(_kind: String) -> Res<()> { Err(unsupported()) }
#[tauri::command] pub fn close_widget(_kind: String) -> Res<()> { Err(unsupported()) }
#[tauri::command] pub fn set_widget_mode(_kind: String, _mode: String) -> Res<()> { Err(unsupported()) }
#[tauri::command] pub fn list_open_widgets() -> Vec<String> { vec![] }
#[tauri::command] pub fn open_quick() -> Res<()> { Err(unsupported()) }
#[tauri::command] pub fn get_autostart() -> bool { false }
#[tauri::command] pub fn set_autostart(_on: bool) -> Res<()> { Err(unsupported()) }

pub fn init(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> { builder }
pub fn setup(_app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
