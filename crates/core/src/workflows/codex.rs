//! Codex account state and sign-in, without the part that needs a browser.
//!
//! The frontend never receives a process handle, a raw JSON-RPC message, an
//! authorization URL, or a credential. It gets a status payload it can render
//! and nothing else. Sign-in returns the raw [`LoginStart`] to the shell that
//! called it, because the desktop app hands a browser URL to the system
//! opener and a server has no browser to hand it to.

use std::sync::Arc;

use serde::Serialize;

use crate::ai::providers::codex::account::{parse_account, CodexAccount};
use crate::ai::providers::codex::auth::{LoginMode, LoginStart};
use crate::ai::providers::codex::models::CodexModel;
use crate::ai::providers::codex::service::{CodexAvailability, CodexService};
use crate::ai::providers::AiProvider;
use crate::db::{settings, Database};
use crate::error::AppResult;

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

pub async fn get_codex_status(
    db: &Database,
    codex: &Arc<CodexService>,
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
pub async fn refresh_codex(db: &Database, codex: &Arc<CodexService>) -> AppResult<CodexStatus> {
    codex.refresh().await;
    get_codex_status(db, codex).await
}

/// Starts a ChatGPT sign-in and hands the raw start back to the caller.
///
/// The browser flow's authorization URL must never reach the frontend or a
/// log, so the shell that receives it either gives it straight to the system
/// opener or refuses the mode outright.
pub async fn start_codex_login(
    db: &Database,
    codex: &Arc<CodexService>,
    mode: LoginMode,
) -> AppResult<LoginStart> {
    let settings = db.with(settings::load)?;
    codex.start_login(settings.codex_path.as_deref(), mode).await
}

/// Abandons an in-flight sign-in.
pub async fn cancel_codex_login(codex: &Arc<CodexService>) -> AppResult<()> {
    codex.cancel_login().await
}

/// Waits for the sign-in that `start_codex_login` began, then reports the
/// refreshed status.
pub async fn await_codex_login(
    db: &Database,
    codex: &Arc<CodexService>,
) -> AppResult<CodexStatus> {
    codex.await_login().await?;
    get_codex_status(db, codex).await
}

/// Disconnects the account from Command Center only. The user's own Codex CLI
/// login is in a different Codex home and is not touched.
pub async fn disconnect_codex(
    db: &Database,
    codex: &Arc<CodexService>,
) -> AppResult<CodexStatus> {
    let settings = db.with(settings::load)?;
    codex.logout(settings.codex_path.as_deref()).await?;
    get_codex_status(db, codex).await
}

/// The live model catalogue for the connected account.
pub async fn list_codex_models(
    db: &Database,
    codex: &Arc<CodexService>,
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

        assert!(provider_is_ready(
            &settings,
            true,
            &CodexAccount::NotConnected,
            false
        ));
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
