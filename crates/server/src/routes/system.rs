//! Settings, plus the one piece of app metadata the UI shows in About.
//!
//! Exporting and backing up are missing on purpose: those write to a path the
//! caller names, which only makes sense when the caller is on the same machine.
//! On a server they become downloads, which is its own phase.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use command_center_core::db::settings::{self, AppSettings};
use command_center_core::library;

use crate::error::{ok, ApiResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/get_settings", post(get_settings))
        .route("/save_settings", post(save_settings))
        .route("/library_location", post(library_location))
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
