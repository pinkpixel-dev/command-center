//! Import. The two commands the desktop app answers with a file path are here
//! too, inverted: the client uploads the bytes rather than naming a file the
//! server has no way to reach.

use axum::extract::{Multipart, State};
use axum::routing::post;
use axum::Router;
use serde::Deserialize;

use command_center_core::import::apply::{ImportItem, ImportSummary};
use command_center_core::import::document::ImportDocument;
use command_center_core::import::{self, ImportPreview, SnippetAnalysis};
use command_center_core::library;

use crate::error::{ok, ApiResult};
use crate::json::Json;
use crate::state::AppState;
use crate::upload;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/preview_import_text", post(preview_import_text))
        .route("/analyze_snippet", post(analyze_snippet))
        .route("/import_commands", post(import_commands))
        // The raised body limit is on the two upload routes alone, so an
        // oversized JSON post is still refused at the usual size.
        .route(
            "/read_import_document",
            post(read_import_document).layer(upload::body_limit()),
        )
        .route(
            "/preview_import_file",
            post(preview_import_file).layer(upload::body_limit()),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTextBody {
    content: String,
    #[serde(default)]
    source_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentBody {
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemsBody {
    items: Vec<ImportItem>,
}

async fn preview_import_text(
    State(state): State<AppState>,
    Json(body): Json<PreviewTextBody>,
) -> ApiResult<ImportPreview> {
    ok(state
        .db
        .with(|conn| import::preview(conn, &body.content, body.source_name.as_deref()))?)
}

/// Recalculates one snippet after the user edits, splits, or merges it.
async fn analyze_snippet(
    State(state): State<AppState>,
    Json(body): Json<ContentBody>,
) -> ApiResult<SnippetAnalysis> {
    ok(state.db.with(|conn| import::analyze(conn, &body.content))?)
}

/// The upload the frontend hands to the import screen, read here so the
/// browser never has to parse it.
async fn read_import_document(multipart: Multipart) -> ApiResult<ImportDocument> {
    ok(upload::document(multipart).await?)
}

async fn preview_import_file(
    State(state): State<AppState>,
    multipart: Multipart,
) -> ApiResult<ImportPreview> {
    let file = upload::document(multipart).await?;

    ok(state
        .db
        .with(|conn| import::preview(conn, &file.content, file.name.as_deref()))?)
}

async fn import_commands(
    State(state): State<AppState>,
    Json(body): Json<ItemsBody>,
) -> ApiResult<ImportSummary> {
    ok(library::import_commands(
        &state.db,
        state.events.as_ref(),
        body.items,
    )?)
}
