//! Shell conversion. The entry is read from the library rather than sent up by
//! the dialog, so the model only ever sees what the library actually holds.
//!
//! The network work runs in its own task so Stop can actually abort it, and
//! nothing here writes: a conversion the user wants to keep goes through the
//! normal entry form like every other new command.

use serde::Serialize;
use tauri::async_runtime::{channel, spawn, spawn_blocking};
use tauri::State;

use crate::ai::conversion::{self, ConversionRequest, ShellConversion, TargetShell};
use crate::ai::AiService;
use crate::db::settings::{self, AppSettings};
use crate::db::{commands as commands_db, Database};
use crate::error::{AppError, AppResult};

/// Which shells the dialog may offer, named by the backend so the list cannot
/// drift between the picker and what the prompt accepts.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShellOption {
    pub id: &'static str,
    pub label: &'static str,
}

const SHELLS: &[TargetShell] = &[
    TargetShell::Bash,
    TargetShell::Fish,
    TargetShell::Zsh,
    TargetShell::PowerShell,
];

#[tauri::command]
pub async fn conversion_shells() -> AppResult<Vec<ShellOption>> {
    Ok(shell_options())
}

#[tauri::command]
pub async fn convert_command_shell(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    request_id: u64,
    command_id: i64,
    target_shell: String,
) -> AppResult<ShellConversion> {
    let target = TargetShell::parse(&target_shell)?;

    // Settings and the entry are read and the lock dropped before any await, so
    // the database is never held across the network request.
    let (settings, entry) = db.with(|conn| {
        let settings = settings::load(conn)?;
        require_ai_enabled(&settings)?;
        Ok((settings, commands_db::get(conn, command_id)?))
    })?;

    conversion::check(&ConversionRequest {
        content: &entry.content,
        title: &entry.title,
        kind: entry.kind,
        source_shell: entry.shell.as_deref(),
        target,
    })?;

    let model = settings.effective_ai_model().to_owned();
    let credentials = ai.credentials.clone();
    let api_key = spawn_blocking(move || credentials.load())
        .await
        .map_err(|_| AppError::credential("The operating system credential manager stopped."))??
        .ok_or(AppError::AiNotConfigured)?;

    let client = ai.client.clone();
    let (answer, mut answers) = channel::<AppResult<ShellConversion>>(1);
    let handle = spawn(async move {
        let result = conversion::convert(
            &client,
            &api_key,
            &model,
            ConversionRequest {
                content: &entry.content,
                title: &entry.title,
                kind: entry.kind,
                source_shell: entry.shell.as_deref(),
                target,
            },
        )
        .await;
        // The receiver is gone only when this request was already abandoned.
        let _ = answer.send(result).await;
    });

    ai.inflight.register(request_id, handle)?;
    let received = answers.recv().await;
    ai.inflight.finish(request_id)?;

    // No answer means the task was aborted, which only Stop does.
    received.unwrap_or(Err(AppError::AiCancelled))
}

fn shell_options() -> Vec<ShellOption> {
    SHELLS
        .iter()
        .map(|shell| ShellOption {
            id: shell.as_str(),
            label: shell.label(),
        })
        .collect()
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
    use crate::models::{Command, CommandInput};

    fn settings(ai_enabled: bool) -> AppSettings {
        AppSettings {
            ai_enabled,
            ..AppSettings::default()
        }
    }

    fn library(ai_enabled: bool) -> (Database, i64) {
        let db = Database::open_in_memory().unwrap();
        let input: CommandInput = serde_json::from_value(serde_json::json!({
            "title": "Set the API host",
            "content": "export API_HOST=example.com",
            "shell": "bash"
        }))
        .unwrap();

        let id = db.with_mut(|conn| commands_db::create(conn, input)).unwrap().id;
        db.with_mut(|conn| settings::save(conn, settings(ai_enabled))).unwrap();
        (db, id)
    }

    /// The same read the command performs before it releases the lock.
    fn prepare(db: &Database, command_id: i64) -> AppResult<Command> {
        db.with(|conn| {
            require_ai_enabled(&settings::load(conn)?)?;
            commands_db::get(conn, command_id)
        })
    }

    #[test]
    fn converting_is_refused_while_ai_is_off() {
        let (db, id) = library(false);
        assert_eq!(prepare(&db, id).unwrap_err().kind(), "ai_disabled");
    }

    /// The dialog sends an id, not content, so the model can only ever see what
    /// the library actually holds for that entry.
    #[test]
    fn the_entry_comes_from_the_library_row() {
        let (db, id) = library(true);
        let entry = prepare(&db, id).unwrap();

        assert_eq!(entry.content, "export API_HOST=example.com");
        assert_eq!(entry.shell.as_deref(), Some("bash"));
    }

    #[test]
    fn converting_a_deleted_entry_fails_before_any_request() {
        let (db, id) = library(true);
        db.with_mut(|conn| commands_db::delete(conn, id)).unwrap();

        assert_eq!(prepare(&db, id).unwrap_err().kind(), "not_found");
    }

    #[test]
    fn an_unsupported_target_is_rejected_before_anything_is_read() {
        assert_eq!(
            TargetShell::parse("nushell").unwrap_err().kind(),
            "invalid"
        );
    }

    /// The picker is filled from the backend, so it can never offer a shell the
    /// prompt and parser do not both know about.
    #[test]
    fn the_offered_shells_are_exactly_the_supported_ones() {
        let options = shell_options();

        assert_eq!(options.len(), 4);
        for option in &options {
            assert_eq!(TargetShell::parse(option.id).unwrap().as_str(), option.id);
            assert!(!option.label.is_empty());
        }
        assert_eq!(options[0].id, "bash");
        assert_eq!(options[3].label, "PowerShell");
    }
}
