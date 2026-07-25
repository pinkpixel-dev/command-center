//! Settings, the Quick Add window, and the small bits of app metadata the UI
//! shows in About.

use tauri::{AppHandle, Emitter, State};

use crate::db::settings::{self, AppSettings};
use crate::db::Database;
use crate::error::AppResult;
use crate::quick_add;

/// Fired when settings change so every open window can react (theme, mostly).
pub const SETTINGS_CHANGED: &str = "settings-changed";

#[tauri::command]
pub fn get_settings(db: State<'_, Database>) -> AppResult<AppSettings> {
    db.with(settings::load)
}

/// Saves settings and re-registers the global shortcut. A shortcut the OS
/// refuses is reported as an error, and the stored settings keep the previous
/// working accelerator.
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    db: State<'_, Database>,
    settings_input: AppSettings,
) -> AppResult<AppSettings> {
    let previous = db.with(settings::load)?;
    let saved = db.with(|conn| settings::save(conn, settings_input))?;

    #[cfg(desktop)]
    if saved.quick_add_shortcut != previous.quick_add_shortcut {
        if let Err(error) = quick_add::shortcut::register(&app, &saved.quick_add_shortcut) {
            // Put the working shortcut back before telling the user.
            db.with(|conn| settings::save(conn, previous.clone()))?;
            quick_add::shortcut::register(&app, &previous.quick_add_shortcut).ok();
            return Err(error);
        }
    }

    let _ = app.emit(SETTINGS_CHANGED, &saved);
    Ok(saved)
}

#[tauri::command]
pub fn open_quick_add(app: AppHandle) -> AppResult<()> {
    quick_add::show(&app)
}

#[tauri::command]
pub fn close_quick_add(app: AppHandle) -> AppResult<()> {
    quick_add::hide(&app)
}

/// Where the library file lives, shown in Settings so the file is findable.
#[tauri::command]
pub fn library_location(app: AppHandle) -> AppResult<String> {
    let path = crate::library_path(&app)?;
    Ok(path.to_string_lossy().to_string())
}
