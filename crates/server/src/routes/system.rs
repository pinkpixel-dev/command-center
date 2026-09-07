//! Settings, app metadata, and the two ways a library leaves the server.
//!
//! Exporting and backing up name a save location on the desktop. Here they are
//! downloads: the same core code produces the same bytes, and the response
//! carries the filename the browser saves it under.

use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::State;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use command_center_core::db::settings::{self, AppSettings};
use command_center_core::error::{AppError, AppResult};
use command_center_core::{export, library};

use crate::download;
use crate::error::{ok, ApiError, ApiResult};
use crate::state::AppState;

/// Names each in-flight backup's temporary file apart from the others.
static BACKUPS: AtomicU64 = AtomicU64::new(0);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/get_settings", post(get_settings))
        .route("/save_settings", post(save_settings))
        .route("/library_location", post(library_location))
        .route("/export_library_markdown", post(export_library_markdown))
        .route(
            "/export_collection_markdown",
            post(export_collection_markdown),
        )
        .route("/backup_library", post(backup_library))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionBody {
    collection_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsBody {
    settings_input: AppSettings,
}

async fn get_settings(State(state): State<AppState>) -> ApiResult<AppSettings> {
    ok(state.db.with(settings::load)?)
}

async fn save_settings(
    State(state): State<AppState>,
    Json(body): Json<SettingsBody>,
) -> ApiResult<AppSettings> {
    ok(library::save_settings(
        &state.db,
        state.events.as_ref(),
        body.settings_input,
    )?)
}

/// Where the library file lives. This is the path inside the container, which
/// is the one an operator needs when they go looking for it on the mount.
async fn library_location(State(state): State<AppState>) -> ApiResult<String> {
    ok(state.config.library_path().to_string_lossy().to_string())
}

/// The whole library as one Markdown file.
async fn export_library_markdown(State(state): State<AppState>) -> Result<Response, ApiError> {
    let content = export::library_markdown(&state.db)?;

    Ok(download::attachment(
        content.into_bytes(),
        download::MARKDOWN,
        "command-center-library.md",
    ))
}

/// One collection as Markdown, named after the collection so a folder of them
/// stays readable.
async fn export_collection_markdown(
    State(state): State<AppState>,
    Json(body): Json<CollectionBody>,
) -> Result<Response, ApiError> {
    let (name, content) = export::collection_markdown_document(&state.db, body.collection_id)?;
    let slug = download::slug(&name);
    let filename = if slug.is_empty() {
        format!("command-center-collection-{}.md", body.collection_id)
    } else {
        format!("command-center-{slug}.md")
    };

    Ok(download::attachment(
        content.into_bytes(),
        download::MARKDOWN,
        &filename,
    ))
}

/// A consistent copy of the SQLite library. The copy core makes goes to a
/// temporary file on the same mount, which is then read back and sent. Going
/// through a file is what lets the checkpoint-then-copy in core stay exactly as
/// the desktop app runs it.
async fn backup_library(State(state): State<AppState>) -> Result<Response, ApiError> {
    // Counted rather than fixed, so two backups asked for at once do not read
    // and delete each other's file.
    let sequence = BACKUPS.fetch_add(1, Ordering::Relaxed);
    let temporary = state
        .config
        .data_dir
        .join(format!("backup-{}-{sequence}.tmp.db", std::process::id()));

    let bytes = copy_library(&state, &temporary);
    // Removed whether or not the copy worked, so a failed backup does not
    // leave anything behind on the mount.
    let _ = std::fs::remove_file(&temporary);

    Ok(download::attachment(
        bytes?,
        download::SQLITE,
        "command-center-library.db",
    ))
}

fn copy_library(state: &AppState, temporary: &std::path::Path) -> AppResult<Vec<u8>> {
    export::backup_database(&state.db, &state.config.library_path(), temporary)?;

    std::fs::read(temporary)
        .map_err(|error| AppError::runtime(format!("could not read the backup: {error}")))
}
