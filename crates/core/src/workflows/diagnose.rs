//! Terminal error analysis. Pasted output is content the user did not write,
//! so it takes the same two-step path Import does: a local disclosure first,
//! then one request that sends exactly the text the disclosure described.
//!
//! The network work runs in its own task so Stop can actually abort it.

use std::sync::Arc;

use tokio::spawn;
use tokio::sync::mpsc::channel;

use crate::ai::diagnosis::{self, ErrorAnalysis};
use crate::ai::disclosure::{self, OutboundPlan};
use crate::ai::providers::{self, codex::service::CodexService};
use crate::ai::AiService;
use crate::db::settings;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::workflows::{require_ai_enabled, selected_model};

/// What the user is told before their terminal output is sent.
pub type ErrorAnalysisPlan = OutboundPlan;

pub fn prepare_error_analysis(db: &Database, output: &str) -> AppResult<ErrorAnalysisPlan> {
    let settings = db.with(settings::load)?;
    require_ai_enabled(&settings)?;
    diagnosis::check_output_size(output)?;

    Ok(disclosure::plan(output, selected_model(&settings)?).1)
}

pub async fn analyze_terminal_error(
    db: &Database,
    ai: &AiService,
    codex: &Arc<CodexService>,
    request_id: u64,
    output: String,
) -> AppResult<ErrorAnalysis> {
    // Settings are read and the lock dropped before any await, so the database
    // is never held across the network request.
    let settings = db.with(settings::load)?;
    require_ai_enabled(&settings)?;
    diagnosis::check_output_size(&output)?;

    // Resolve first: the disclosure has to name the model that will actually
    // receive the text, not whichever provider happens to have one saved.
    let (provider, model) = providers::resolve(&settings, ai, codex).await?;
    // The same call the disclosure made, so what leaves the machine is the text
    // the user was shown rather than a second redaction of the same paste.
    let (redacted, _) = disclosure::plan(&output, &model);
    let (answer, mut answers) = channel::<AppResult<ErrorAnalysis>>(1);
    let handle = spawn(async move {
        let result = diagnosis::analyze(&provider, &model, &redacted).await;
        // The receiver is gone only when this request was already abandoned.
        let _ = answer.send(result).await;
    });

    ai.inflight.register(request_id, handle)?;
    let received = answers.recv().await;
    ai.inflight.finish(request_id)?;

    // No answer means the task was aborted, which only Stop does.
    received.unwrap_or(Err(AppError::AiCancelled))
}

#[cfg(test)]
mod tests {
    use super::*;

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
