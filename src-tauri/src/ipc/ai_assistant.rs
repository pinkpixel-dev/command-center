//! The command assistant, over `core::workflows::assistant`.

use std::sync::Arc;

use tauri::State;

use crate::ai::assistant::AssistantReply;
use crate::ai::providers::codex::service::CodexService;
use crate::ai::AiService;
use crate::db::Database;
use crate::error::AppResult;
use crate::workflows::assistant::{self, AssistantAsk};

#[tauri::command]
pub async fn ask_assistant(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    codex: State<'_, Arc<CodexService>>,
    request: AssistantAsk,
) -> AppResult<AssistantReply> {
    assistant::ask_assistant(&db, &ai, &codex, request).await
}

/// Aborts a request that is still on the wire. Deliberately not gated on the AI
/// switch: stopping something already running has to keep working.
#[tauri::command]
pub async fn cancel_assistant_request(
    ai: State<'_, AiService>,
    request_id: u64,
) -> AppResult<bool> {
    assistant::cancel_assistant_request(&ai, request_id)
}
