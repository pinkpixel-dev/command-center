//! Terminal error analysis, over `core::workflows::diagnose`.

use std::sync::Arc;

use tauri::State;

use crate::ai::diagnosis::ErrorAnalysis;
use crate::ai::providers::codex::service::CodexService;
use crate::ai::AiService;
use crate::db::Database;
use crate::error::AppResult;
use crate::workflows::diagnose::{self, ErrorAnalysisPlan};

#[tauri::command]
pub async fn prepare_error_analysis(
    db: State<'_, Database>,
    output: String,
) -> AppResult<ErrorAnalysisPlan> {
    diagnose::prepare_error_analysis(&db, &output)
}

#[tauri::command]
pub async fn analyze_terminal_error(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    codex: State<'_, Arc<CodexService>>,
    request_id: u64,
    output: String,
) -> AppResult<ErrorAnalysis> {
    diagnose::analyze_terminal_error(&db, &ai, &codex, request_id, output).await
}
