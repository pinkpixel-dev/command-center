pub mod db;
#[cfg(desktop)]
pub mod desktop;
pub mod error;
pub mod export;
pub mod import;
pub mod ipc;
pub mod models;
pub mod normalize;
pub mod recovery;
pub mod risk;

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

/// The SQLite file lives in the platform app-data directory, next to nothing
/// else, so backing up the library means copying one file.
pub fn library_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::runtime(format!("could not resolve app data directory: {err}")))?;
    Ok(dir.join("library.db"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(desktop)]
    let builder = builder
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("Command Center")
                .build(),
        )
        .on_window_event(desktop::handle_window_event);

    builder
        .setup(|app| {
            let handle = app.handle().clone();
            let database = recovery::open_library(&handle, library_path(&handle)?)?;

            app.manage(database);
            #[cfg(desktop)]
            desktop::setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::library::list_commands,
            ipc::library::get_command,
            ipc::library::create_command,
            ipc::library::update_command,
            ipc::library::delete_command,
            ipc::library::toggle_favorite,
            ipc::library::record_copy,
            ipc::library::find_duplicate,
            ipc::library::library_stats,
            ipc::library::list_tags,
            ipc::library::list_collections,
            ipc::library::create_collection,
            ipc::library::update_collection,
            ipc::library::delete_collection,
            ipc::import::preview_import_text,
            ipc::import::preview_import_file,
            ipc::import::analyze_snippet,
            ipc::import::import_commands,
            ipc::system::get_settings,
            ipc::system::save_settings,
            ipc::system::library_location,
            ipc::system::export_library_markdown,
            ipc::system::backup_library,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Command Center");
}
