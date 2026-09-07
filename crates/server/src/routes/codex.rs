//! Codex account state and sign-in.
//!
//! Sign-in is the one command that behaves differently here. The desktop app
//! can hand an authorization URL to the system opener; a container has no
//! browser to open, so the browser flow is refused with the reason rather than
//! quietly swapped for the other one.

use axum::extract::State;
use axum::routing::post;
use axum::Router;
use serde::Deserialize;

use command_center_core::ai::providers::codex::auth::{LoginMode, LoginPrompt, LoginStart};
use command_center_core::ai::providers::codex::models::CodexModel;
use command_center_core::error::AppError;
use command_center_core::workflows::codex::{self, CodexStatus};

use crate::error::{ok, ApiError, ApiResult};
use crate::json::Json;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/get_codex_status", post(get_codex_status))
        .route("/refresh_codex", post(refresh_codex))
        .route("/start_codex_login", post(start_codex_login))
        .route("/await_codex_login", post(await_codex_login))
        .route("/cancel_codex_login", post(cancel_codex_login))
        .route("/disconnect_codex", post(disconnect_codex))
        .route("/list_codex_models", post(list_codex_models))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginBody {
    use_device_code: bool,
}

async fn get_codex_status(State(state): State<AppState>) -> ApiResult<CodexStatus> {
    ok(codex::get_codex_status(&state.db, &state.codex).await?)
}

async fn refresh_codex(State(state): State<AppState>) -> ApiResult<CodexStatus> {
    ok(codex::refresh_codex(&state.db, &state.codex).await?)
}

async fn start_codex_login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> ApiResult<LoginPrompt> {
    if !body.use_device_code {
        return Err(ApiError::Core(AppError::ai_auth(
            "This server has no browser to open. Use the sign-in code option instead.",
        )));
    }

    let start = codex::start_codex_login(&state.db, &state.codex, LoginMode::DeviceCode).await?;

    // Device code is the only mode this server starts, so a browser URL here
    // would mean Codex ignored the request. Abandoning it is safer than
    // returning a URL nobody asked for.
    if matches!(start, LoginStart::Browser { .. }) {
        let _ = codex::cancel_codex_login(&state.codex).await;
        return Err(ApiError::Core(AppError::ai_auth(
            "Codex started a browser sign-in this server cannot complete. Update Codex and try again.",
        )));
    }

    ok(LoginPrompt::from(&start))
}

async fn await_codex_login(State(state): State<AppState>) -> ApiResult<CodexStatus> {
    ok(codex::await_codex_login(&state.db, &state.codex).await?)
}

async fn cancel_codex_login(State(state): State<AppState>) -> ApiResult<()> {
    ok(codex::cancel_codex_login(&state.codex).await?)
}

async fn disconnect_codex(State(state): State<AppState>) -> ApiResult<CodexStatus> {
    ok(codex::disconnect_codex(&state.db, &state.codex).await?)
}

async fn list_codex_models(State(state): State<AppState>) -> ApiResult<Vec<CodexModel>> {
    ok(codex::list_codex_models(&state.db, &state.codex).await?)
}
