//! AI-assisted import. The disclosure step is local only; the extraction step
//! is the one place in the app where a user document leaves the machine, and it
//! only runs after the user has seen what will be sent.

use std::sync::Arc;

use tauri::State;

use crate::ai::disclosure::{self, OutboundPlan};
use crate::ai::providers::{self, codex::service::CodexService};
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

    Ok(disclosure::plan(&content, selected_model(&settings)?).1)
}

#[tauri::command]
pub async fn run_ai_import(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    codex: State<'_, Arc<CodexService>>,
    content: String,
    source_name: Option<String>,
) -> AppResult<ImportPreview> {
    // Settings are read and the lock dropped before any await, so the database
    // is never held across the network request.
    let settings = db.with(settings::load)?;
    require_ai_enabled(&settings)?;
    ai_import::check_document_size(&content)?;

    // Resolve first, so the disclosure names the model that will receive the
    // document rather than the other provider's saved choice.
    let (provider, model) = providers::resolve(&settings, &ai, &codex).await?;
    let (redacted, _) = disclosure::plan(&content, &model);

    let items = ai_import::extract(
        &provider,
        &model,
        &redacted,
        source_name.as_deref(),
    )
    .await?;

    db.with(|conn| ai_candidates::build(conn, items, source_name.as_deref()))
}

/// The model a disclosure screen must name.
///
/// Naming the wrong provider's model on the one screen that explains where a
/// document is going would be worse than refusing to show it.
pub(crate) fn selected_model(settings: &AppSettings) -> AppResult<&str> {
    settings
        .selected_model()
        .ok_or_else(|| AppError::ai_model("Choose a Codex model in Settings first."))
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
