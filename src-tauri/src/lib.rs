pub mod db;
pub mod error;
pub mod ipc;
pub mod models;
pub mod normalize;
pub mod quick_add;
pub mod risk;

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::db::Database;
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
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }

    builder
        .setup(|app| {
            let handle = app.handle().clone();
            let database = Database::open(library_path(&handle)?)?;

            #[cfg(desktop)]
            {
                let settings = database.with(db::settings::load)?;
                if let Err(error) =
                    quick_add::shortcut::register(&handle, &settings.quick_add_shortcut)
                {
                    // A shortcut collision must not stop the app from opening.
                    eprintln!("could not register the Quick Add shortcut: {error}");
                }
            }

            app.manage(database);
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
            ipc::system::get_settings,
            ipc::system::save_settings,
            ipc::system::open_quick_add,
            ipc::system::close_quick_add,
            ipc::system::library_location,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Command Center");
}
