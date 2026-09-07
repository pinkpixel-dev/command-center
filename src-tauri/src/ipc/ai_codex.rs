//! Settings-facing Codex commands, over `core::workflows::codex`.
//!
//! Sign-in is the one place the desktop app adds something of its own: the
//! browser flow's authorization URL goes straight from the core to the system
//! opener, so it never reaches the frontend or a log.

use std::sync::Arc;

use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::ai::providers::codex::auth::{LoginMode, LoginPrompt, LoginStart};
use crate::ai::providers::codex::models::CodexModel;
use crate::ai::providers::codex::service::CodexService;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::workflows::codex::{self, CodexStatus};

#[tauri::command]
pub async fn get_codex_status(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<CodexStatus> {
    codex::get_codex_status(&db, &codex).await
}

/// Drops the cached discovery result and any running process, then reports the
/// fresh state. This is the Retry action, and the path-changed action.
#[tauri::command]
pub async fn refresh_codex(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<CodexStatus> {
    codex::refresh_codex(&db, &codex).await
}

/// Starts a ChatGPT sign-in.
///
/// For the browser flow the authorization URL is handed straight to the
/// system opener and never returned, so it cannot reach the frontend or a log.
#[tauri::command]
pub async fn start_codex_login(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
    use_device_code: bool,
) -> AppResult<LoginPrompt> {
    let mode = if use_device_code {
        LoginMode::DeviceCode
    } else {
        LoginMode::Browser
    };

    let start = codex::start_codex_login(&db, &codex, mode).await?;
    let prompt = LoginPrompt::from(&start);

    if let LoginStart::Browser { auth_url, .. } = &start {
        if let Err(error) = app.opener().open_url(auth_url.as_str(), None::<&str>) {
            // Leaving the attempt open would keep Codex's callback listener
            // running for a sign-in the user cannot reach.
            let _ = codex::cancel_codex_login(&codex).await;
            let _ = error;
            return Err(AppError::ai_auth(
                "The browser could not be opened. Use the sign-in code option instead.",
            ));
        }
    }

    Ok(prompt)
}

/// Waits for the sign-in that `start_codex_login` began, then reports the
/// refreshed status.
#[tauri::command]
pub async fn await_codex_login(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<CodexStatus> {
    codex::await_codex_login(&db, &codex).await
}

/// Abandons an in-flight sign-in.
#[tauri::command]
pub async fn cancel_codex_login(codex: State<'_, Arc<CodexService>>) -> AppResult<()> {
    codex::cancel_codex_login(&codex).await
}

/// Disconnects the account from Command Center only. The user's own Codex CLI
/// login is in a different Codex home and is not touched.
#[tauri::command]
pub async fn disconnect_codex(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<CodexStatus> {
    codex::disconnect_codex(&db, &codex).await
}

/// The live model catalogue for the connected account.
#[tauri::command]
pub async fn list_codex_models(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<Vec<CodexModel>> {
    codex::list_codex_models(&db, &codex).await
}
