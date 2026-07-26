//! Startup recovery for a library file SQLite cannot open.

use std::path::{Path, PathBuf};

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::db::Database;
use crate::error::{AppError, AppResult};

pub fn open_library(app: &AppHandle, path: PathBuf) -> AppResult<Database> {
    match Database::open(&path) {
        Ok(database) => Ok(database),
        Err(error) if path.exists() => {
            let recover = app
                .dialog()
                .message(format!(
                    "Command Center could not open its library:\n\n{}\n\n\
                     The existing file can be preserved under a new name before \
                     Command Center starts with an empty library.",
                    path.to_string_lossy()
                ))
                .title("Library could not be opened")
                .kind(MessageDialogKind::Error)
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "Preserve and start fresh".into(),
                    "Quit".into(),
                ))
                .blocking_show();

            if !recover {
                return Err(error);
            }

            let preserved = preserve_unreadable_library(&path)?;
            let database = Database::open(&path)?;
            app.dialog()
                .message(format!(
                    "The unreadable library was preserved here:\n\n{}",
                    preserved.to_string_lossy()
                ))
                .title("Fresh library created")
                .kind(MessageDialogKind::Info)
                .blocking_show();
            Ok(database)
        }
        Err(error) => Err(error),
    }
}

pub fn preserve_unreadable_library(path: &Path) -> AppResult<PathBuf> {
    if !path.exists() {
        return Err(AppError::not_found("The unreadable library file"));
    }

    let parent = path
        .parent()
        .ok_or_else(|| AppError::runtime("the library path has no parent directory"))?;
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let mut preserved = parent.join(format!("library-unreadable-{timestamp}.db"));
    let mut collision = 1;
    while preserved.exists() {
        preserved = parent.join(format!("library-unreadable-{timestamp}-{collision}.db"));
        collision += 1;
    }

    std::fs::rename(path, &preserved).map_err(|error| {
        AppError::runtime(format!("could not preserve unreadable library: {error}"))
    })?;

    preserve_sidecar(path, &preserved, "-wal")?;
    preserve_sidecar(path, &preserved, "-shm")?;
    Ok(preserved)
}

fn preserve_sidecar(original: &Path, preserved: &Path, suffix: &str) -> AppResult<()> {
    let original = PathBuf::from(format!("{}{suffix}", original.to_string_lossy()));
    if !original.exists() {
        return Ok(());
    }

    let preserved = PathBuf::from(format!("{}{suffix}", preserved.to_string_lossy()));
    std::fs::rename(original, preserved)
        .map_err(|error| AppError::runtime(format!("could not preserve SQLite sidecar: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_library_and_sidecars_are_preserved_without_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("library.db");
        let wal = dir.path().join("library.db-wal");
        std::fs::write(&library, b"not sqlite").unwrap();
        std::fs::write(&wal, b"pending pages").unwrap();

        let preserved = preserve_unreadable_library(&library).unwrap();

        assert!(!library.exists());
        assert_eq!(std::fs::read(&preserved).unwrap(), b"not sqlite");
        assert_eq!(
            std::fs::read(format!("{}-wal", preserved.to_string_lossy())).unwrap(),
            b"pending pages"
        );
    }
}
