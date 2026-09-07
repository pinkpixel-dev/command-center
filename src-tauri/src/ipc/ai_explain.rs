//! Explain and its cache, over `core::workflows::explain`.

use std::sync::Arc;

use tauri::State;

use crate::ai::providers::codex::service::CodexService;
use crate::ai::AiService;
use crate::db::Database;
use crate::error::AppResult;
use crate::workflows::explain::{self, ExplanationView};

#[tauri::command]
pub async fn get_command_explanation(
    db: State<'_, Database>,
    command_id: i64,
) -> AppResult<Option<ExplanationView>> {
    explain::get_command_explanation(&db, command_id)
}

#[tauri::command]
pub async fn explain_command(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    codex: State<'_, Arc<CodexService>>,
    command_id: i64,
) -> AppResult<ExplanationView> {
    explain::explain_command(&db, &ai, &codex, command_id).await
}

/// Empties the explanation cache. Disabling AI hides explanations; this is the
/// action that actually removes them.
#[tauri::command]
pub async fn clear_ai_explanations(db: State<'_, Database>) -> AppResult<usize> {
    explain::clear_ai_explanations(&db)
}
