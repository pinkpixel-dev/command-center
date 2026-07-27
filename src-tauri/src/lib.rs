pub mod ai;
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

#[cfg(target_os = "linux")]
const DISABLED_GIO_MODULE_PATH: &str = "/__command_center_appimage_disabled_gio_modules__";

#[cfg(target_os = "linux")]
fn appimage_gio_module_path(appimage: Option<&std::ffi::OsStr>) -> Option<&'static str> {
    appimage.map(|_| DISABLED_GIO_MODULE_PATH)
}

#[cfg(target_os = "linux")]
fn apply_appimage_gio_workaround() {
    let Some(disabled_path) = appimage_gio_module_path(std::env::var_os("APPIMAGE").as_deref())
    else {
        return;
    };

    std::env::set_var("GIO_MODULE_DIR", disabled_path);
    std::env::set_var("GIO_EXTRA_MODULES", disabled_path);
}

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
    #[cfg(target_os = "linux")]
    apply_appimage_gio_workaround();

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
            let ai_service = ai::AiService::new()?;

            app.manage(database);
            app.manage(ai_service);
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
            ipc::import::read_import_document,
            ipc::import::preview_import_text,
            ipc::import::preview_import_file,
            ipc::import::analyze_snippet,
            ipc::import::import_commands,
            ipc::system::get_settings,
            ipc::system::save_settings,
            ipc::system::library_location,
            ipc::system::export_library_markdown,
            ipc::system::backup_library,
            ipc::ai::get_ai_status,
            ipc::ai::save_ai_key,
            ipc::ai::remove_ai_key,
            ipc::ai::test_ai_connection,
            ipc::ai_import::prepare_ai_import,
            ipc::ai_import::run_ai_import,
            ipc::ai_assistant::ask_assistant,
            ipc::ai_assistant::cancel_assistant_request,
            ipc::ai_explain::get_command_explanation,
            ipc::ai_explain::explain_command,
            ipc::ai_explain::clear_ai_explanations,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Command Center");
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn gio_workaround_only_has_a_path_inside_an_appimage() {
        assert_eq!(appimage_gio_module_path(None), None);
        assert_eq!(
            appimage_gio_module_path(Some(std::ffi::OsStr::new("/tmp/Command_Center.AppImage"))),
            Some(DISABLED_GIO_MODULE_PATH)
        );
    }
}
