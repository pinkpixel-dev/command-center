//! Import commands. Reading a file happens here rather than in the frontend,
//! so the app never needs filesystem permissions in the webview.

use std::path::Path;

use tauri::{AppHandle, State};

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::import::apply::{ImportItem, ImportSummary};
use crate::import::{self, ImportPreview, SnippetAnalysis};
use crate::ipc::library::announce_change;

/// Extensions the file picker and drag-drop accept. Anything else is almost
/// certainly not a document worth parsing.
const READABLE_EXTENSIONS: [&str; 9] = [
    "md", "markdown", "mdx", "txt", "text", "rst", "adoc", "org", "sh",
];

/// A file bigger than this is not a cheat sheet, and parsing it would freeze
/// the window for no good reason.
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

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
    let content = read_document(&path)?;
    let name = Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string());

    db.with(|conn| import::preview(conn, &content, name.as_deref()))
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
    let summary = db.with_mut(|conn| import::apply::run(conn, items))?;
    if summary.touched() > 0 {
        announce_change(&app);
    }
    Ok(summary)
}

fn read_document(path: &str) -> AppResult<String> {
    let path = Path::new(path);

    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // README and LICENSE style files have no extension at all, which is fine.
    if !extension.is_empty() && !READABLE_EXTENSIONS.contains(&extension.as_str()) {
        return Err(AppError::invalid(format!(
            "Command Center reads text and Markdown files. \"{}\" is a .{} file.",
            path.file_name().unwrap_or_default().to_string_lossy(),
            extension
        )));
    }

    let metadata = std::fs::metadata(path)
        .map_err(|error| AppError::invalid(format!("Could not open that file: {error}")))?;

    if metadata.is_dir() {
        return Err(AppError::invalid("That is a folder, not a document"));
    }

    if metadata.len() > MAX_FILE_BYTES {
        return Err(AppError::invalid(
            "That file is larger than 4 MB. Paste the part you want instead.",
        ));
    }

    let bytes = std::fs::read(path)
        .map_err(|error| AppError::invalid(format!("Could not read that file: {error}")))?;

    String::from_utf8(bytes)
        .map_err(|_| AppError::invalid("That file is not text Command Center can read"))
}
