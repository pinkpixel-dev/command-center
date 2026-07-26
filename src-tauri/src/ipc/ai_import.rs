//! AI-assisted import. The disclosure step is local only; the extraction step
//! is the one place in the app where a user document leaves the machine, and it
//! only runs after the user has seen what will be sent.

use serde::Serialize;
use tauri::State;

use crate::ai::redaction::{self, RedactionFinding};
use crate::ai::{import as ai_import, AiService};
use crate::db::settings::{self, AppSettings};
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::import::{ai_candidates, ImportPreview};

/// What the user is told before anything is sent.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiImportPlan {
    /// Size of the document as it sits on this machine.
    pub document_bytes: usize,
    /// Size of the redacted text that would actually be sent.
    pub sent_bytes: usize,
    pub line_count: usize,
    pub model: String,
    pub findings: Vec<PlannedRedaction>,
}

/// A likely secret, described by where it is rather than by what it says.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlannedRedaction {
    pub kind: &'static str,
    pub placeholder: &'static str,
    pub line: usize,
}

#[tauri::command]
pub async fn prepare_ai_import(db: State<'_, Database>, content: String) -> AppResult<AiImportPlan> {
    let settings = db.with(settings::load)?;
    require_ai_enabled(&settings)?;
    ai_import::check_document_size(&content)?;

    Ok(plan(&content, settings.effective_ai_model()).1)
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
    let (redacted, _) = plan(&content, &model);

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

/// Redacts once and describes the result. Both callers use this, so what the
/// user is shown and what is sent are produced by the same code.
fn plan(content: &str, model: &str) -> (String, AiImportPlan) {
    let result = redaction::redact(content);
    let starts = line_starts(content);

    let findings = result
        .findings
        .iter()
        .map(|finding| PlannedRedaction {
            kind: finding.kind,
            placeholder: finding.placeholder,
            line: line_of(&starts, finding),
        })
        .collect();

    let plan = AiImportPlan {
        document_bytes: content.len(),
        sent_bytes: result.redacted_text.len(),
        line_count: content.lines().count(),
        model: model.to_owned(),
        findings,
    };
    (result.redacted_text, plan)
}

fn line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        content
            .match_indices('\n')
            .map(|(index, _)| index.saturating_add(1)),
    );
    starts
}

fn line_of(starts: &[usize], finding: &RedactionFinding) -> usize {
    match starts.binary_search(&finding.start) {
        Ok(index) => index + 1,
        Err(index) => index,
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

    #[test]
    fn the_plan_reports_locations_and_never_the_secret_itself() {
        let secret = ["sk-", "abcdefghijklmnopqrstuvwxyz012345"].concat();
        let document = format!("# Notes\n\ngit status\nexport OPENAI_API_KEY={secret}\npassword=hunter2\n");

        let (redacted, plan) = plan(&document, "gpt-test");
        let serialized = serde_json::to_string(&plan).unwrap();

        assert_eq!(plan.document_bytes, document.len());
        assert_eq!(plan.sent_bytes, redacted.len());
        assert_eq!(plan.line_count, 5);
        assert_eq!(plan.model, "gpt-test");
        assert_eq!(
            plan.findings,
            vec![
                PlannedRedaction {
                    kind: "openai_api_key",
                    placeholder: "{{OPENAI_API_KEY}}",
                    line: 4,
                },
                PlannedRedaction {
                    kind: "password",
                    placeholder: "{{PASSWORD}}",
                    line: 5,
                },
            ]
        );
        for raw in [secret.as_str(), "hunter2"] {
            assert!(!serialized.contains(raw));
            assert!(!redacted.contains(raw));
        }
    }

    #[test]
    fn a_clean_document_reports_nothing_to_review() {
        let (redacted, plan) = plan("git status\ndocker ps\n", "gpt-test");

        assert!(plan.findings.is_empty());
        assert_eq!(redacted, "git status\ndocker ps\n");
        assert_eq!(plan.sent_bytes, plan.document_bytes);
    }

    #[test]
    fn line_numbers_hold_up_on_the_first_and_last_line() {
        let document = "password=first\ngit status\ntoken=last-value-here";
        let (_, plan) = plan(document, "gpt-test");

        assert_eq!(plan.findings[0].line, 1);
        assert_eq!(plan.findings[1].line, 3);
    }
}
