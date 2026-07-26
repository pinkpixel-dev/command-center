//! Settings and the small bits of app metadata the UI shows in About.

use tauri::{AppHandle, Emitter, State};

use crate::db::settings::{self, AppSettings};
use crate::db::Database;
use crate::error::AppResult;

/// Fired when settings change so every open window can react (theme, mostly).
pub const SETTINGS_CHANGED: &str = "settings-changed";

#[tauri::command]
pub fn get_settings(db: State<'_, Database>) -> AppResult<AppSettings> {
    db.with(settings::load)
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    db: State<'_, Database>,
    settings_input: AppSettings,
) -> AppResult<AppSettings> {
    let saved = db.with(|conn| settings::save(conn, settings_input))?;

    let _ = app.emit(SETTINGS_CHANGED, &saved);
    Ok(saved)
}

/// Where the library file lives, shown in Settings so the file is findable.
#[tauri::command]
pub fn library_location(app: AppHandle) -> AppResult<String> {
    let path = crate::library_path(&app)?;
    Ok(path.to_string_lossy().to_string())
}
