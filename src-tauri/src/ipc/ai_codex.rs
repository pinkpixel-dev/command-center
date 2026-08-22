//! Settings-facing Codex commands.
//!
//! The frontend never receives a process handle, a raw JSON-RPC message, an
//! authorization URL, or a credential. It gets a status payload it can render
//! and nothing else.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::ai::providers::codex::account::{parse_account, CodexAccount};
use crate::ai::providers::codex::auth::{LoginMode, LoginPrompt, LoginStart};
use crate::ai::providers::codex::models::CodexModel;
use crate::ai::providers::codex::service::{CodexAvailability, CodexService};
use crate::ai::providers::AiProvider;
use crate::db::{settings, Database};
use crate::error::{AppError, AppResult};

/// Everything the ChatGPT panel needs to render one of its states.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatus {
    /// Whether Codex itself is present and usable.
    pub availability: CodexAvailability,
    /// The account in Command Center's own Codex home. Only meaningful when
    /// `availability` is ready.
    pub account: CodexAccount,
    /// Set when Codex is usable but the account could not be read.
    pub account_error: Option<String>,
    /// Bounded, sanitized lines for a details panel.
    pub diagnostics: Vec<String>,
}

#[tauri::command]
pub async fn get_codex_status(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<CodexStatus> {
    let settings = db.with(settings::load)?;
    let saved_path = settings.codex_path.clone();

    let availability = codex.availability(saved_path.as_deref()).await;

    // Only reach for an account when there is a Codex that could hold one.
    // Anything else would start a process just to fail.
    let (account, account_error) = match availability {
        CodexAvailability::Ready { .. } => match codex.read_account(saved_path.as_deref()).await {
            Ok(value) => (parse_account(&value), None),
            Err(error) => (CodexAccount::NotConnected, Some(error.to_string())),
        },
        _ => (CodexAccount::NotConnected, None),
    };

    Ok(CodexStatus {
        availability,
        account,
        account_error,
        diagnostics: codex.diagnostics().await,
    })
}

/// Drops the cached discovery result and any running process, then reports the
/// fresh state. This is the Retry action, and the path-changed action.
#[tauri::command]
pub async fn refresh_codex(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<CodexStatus> {
    codex.refresh().await;
    get_codex_status(db, codex).await
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
    let settings = db.with(settings::load)?;
    let mode = if use_device_code {
        LoginMode::DeviceCode
    } else {
        LoginMode::Browser
    };

    let start = codex.start_login(settings.codex_path.as_deref(), mode).await?;
    let prompt = LoginPrompt::from(&start);

    if let LoginStart::Browser { auth_url, .. } = &start {
        if let Err(error) = app.opener().open_url(auth_url.as_str(), None::<&str>) {
            // Leaving the attempt open would keep Codex's callback listener
            // running for a sign-in the user cannot reach.
            let _ = codex.cancel_login().await;
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
    codex.await_login().await?;
    get_codex_status(db, codex).await
}

/// Abandons an in-flight sign-in.
#[tauri::command]
pub async fn cancel_codex_login(codex: State<'_, CodexService>) -> AppResult<()> {
    codex.cancel_login().await
}

/// Disconnects the account from Command Center only. The user's own Codex CLI
/// login is in a different Codex home and is not touched.
#[tauri::command]
pub async fn disconnect_codex(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<CodexStatus> {
    let settings = db.with(settings::load)?;
    codex.logout(settings.codex_path.as_deref()).await?;
    get_codex_status(db, codex).await
}

/// The live model catalogue for the connected account.
#[tauri::command]
pub async fn list_codex_models(
    db: State<'_, Database>,
    codex: State<'_, Arc<CodexService>>,
) -> AppResult<Vec<CodexModel>> {
    let settings = db.with(settings::load)?;
    codex.list_models(settings.codex_path.as_deref()).await
}

/// Whether the selected provider is ready to run a workflow.
///
/// Rust owns this answer so the frontend cannot enable an AI action by
/// disagreeing with it.
pub fn provider_is_ready(
    settings: &settings::AppSettings,
    key_stored: bool,
    codex_account: &CodexAccount,
    codex_available: bool,
) -> bool {
    if !settings.ai_enabled {
        return false;
    }
    let provider = settings.provider();
    if !provider.is_available_on_this_platform() {
        return false;
    }

    match provider {
        AiProvider::OpenaiApi => key_stored,
        AiProvider::ChatgptCodex => {
            codex_available
                && matches!(codex_account, CodexAccount::Connected { .. })
                // A saved model is required. There is no invented default,
                // because the Codex catalogue is discovered, not curated.
                && settings.codex_model.is_some()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::settings::AppSettings;

    fn connected() -> CodexAccount {
        CodexAccount::Connected {
            email: Some("person@example.com".into()),
            plan: Some("plus".into()),
        }
    }

    fn codex_settings() -> AppSettings {
        AppSettings {
            ai_enabled: true,
            ai_provider: AiProvider::ChatgptCodex.as_str().into(),
            codex_model: Some("gpt-5.6-luna".into()),
            ..AppSettings::default()
        }
    }

    #[test]
    fn ai_off_is_never_ready_for_either_provider() {
        let mut settings = codex_settings();
        settings.ai_enabled = false;

        assert!(!provider_is_ready(&settings, true, &connected(), true));

        settings.ai_provider = AiProvider::OpenaiApi.as_str().into();
        assert!(!provider_is_ready(&settings, true, &connected(), true));
    }

    #[test]
    fn the_api_key_provider_only_needs_a_stored_key() {
        let settings = AppSettings {
            ai_enabled: true,
            ai_provider: AiProvider::OpenaiApi.as_str().into(),
            ..AppSettings::default()
        };

        assert!(provider_is_ready(&settings, true, &CodexAccount::NotConnected, false));
        assert!(!provider_is_ready(&settings, false, &connected(), true));
    }

    #[test]
    fn codex_needs_an_installation_an_account_and_a_model() {
        let settings = codex_settings();

        assert!(provider_is_ready(&settings, false, &connected(), true));
        // Missing Codex.
        assert!(!provider_is_ready(&settings, false, &connected(), false));
        // Missing account.
        assert!(!provider_is_ready(
            &settings,
            false,
            &CodexAccount::NotConnected,
            true
        ));
    }

    #[test]
    fn codex_without_a_chosen_model_is_not_ready() {
        let settings = AppSettings {
            codex_model: None,
            ..codex_settings()
        };

        // There is no curated Codex default to fall back to, so a missing
        // selection has to block rather than guess.
        assert!(!provider_is_ready(&settings, false, &connected(), true));
    }

    #[test]
    fn a_stored_api_key_does_not_make_codex_ready() {
        let settings = codex_settings();

        // The two providers are never interchangeable.
        assert!(!provider_is_ready(
            &settings,
            true,
            &CodexAccount::NotConnected,
            true
        ));
    }

    #[test]
    fn an_account_authenticated_some_other_way_is_not_a_chatgpt_connection() {
        let settings = codex_settings();
        let other = CodexAccount::ConnectedWithOtherCredentials {
            kind: "apiKey".into(),
        };

        assert!(!provider_is_ready(&settings, false, &other, true));
    }

    #[test]
    fn the_status_payload_carries_no_credential_shaped_field() {
        let status = CodexStatus {
            availability: CodexAvailability::Ready {
                version: "0.147.0".into(),
            },
            account: connected(),
            account_error: None,
            diagnostics: vec!["Declined a tool request.".into()],
        };
        let encoded = serde_json::to_string(&status).unwrap();

        for forbidden in ["token", "apiKey", "secret", "authUrl", "codexHome"] {
            assert!(
                !encoded.contains(forbidden),
                "status leaked {forbidden}: {encoded}",
            );
        }
    }
}
