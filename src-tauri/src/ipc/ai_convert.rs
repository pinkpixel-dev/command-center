//! Shell conversion, over `core::workflows::convert`.

use std::sync::Arc;

use tauri::State;

use crate::ai::conversion::ShellConversion;
use crate::ai::providers::codex::service::CodexService;
use crate::ai::AiService;
use crate::db::Database;
use crate::error::AppResult;
use crate::workflows::convert::{self, ShellOption};

#[tauri::command]
pub async fn conversion_shells() -> AppResult<Vec<ShellOption>> {
    Ok(convert::conversion_shells())
}

#[tauri::command]
pub async fn convert_command_shell(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    codex: State<'_, Arc<CodexService>>,
    request_id: u64,
    command_id: i64,
    target_shell: String,
) -> AppResult<ShellConversion> {
    convert::convert_command_shell(&db, &ai, &codex, request_id, command_id, &target_shell).await
}
