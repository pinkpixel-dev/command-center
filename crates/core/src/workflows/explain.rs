//! Explain, its local cache, and the explicit action that empties that cache.
//!
//! Reading the cache is local work, but a cached explanation is still AI
//! output, so it stays hidden while AI is off. Clearing is deliberately not
//! gated: a removal action has to keep working after the switch is turned off.

use std::sync::Arc;

use serde::Serialize;

use crate::ai::explanation::{self, Explanation, ExplanationRequest};
use crate::ai::providers::{self, codex::service::CodexService};
use crate::ai::AiService;
use crate::db::settings;
use crate::db::{commands as commands_db, explanations, Database};
use crate::error::{AppError, AppResult};
use crate::workflows::require_ai_enabled;

/// A cached or freshly generated explanation, with everything the panel needs
/// to say where it came from and whether it still matches the entry.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationView {
    pub command_id: i64,
    pub explanation: Explanation,
    pub model: String,
    /// The entry changed after this explanation was written.
    pub stale: bool,
    pub generated_at: String,
}

pub fn get_command_explanation(
    db: &Database,
    command_id: i64,
) -> AppResult<Option<ExplanationView>> {
    db.with(|conn| {
        if !settings::load(conn)?.ai_enabled {
            return Ok(None);
        }
        let Some(stored) = explanations::load(conn, command_id)? else {
            return Ok(None);
        };
        view(command_id, stored).map(Some)
    })
}

pub async fn explain_command(
    db: &Database,
    ai: &AiService,
    codex: &Arc<CodexService>,
    command_id: i64,
) -> AppResult<ExplanationView> {
    // The entry and the settings are read and the lock dropped before any
    // await, so the database is never held across the network request.
    let (settings, entry) = db.with(|conn| {
        let settings = settings::load(conn)?;
        require_ai_enabled(&settings)?;
        Ok((settings, commands_db::get(conn, command_id)?))
    })?;

    explanation::check_size(&entry.content)?;

    let (provider, model) = providers::resolve(&settings, ai, codex).await?;

    let generated = explanation::generate(
        &provider,
        &model,
        ExplanationRequest {
            content: &entry.content,
            title: &entry.title,
            description: &entry.description,
            kind: entry.kind,
            shell: entry.shell.as_deref(),
            language: entry.language.as_deref(),
        },
    )
    .await?;

    // The hash of the content that was actually explained. If the user edited
    // the entry while the request was in flight, the cache lands stale, which
    // is the honest result.
    let explained_hash = entry.content_hash.clone();
    let payload = serde_json::to_string(&generated)
        .map_err(|err| AppError::runtime(format!("could not store the explanation: {err}")))?;
    let search_text = generated.search_text();

    let stored = db.with_mut(|conn| {
        explanations::save(
            conn,
            explanations::NewExplanation {
                command_id,
                content_hash: &explained_hash,
                model: &model,
                explanation_json: &payload,
                search_text: &search_text,
            },
        )
    })?;

    view(command_id, stored)
}

/// Empties the explanation cache. Disabling AI hides explanations; this is the
/// action that actually removes them.
pub fn clear_ai_explanations(db: &Database) -> AppResult<usize> {
    db.with_mut(explanations::clear_all)
}

