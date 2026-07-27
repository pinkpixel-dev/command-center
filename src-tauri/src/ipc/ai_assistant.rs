//! The command assistant. Sending is the explicit action, so there is no
//! separate disclosure step here, but the conversation never touches SQLite:
//! the panel holds it in memory and hands back what it wants resent.
//!
//! The network work runs in its own task so Cancel can actually abort it.

use serde::Deserialize;
use tauri::async_runtime::{channel, spawn, spawn_blocking};
use tauri::State;

use crate::ai::assistant::{self, AssistantReply, AssistantRequest, EntryContext, Subject, Turn};
use crate::ai::{diagnosis, AiService};
use crate::db::settings::{self, AppSettings};
use crate::db::{commands as commands_db, Database};
use crate::error::{AppError, AppResult};
use crate::models::Command;

/// One question, plus whatever the panel wants the model to remember.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantAsk {
    /// Identifies this request so Cancel has something to name.
    pub request_id: u64,
    /// The saved entry the conversation is about, read here rather than sent
    /// up by the panel, so the model only ever sees what the library holds.
    #[serde(default)]
    pub command_id: Option<i64>,
    /// Terminal output an analysis already ran on. It is the one piece of
    /// context the panel does send up, because a paste is never stored: it
    /// lives in memory for as long as that conversation does.
    #[serde(default)]
    pub error_output: Option<String>,
    #[serde(default)]
    pub turns: Vec<Turn>,
    pub message: String,
}

#[tauri::command]
pub async fn ask_assistant(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    request: AssistantAsk,
) -> AppResult<AssistantReply> {
    // Settings and the entry are read and the lock dropped before any await, so
    // the database is never held across the network request.
    let (settings, entry) = db.with(|conn| {
        let settings = settings::load(conn)?;
        require_ai_enabled(&settings)?;
        let entry = match request.command_id {
            Some(id) => Some(commands_db::get(conn, id)?),
            None => None,
        };
        Ok((settings, entry))
    })?;

    assistant::check_message(&request.message)?;
    if let Some(entry) = &entry {
        assistant::check_entry(&entry.content)?;
    }
    if let Some(output) = &request.error_output {
        diagnosis::check_output_size(output)?;
    }

    let model = settings.effective_ai_model().to_owned();
    let credentials = ai.credentials.clone();
    let api_key = spawn_blocking(move || credentials.load())
        .await
        .map_err(|_| AppError::credential("The operating system credential manager stopped."))??
        .ok_or(AppError::AiNotConfigured)?;

    let client = ai.client.clone();
    let AssistantAsk {
        request_id,
        error_output,
        turns,
        message,
        ..
    } = request;

    let (answer, mut answers) = channel::<AppResult<AssistantReply>>(1);
    let handle = spawn(async move {
        let result = assistant::ask(
            &client,
            &api_key,
            &model,
            AssistantRequest {
                subject: subject(entry.as_ref(), error_output.as_deref()),
                turns: &turns,
                message: &message,
            },
        )
        .await;
        // The receiver is gone only when this request was already abandoned.
        let _ = answer.send(result).await;
    });

    ai.inflight.register(request_id, handle)?;
    let received = answers.recv().await;
    ai.inflight.finish(request_id)?;

    // No answer means the task was aborted, which only Cancel does.
    received.unwrap_or(Err(AppError::AiCancelled))
}

/// Aborts a request that is still on the wire. Deliberately not gated on the AI
/// switch: stopping something already running has to keep working.
#[tauri::command]
pub async fn cancel_assistant_request(
    ai: State<'_, AiService>,
    request_id: u64,
) -> AppResult<bool> {
    ai.inflight.cancel(request_id)
}

/// A conversation is about one thing. An entry wins over a paste if both
/// somehow arrive, because the entry came from the library and the paste did
/// not.
fn subject<'a>(entry: Option<&'a Command>, error_output: Option<&'a str>) -> Subject<'a> {
    match (entry, error_output) {
        (Some(entry), _) => Subject::Entry(entry_context(entry)),
        (None, Some(output)) => Subject::TerminalError(output),
        (None, None) => Subject::General,
    }
}

fn entry_context(entry: &Command) -> EntryContext<'_> {
    EntryContext {
        title: &entry.title,
        content: &entry.content,
        kind: entry.kind,
        shell: entry.shell.as_deref(),
        language: entry.language.as_deref(),
    }
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
    use crate::models::{CommandInput, CommandKind};

    fn settings(ai_enabled: bool) -> AppSettings {
        AppSettings {
            ai_enabled,
            ..AppSettings::default()
        }
    }

    fn library(ai_enabled: bool) -> (Database, i64) {
        let db = Database::open_in_memory().unwrap();
        let input: CommandInput = serde_json::from_value(serde_json::json!({
            "title": "Clean the build",
            "content": "rm -r build",
            "shell": "bash"
        }))
        .unwrap();

        let id = db.with_mut(|conn| commands_db::create(conn, input)).unwrap().id;
        db.with_mut(|conn| settings::save(conn, settings(ai_enabled))).unwrap();
        (db, id)
    }

    /// The same read the command performs before it releases the lock.
    fn prepare(db: &Database, command_id: Option<i64>) -> AppResult<Option<Command>> {
        db.with(|conn| {
            require_ai_enabled(&settings::load(conn)?)?;
            match command_id {
                Some(id) => Ok(Some(commands_db::get(conn, id)?)),
                None => Ok(None),
            }
        })
    }

    #[test]
    fn asking_is_refused_while_ai_is_off() {
        let (db, id) = library(false);

        assert_eq!(prepare(&db, Some(id)).unwrap_err().kind(), "ai_disabled");
        assert_eq!(prepare(&db, None).unwrap_err().kind(), "ai_disabled");
    }

    #[test]
    fn a_general_question_reads_no_entry_at_all() {
        let (db, _) = library(true);
        assert!(prepare(&db, None).unwrap().is_none());
    }

    /// The panel sends an id, not content, so the model can only ever see what
    /// the library actually holds for that entry.
    #[test]
    fn entry_context_comes_from_the_library_row() {
        let (db, id) = library(true);
        let entry = prepare(&db, Some(id)).unwrap().unwrap();
        let context = entry_context(&entry);

        assert_eq!(context.title, "Clean the build");
        assert_eq!(context.content, "rm -r build");
        assert_eq!(context.kind, CommandKind::Command);
        assert_eq!(context.shell, Some("bash"));
        assert_eq!(context.language, None);
    }

    #[test]
    fn a_question_about_a_deleted_entry_fails_before_any_request() {
        let (db, id) = library(true);
        db.with_mut(|conn| commands_db::delete(conn, id)).unwrap();

        assert_eq!(prepare(&db, Some(id)).unwrap_err().kind(), "not_found");
    }

    #[test]
    fn cancelling_a_request_nobody_is_running_is_not_a_failure() {
        let inflight = crate::ai::InFlight::default();
        assert!(!inflight.cancel(1).unwrap());
    }
}
