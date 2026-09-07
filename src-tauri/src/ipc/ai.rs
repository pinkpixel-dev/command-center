//! Settings-facing AI commands. The work is in `core::workflows::status`; this
//! file only unwraps Tauri's managed state.

use tauri::State;

use crate::ai::providers::codex::service::CodexService;
use crate::ai::{AiConnectionResult, AiService};
use crate::db::Database;
use crate::error::AppResult;
use crate::workflows::status::{self, AiKeyStatus, AiStatus};

#[tauri::command]
pub async fn get_ai_status(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
) -> AppResult<AiStatus> {
    status::get_ai_status(&db, &ai).await
}

#[tauri::command]
pub async fn save_ai_key(ai: State<'_, AiService>, api_key: String) -> AppResult<AiKeyStatus> {
    status::save_ai_key(&ai, api_key).await
}

#[tauri::command]
pub async fn remove_ai_key(ai: State<'_, AiService>) -> AppResult<AiKeyStatus> {
    status::remove_ai_key(&ai).await
}

#[tauri::command]
pub async fn test_ai_connection(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    codex: State<'_, std::sync::Arc<CodexService>>,
) -> AppResult<AiConnectionResult> {
    status::test_ai_connection(&db, &ai, &codex).await
}
