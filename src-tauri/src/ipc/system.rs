//! Settings and the small bits of app metadata the UI shows in About.

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};

use crate::db::settings::{self, AppSettings};
use crate::db::Database;
use crate::error::{AppError, AppResult};

/// Fired when settings change so every open window can react (theme, mostly).
pub const SETTINGS_CHANGED: &str = "settings-changed";

#[tauri::command]
pub fn get_settings(app: AppHandle, db: State<'_, Database>) -> AppResult<AppSettings> {
    let mut loaded = db.with(settings::load)?;

    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        if let Ok(enabled) = app.autolaunch().is_enabled() {
            loaded.launch_at_startup = enabled;
        }
    }

    Ok(loaded)
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    db: State<'_, Database>,
    settings_input: AppSettings,
) -> AppResult<AppSettings> {
    let previous = db.with(settings::load)?;

    #[cfg(desktop)]
    if previous.launch_at_startup != settings_input.launch_at_startup {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        let result = if settings_input.launch_at_startup {
            manager.enable()
        } else {
            manager.disable()
        };
        result.map_err(|error| {
            AppError::runtime(format!(
                "could not update launch-at-startup setting: {error}"
            ))
        })?;
    }

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

#[tauri::command]
pub fn export_library_markdown(db: State<'_, Database>, destination: String) -> AppResult<String> {
    let destination = PathBuf::from(destination);
    crate::export::export_markdown(&db, &destination)?;
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_collection_markdown(
    db: State<'_, Database>,
    collection_id: i64,
    destination: String,
) -> AppResult<String> {
    let destination = PathBuf::from(destination);
    crate::export::export_collection_markdown(&db, collection_id, &destination)?;
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn backup_library(
    app: AppHandle,
    db: State<'_, Database>,
    destination: String,
) -> AppResult<String> {
    let destination = PathBuf::from(destination);
    let source = crate::library_path(&app)?;
    crate::export::backup_database(&db, &source, &destination)?;
    Ok(destination.to_string_lossy().to_string())
}
