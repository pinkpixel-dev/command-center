//! Import commands. Reading a file happens here rather than in the frontend,
//! so the app never needs filesystem permissions in the webview.

use std::path::Path;

use tauri::{AppHandle, State};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::events::DesktopEvents;
use crate::import::apply::{ImportItem, ImportSummary};
use crate::import::document::{self, ImportDocument};
use crate::import::{self, ImportPreview, SnippetAnalysis};
use crate::library;

#[tauri::command]
pub fn read_import_document(path: String) -> AppResult<ImportDocument> {
    read_file(&path)
}

#[tauri::command]
pub fn preview_import_text(
    db: State<'_, Database>,
    content: String,
    source_name: Option<String>,
) -> AppResult<ImportPreview> {
    db.with(|conn| import::preview(conn, &content, source_name.as_deref()))
}

#[tauri::command]
pub fn preview_import_file(db: State<'_, Database>, path: String) -> AppResult<ImportPreview> {
    let file = read_file(&path)?;

    db.with(|conn| import::preview(conn, &file.content, file.name.as_deref()))
}

/// Recalculates one snippet after the user edits, splits, or merges it.
#[tauri::command]
pub fn analyze_snippet(db: State<'_, Database>, content: String) -> AppResult<SnippetAnalysis> {
    db.with(|conn| import::analyze(conn, &content))
}

#[tauri::command]
pub fn import_commands(
    app: AppHandle,
    db: State<'_, Database>,
    items: Vec<ImportItem>,
) -> AppResult<ImportSummary> {
    library::import_commands(&db, &DesktopEvents(app), items)
}

/// Reads a document off the local disk. What makes a file readable is decided
/// in core, so the desktop app and the server refuse the same file the same
/// way. Only the parts that need a real path are here.
fn read_file(path: &str) -> AppResult<ImportDocument> {
    let path = Path::new(path);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string());

    if let Some(name) = name.as_deref() {
        document::check_name(name)?;
    }

    let metadata = std::fs::metadata(path)
        .map_err(|error| AppError::invalid(format!("Could not open that file: {error}")))?;

    if metadata.is_dir() {
        return Err(AppError::invalid("That is a folder, not a document"));
    }

    // Checked before the read so a huge file is refused rather than loaded.
    document::check_size(metadata.len())?;

    let bytes = std::fs::read(path)
        .map_err(|error| AppError::invalid(format!("Could not read that file: {error}")))?;

    document::from_bytes(name, bytes)
}
