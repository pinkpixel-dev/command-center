//! AI-assisted import. The disclosure step is local only; the extraction step
//! is the one place in the app where a user document leaves the machine, and it
//! only runs after the user has seen what will be sent.

use tauri::State;

use crate::ai::disclosure::{self, OutboundPlan};
use crate::ai::{import as ai_import, AiService};
use crate::db::settings::{self, AppSettings};
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::import::{ai_candidates, ImportPreview};

/// What the user is told before a document is sent. Error analysis shows the
/// same thing, so the shape lives in `ai::disclosure`.
pub type AiImportPlan = OutboundPlan;

#[tauri::command]
pub async fn prepare_ai_import(db: State<'_, Database>, content: String) -> AppResult<AiImportPlan> {
    let settings = db.with(settings::load)?;
    require_ai_enabled(&settings)?;
    ai_import::check_document_size(&content)?;

    Ok(disclosure::plan(&content, settings.effective_ai_model()).1)
}

#[tauri::command]
pub async fn run_ai_import(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    content: String,
    source_name: Option<String>,
) -> AppResult<ImportPreview> {
    // Settings are read and the lock dropped before any await, so the database
    // is never held across the network request.
    let settings = db.with(settings::load)?;
    require_ai_enabled(&settings)?;
    ai_import::check_document_size(&content)?;

    let model = settings.effective_ai_model().to_owned();
    let (redacted, _) = disclosure::plan(&content, &model);

    let credentials = ai.credentials.clone();
    let api_key = tauri::async_runtime::spawn_blocking(move || credentials.load())
        .await
        .map_err(|_| AppError::credential("The operating system credential manager stopped."))??
        .ok_or(AppError::AiNotConfigured)?;

    let items = ai_import::extract(
        &ai.client,
        &api_key,
        &model,
        &redacted,
        source_name.as_deref(),
    )
    .await?;

    db.with(|conn| ai_candidates::build(conn, items, source_name.as_deref()))
}

fn require_ai_enabled(settings: &AppSettings) -> AppResult<()> {
    if settings.ai_enabled {
        Ok(())
    } else {
        Err(AppError::AiDisabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(ai_enabled: bool) -> AppSettings {
        AppSettings {
            ai_enabled,
            ..AppSettings::default()
        }
    }

    #[test]
    fn network_backed_import_is_refused_while_ai_is_off() {
        assert_eq!(
            require_ai_enabled(&settings(false)).unwrap_err().kind(),
            "ai_disabled"
        );
        assert!(require_ai_enabled(&settings(true)).is_ok());
    }

    /// The disclosure the user sees and the text that is sent come from one
    /// call, which is what keeps them from drifting apart.
    #[test]
    fn what_is_disclosed_is_what_would_be_sent() {
        let document = "git status\npassword=hunter2\n";
        let (redacted, plan) = disclosure::plan(document, "gpt-test");

        assert_eq!(plan.sent_bytes, redacted.len());
        assert_eq!(plan.findings.len(), 1);
        assert!(!redacted.contains("hunter2"));
    }
}
