//! AI-assisted import, over `core::workflows::import`.

use std::sync::Arc;

use tauri::State;

use crate::ai::providers::codex::service::CodexService;
use crate::ai::AiService;
use crate::db::Database;
use crate::error::AppResult;
use crate::import::ImportPreview;
use crate::workflows::import::{self as ai_import, AiImportPlan};

#[tauri::command]
pub async fn prepare_ai_import(db: State<'_, Database>, content: String) -> AppResult<AiImportPlan> {
    ai_import::prepare_ai_import(&db, &content)
}

#[tauri::command]
pub async fn run_ai_import(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    codex: State<'_, Arc<CodexService>>,
    content: String,
    source_name: Option<String>,
) -> AppResult<ImportPreview> {
    ai_import::run_ai_import(&db, &ai, &codex, content, source_name).await
}