fn view(command_id: i64, stored: explanations::StoredExplanation) -> AppResult<ExplanationView> {
    let explanation: Explanation = serde_json::from_str(&stored.explanation_json)
        .map_err(|_| AppError::AiMalformed("A cached explanation could not be read.".into()))?;

    Ok(ExplanationView {
        command_id,
        explanation,
        model: stored.model,
        stale: stored.stale,
        generated_at: stored.updated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::explanation::{PreviewCommand, SafetyReview};
    use crate::db::settings::AppSettings;
    use crate::models::{CommandInput, RiskLevel};

    fn settings(ai_enabled: bool) -> AppSettings {
        AppSettings {
            ai_enabled,
            ..AppSettings::default()
        }
    }

    fn explanation() -> Explanation {
        Explanation {
            summary: "Removes the build directory.".into(),
            flags: Vec::new(),
            pipeline: Vec::new(),
            side_effects: vec!["Deletes files".into()],
            safety: SafetyReview {
                level: RiskLevel::Caution,
                local_reasons: vec!["Deletes files or directories".into()],
                ai_reasons: Vec::new(),
            },
            preview_command: Some(PreviewCommand {
                command: "ls build".into(),
                risk_level: RiskLevel::Safe,
                risk_reasons: Vec::new(),
            }),
            assumptions: Vec::new(),
            caveats: Vec::new(),
        }
    }

    fn library(ai_enabled: bool) -> (Database, i64) {
        let db = Database::open_in_memory().unwrap();
        let input: CommandInput = serde_json::from_value(serde_json::json!({
            "title": "Clean the build",
            "content": "rm -r build"
        }))
        .unwrap();
        let id = db
            .with_mut(|conn| commands_db::create(conn, input))
            .unwrap()
            .id;

        db.with_mut(|conn| {
            settings::save(conn, settings(ai_enabled))?;
            let entry = commands_db::get(conn, id)?;
            explanations::save(
                conn,
                explanations::NewExplanation {
                    command_id: id,
                    content_hash: &entry.content_hash,
                    model: "gpt-test",
                    explanation_json: &serde_json::to_string(&explanation()).unwrap(),
                    search_text: "removes the build directory",
                },
            )?;
            Ok(())
        })
        .unwrap();

        (db, id)
    }

    #[test]
    fn generating_an_explanation_is_refused_while_ai_is_off() {
        assert_eq!(
            require_ai_enabled(&settings(false)).unwrap_err().kind(),
            "ai_disabled"
        );
        assert!(require_ai_enabled(&settings(true)).is_ok());
    }

    #[test]
    fn a_cached_explanation_is_returned_whole_when_ai_is_on() {
        let (db, id) = library(true);
        let loaded = get_command_explanation(&db, id)
            .unwrap()
            .expect("a cached explanation");

        assert_eq!(loaded.command_id, id);
        assert_eq!(loaded.model, "gpt-test");
        assert!(!loaded.stale);
        assert_eq!(loaded.explanation, explanation());
        assert!(!loaded.generated_at.is_empty());
    }

    #[test]
    fn cached_explanations_stay_hidden_while_ai_is_off() {
        let (db, id) = library(false);
        assert_eq!(get_command_explanation(&db, id).unwrap(), None);

        // Hidden, not deleted: the row is still there for the explicit clear.
        let rows: i64 = db
            .with(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM ai_explanations", [], |row| row.get(0))?)
            })
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn clearing_works_even_after_ai_has_been_turned_off() {
        let (db, id) = library(false);

        assert_eq!(clear_ai_explanations(&db).unwrap(), 1);
        assert_eq!(db.with(|conn| explanations::load(conn, id)).unwrap(), None);
    }

    #[test]
    fn an_edited_entry_returns_its_explanation_marked_stale() {
        let (db, id) = library(true);
        let edited: CommandInput = serde_json::from_value(serde_json::json!({
            "title": "Clean the build",
            "content": "rm -rf build"
        }))
        .unwrap();
        db.with_mut(|conn| commands_db::update(conn, id, edited))
            .unwrap();

        let loaded = get_command_explanation(&db, id).unwrap().unwrap();
        assert!(loaded.stale);
        assert_eq!(loaded.explanation.summary, explanation().summary);
    }

    #[test]
    fn an_entry_with_no_explanation_reports_nothing_rather_than_failing() {
        let (db, _) = library(true);
        let second: CommandInput = serde_json::from_value(serde_json::json!({
            "title": "Status",
            "content": "git status"
        }))
        .unwrap();
        let id = db
            .with_mut(|conn| commands_db::create(conn, second))
            .unwrap()
            .id;

        assert_eq!(get_command_explanation(&db, id).unwrap(), None);
    }

    #[test]
    fn a_corrupted_cache_row_is_reported_rather_than_rendered() {
        let stored = explanations::StoredExplanation {
            explanation_json: "{not json".into(),
            model: "gpt-test".into(),
            stale: false,
            created_at: "2026-07-26T00:00:00Z".into(),
            updated_at: "2026-07-26T00:00:00Z".into(),
        };

        assert_eq!(view(1, stored).unwrap_err().kind(), "ai_malformed");
    }
}
