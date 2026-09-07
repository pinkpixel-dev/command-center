//! The AI workflows. Every one of these is a thin call into
//! `core::workflows`, which is the same code the desktop app runs.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use command_center_core::ai::assistant::AssistantReply;
use command_center_core::ai::conversion::ShellConversion;
use command_center_core::ai::diagnosis::ErrorAnalysis;
use command_center_core::ai::AiConnectionResult;
use command_center_core::import::ImportPreview;
use command_center_core::workflows::assistant::{self, AssistantAsk};
use command_center_core::workflows::convert::{self, ShellOption};
use command_center_core::workflows::diagnose::{self, ErrorAnalysisPlan};
use command_center_core::workflows::explain::{self, ExplanationView};
use command_center_core::workflows::import::{self as ai_import, AiImportPlan};
use command_center_core::workflows::status::{self, AiKeyStatus, AiStatus};

use crate::error::{ok, ApiResult};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/get_ai_status", post(get_ai_status))
        .route("/save_ai_key", post(save_ai_key))
        .route("/remove_ai_key", post(remove_ai_key))
        .route("/test_ai_connection", post(test_ai_connection))
        .route("/prepare_ai_import", post(prepare_ai_import))
        .route("/run_ai_import", post(run_ai_import))
        .route("/ask_assistant", post(ask_assistant))
        .route("/cancel_assistant_request", post(cancel_assistant_request))
        .route("/get_command_explanation", post(get_command_explanation))
        .route("/explain_command", post(explain_command))
        .route("/clear_ai_explanations", post(clear_ai_explanations))
        .route("/prepare_error_analysis", post(prepare_error_analysis))
        .route("/analyze_terminal_error", post(analyze_terminal_error))
        .route("/conversion_shells", post(conversion_shells))
        .route("/convert_command_shell", post(convert_command_shell))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyBody {
    api_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentBody {
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiImportBody {
    content: String,
    #[serde(default)]
    source_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantBody {
    request: AssistantAsk,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestIdBody {
    request_id: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandIdBody {
    command_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputBody {
    output: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeBody {
    request_id: u64,
    output: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertBody {
    request_id: u64,
    command_id: i64,
    target_shell: String,
}

async fn get_ai_status(State(state): State<AppState>) -> ApiResult<AiStatus> {
    ok(status::get_ai_status(&state.db, &state.ai).await?)
}

async fn save_ai_key(
    State(state): State<AppState>,
    Json(body): Json<ApiKeyBody>,
) -> ApiResult<AiKeyStatus> {
    ok(status::save_ai_key(&state.ai, body.api_key).await?)
}

async fn remove_ai_key(State(state): State<AppState>) -> ApiResult<AiKeyStatus> {
    ok(status::remove_ai_key(&state.ai).await?)
}

async fn test_ai_connection(State(state): State<AppState>) -> ApiResult<AiConnectionResult> {
    ok(status::test_ai_connection(&state.db, &state.ai, &state.codex).await?)
}

async fn prepare_ai_import(
    State(state): State<AppState>,
    Json(body): Json<ContentBody>,
) -> ApiResult<AiImportPlan> {
    ok(ai_import::prepare_ai_import(&state.db, &body.content)?)
}

async fn run_ai_import(
    State(state): State<AppState>,
    Json(body): Json<AiImportBody>,
) -> ApiResult<ImportPreview> {
    ok(ai_import::run_ai_import(
        &state.db,
        &state.ai,
        &state.codex,
        body.content,
        body.source_name,
    )
    .await?)
}

async fn ask_assistant(
    State(state): State<AppState>,
    Json(body): Json<AssistantBody>,
) -> ApiResult<AssistantReply> {
    ok(assistant::ask_assistant(&state.db, &state.ai, &state.codex, body.request).await?)
}

async fn cancel_assistant_request(
    State(state): State<AppState>,
    Json(body): Json<RequestIdBody>,
) -> ApiResult<bool> {
    ok(assistant::cancel_assistant_request(
        &state.ai,
        body.request_id,
    )?)
}

async fn get_command_explanation(
    State(state): State<AppState>,
    Json(body): Json<CommandIdBody>,
) -> ApiResult<Option<ExplanationView>> {
    ok(explain::get_command_explanation(&state.db, body.command_id)?)
}

async fn explain_command(
    State(state): State<AppState>,
    Json(body): Json<CommandIdBody>,
) -> ApiResult<ExplanationView> {
    ok(explain::explain_command(&state.db, &state.ai, &state.codex, body.command_id).await?)
}

async fn clear_ai_explanations(State(state): State<AppState>) -> ApiResult<usize> {
    ok(explain::clear_ai_explanations(&state.db)?)
}

async fn prepare_error_analysis(
    State(state): State<AppState>,
    Json(body): Json<OutputBody>,
) -> ApiResult<ErrorAnalysisPlan> {
    ok(diagnose::prepare_error_analysis(&state.db, &body.output)?)
}

async fn analyze_terminal_error(
    State(state): State<AppState>,
    Json(body): Json<AnalyzeBody>,
) -> ApiResult<ErrorAnalysis> {
    ok(diagnose::analyze_terminal_error(
        &state.db,
        &state.ai,
        &state.codex,
        body.request_id,
        body.output,
    )
    .await?)
}

async fn conversion_shells() -> ApiResult<Vec<ShellOption>> {
    ok(convert::conversion_shells())
}

async fn convert_command_shell(
    State(state): State<AppState>,
    Json(body): Json<ConvertBody>,
) -> ApiResult<ShellConversion> {
    ok(convert::convert_command_shell(
        &state.db,
        &state.ai,
        &state.codex,
        body.request_id,
        body.command_id,
        &body.target_shell,
    )
    .await?)
}
