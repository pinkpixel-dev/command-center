//! Import. The two commands that read a path off the local disk are not here:
//! on a server the client uploads bytes instead, which is its own phase.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use command_center_core::import::apply::{ImportItem, ImportSummary};
use command_center_core::import::{self, ImportPreview, SnippetAnalysis};
use command_center_core::library;

use crate::error::{ok, ApiResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/preview_import_text", post(preview_import_text))
        .route("/analyze_snippet", post(analyze_snippet))
        .route("/import_commands", post(import_commands))
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
