//! Terminal error analysis. Pasted output is content the user did not write,
//! so it takes the same two-step path Import does: a local disclosure first,
//! then one request that sends exactly the text the disclosure described.
//!
//! The network work runs in its own task so Stop can actually abort it.

use tauri::async_runtime::{channel, spawn, spawn_blocking};
use tauri::State;

use crate::ai::diagnosis::{self, ErrorAnalysis};
use crate::ai::disclosure::{self, OutboundPlan};
use crate::ai::AiService;
use crate::db::settings::{self, AppSettings};
use crate::db::Database;
use crate::error::{AppError, AppResult};

/// What the user is told before their terminal output is sent.
pub type ErrorAnalysisPlan = OutboundPlan;

#[tauri::command]
pub async fn prepare_error_analysis(
    db: State<'_, Database>,
    output: String,
) -> AppResult<ErrorAnalysisPlan> {
    let settings = db.with(settings::load)?;
    require_ai_enabled(&settings)?;
    diagnosis::check_output_size(&output)?;

    Ok(disclosure::plan(&output, settings.effective_ai_model()).1)
}

#[tauri::command]
pub async fn analyze_terminal_error(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    request_id: u64,
    output: String,
) -> AppResult<ErrorAnalysis> {
    // Settings are read and the lock dropped before any await, so the database
    // is never held across the network request.
    let settings = db.with(settings::load)?;
    require_ai_enabled(&settings)?;
    diagnosis::check_output_size(&output)?;

    let model = settings.effective_ai_model().to_owned();
    // The same call the disclosure made, so what leaves the machine is the text
    // the user was shown rather than a second redaction of the same paste.
    let (redacted, _) = disclosure::plan(&output, &model);

    let credentials = ai.credentials.clone();
    let api_key = spawn_blocking(move || credentials.load())
        .await
        .map_err(|_| AppError::credential("The operating system credential manager stopped."))??
        .ok_or(AppError::AiNotConfigured)?;

    let client = ai.client.clone();
    let (answer, mut answers) = channel::<AppResult<ErrorAnalysis>>(1);
    let handle = spawn(async move {
        let result = diagnosis::analyze(&client, &api_key, &model, &redacted).await;
        // The receiver is gone only when this request was already abandoned.
        let _ = answer.send(result).await;
    });

    ai.inflight.register(request_id, handle)?;
    let received = answers.recv().await;
    ai.inflight.finish(request_id)?;

    // No answer means the task was aborted, which only Stop does.
    received.unwrap_or(Err(AppError::AiCancelled))
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
    fn analysis_is_refused_while_ai_is_off() {
        assert_eq!(
            require_ai_enabled(&settings(false)).unwrap_err().kind(),
            "ai_disabled"
        );
        assert!(require_ai_enabled(&settings(true)).is_ok());
    }

    /// Disclosure and the request call the same function on the same paste, so
    /// the text described and the text sent are byte-identical.
    #[test]
    fn the_analyzed_text_is_the_disclosed_text() {
        let output = "Error: auth failed\nAuthorization: Bearer abcdefghijklmnopqrst\n";

        let (disclosed, plan) = disclosure::plan(output, "gpt-test");
        let (sent, _) = disclosure::plan(output, "gpt-test");

        assert_eq!(disclosed, sent);
        assert_eq!(plan.sent_bytes, sent.len());
        assert_eq!(plan.findings.len(), 1);
        assert_eq!(plan.findings[0].kind, "bearer_token");
        assert!(!sent.contains("abcdefghijklmnopqrst"));
    }

    /// An empty or oversized paste fails before a key is ever read.
    #[test]
    fn the_paste_is_checked_before_the_credential_manager_is_touched() {
        assert_eq!(
            diagnosis::check_output_size("  ").unwrap_err().kind(),
            "invalid"
        );

        let oversized = "x".repeat(diagnosis::MAX_OUTPUT_BYTES + 1);
        assert_eq!(
            diagnosis::check_output_size(&oversized).unwrap_err().kind(),
            "invalid"
        );
    }
}
