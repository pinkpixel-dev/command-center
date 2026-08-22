//! Running one structured task as a single ephemeral Codex turn.
//!
//! Command Center asks for strict JSON and nothing else. Codex is an agent
//! protocol, so a turn can also try to run a command, edit a file, or call a
//! tool. None of those are wanted here, and this module is where that is
//! enforced: any such item fails the turn and the result is discarded.
//!
//! The sandbox and approval policy bound what a tool *could* reach. This is
//! the part that makes sure nothing is acted on even so.

use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use crate::ai::prompts::StructuredTask;
use crate::error::{AppError, AppResult};

/// Starting a thread reaches OpenAI, so it is not instant and not local.
pub const THREAD_START_TIMEOUT: Duration = Duration::from_secs(60);
pub const TURN_START_TIMEOUT: Duration = Duration::from_secs(60);
pub const INTERRUPT_TIMEOUT: Duration = Duration::from_secs(10);

/// The most assistant text one structured task may produce, in characters.
/// The workflow parsers bound their own fields; this stops a runaway stream
/// long before that.
pub const MAX_ANSWER_CHARS: usize = 512 * 1024;

/// What a turn item means for a structured task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemDisposition {
    /// Ordinary conversation. Its text may be part of the answer.
    Answer,
    /// Harmless progress reporting. Ignored.
    Ignored,
    /// The model tried to act. The turn fails and the result is thrown away.
    Forbidden,
}

/// Classifies a `ThreadItem` by its `type` discriminator.
///
/// Unknown types are forbidden, not ignored. A future Codex release that adds
/// a way to act must not be silently permitted by an older client.
pub fn item_disposition(item_type: &str) -> ItemDisposition {
    match item_type {
        "agentMessage" => ItemDisposition::Answer,
        // The model thinking out loud, and bookkeeping. Neither acts.
        "userMessage" | "reasoning" | "plan" | "contextCompaction" => ItemDisposition::Ignored,
        _ => ItemDisposition::Forbidden,
    }
}

/// The assistant text carried by an item, when it has any.
pub fn agent_text(item: &Value) -> Option<String> {
    if item.get("type").and_then(Value::as_str) != Some("agentMessage") {
        return None;
    }
    item.get("text")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// Builds the `thread/start` parameters for a structured task.
///
/// Every value here is a containment decision. `ephemeral` keeps the thread
/// off disk, `cwd` points at an empty private directory, the sandbox is
/// read-only, and the approval policy makes Codex ask before acting so the
/// client can refuse.
pub fn thread_params(model: &str, working_dir: &Path, instructions: &str) -> Value {
    json!({
        "ephemeral": true,
        "model": model,
        "cwd": working_dir.to_string_lossy(),
        "sandbox": "read-only",
        "approvalPolicy": "untrusted",
        "developerInstructions": instructions,
        // Identify this client honestly rather than inheriting a default that
        // attributes the thread to another editor.
        "threadSource": "commandCenter",
    })
}

/// Builds the `turn/start` parameters. The output schema applies to this turn
/// only, which is why it is set here and not on the thread.
pub fn turn_params(thread_id: &str, input: &str, task: &StructuredTask) -> Value {
    json!({
        "threadId": thread_id,
        "input": [{ "type": "text", "text": input }],
        "outputSchema": task.schema,
    })
}

/// Turns a finished turn into either the answer or an error.
pub fn turn_outcome(turn: &Value, collected: &str) -> AppResult<String> {
    match turn.get("status").and_then(Value::as_str) {
        Some("completed") => {
            let answer = collected.trim();
            if answer.is_empty() {
                return Err(AppError::AiMalformed(
                    "Codex finished without returning a result.".into(),
                ));
            }
            Ok(answer.to_owned())
        }
        Some("interrupted") => Err(AppError::AiCancelled),
        _ => Err(turn_error(turn.get("error"))),
    }
}

/// Maps a turn failure onto the app's existing error shape.
///
/// Codex's own message is not repeated: it can carry an address, a request
/// body, or an account identifier. The error kind is what the user needs, and
/// the kind is what `codexErrorInfo` reliably provides.
pub fn turn_error(error: Option<&Value>) -> AppError {
    let info = error.and_then(|error| error.get("codexErrorInfo"));
    // The variant is either a bare string or a single-key object.
    let kind = info
        .and_then(Value::as_str)
        .or_else(|| {
            info.and_then(Value::as_object)
                .and_then(|object| object.keys().next())
                .map(String::as_str)
        })
        .unwrap_or("other");

    match kind {
        "usageLimitExceeded" | "sessionBudgetExceeded" => AppError::ai_rate_limit(
            "Your ChatGPT plan's usage limit was reached. Wait for it to reset, or switch to the OpenAI API key provider.",
        ),
        "contextWindowExceeded" => AppError::AiIncomplete(
            "The request was too long for this model. Try a smaller selection.".into(),
        ),
        "unauthorized" => AppError::ai_auth(
            "The ChatGPT connection is no longer valid. Connect ChatGPT again in Settings.",
        ),
        "serverOverloaded" => {
            AppError::ai_rate_limit("OpenAI is busy right now. Try again in a moment.")
        }
        "httpConnectionFailed" | "responseStreamConnectionFailed" | "responseStreamDisconnected" => {
            AppError::ai_network("Codex could not reach OpenAI. Check the network and try again.")
        }
        "sandboxError" => AppError::ai_response(
            "Codex stopped the request because it tried to act outside its sandbox.",
        ),
        "cyberPolicy" => AppError::AiRefusal,
        _ => AppError::ai_response("Codex could not complete the request."),
    }
}

/// The error used when a turn tries to act instead of answering.
pub fn forbidden_item_error(item_type: &str) -> AppError {
    // The item type is named because it is a fixed protocol identifier, not
    // model output, and it is what makes a bug report actionable.
    AppError::ai_response(format!(
        "Codex tried to run a {item_type} step, which Command Center does not allow. The result was discarded."
    ))
}

/// Appends streamed text, refusing to grow past the ceiling.
pub fn append_bounded(collected: &mut String, delta: &str) -> AppResult<()> {
    if collected.chars().count() + delta.chars().count() > MAX_ANSWER_CHARS {
        return Err(AppError::AiResponseTooLarge);
    }
    collected.push_str(delta);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_agent_message_is_the_answer() {
        assert_eq!(item_disposition("agentMessage"), ItemDisposition::Answer);
    }

    #[test]
    fn thinking_and_bookkeeping_are_ignored() {
        for kind in ["userMessage", "reasoning", "plan", "contextCompaction"] {
            assert_eq!(item_disposition(kind), ItemDisposition::Ignored, "{kind}");
        }
    }

    #[test]
    fn every_way_of_acting_is_forbidden() {
        for kind in [
            "commandExecution",
            "fileChange",
            "mcpToolCall",
            "dynamicToolCall",
            "collabAgentToolCall",
            "subAgentActivity",
            "webSearch",
            "imageView",
            "imageGeneration",
            "hookPrompt",
            "sleep",
        ] {
            assert_eq!(item_disposition(kind), ItemDisposition::Forbidden, "{kind}");
        }
    }

    #[test]
    fn an_unknown_item_type_is_forbidden_rather_than_ignored() {
        // A future Codex release must not gain the ability to act just
        // because this client has not heard of the item yet.
        assert_eq!(
            item_disposition("someFutureToolCall"),
            ItemDisposition::Forbidden
        );
        assert_eq!(item_disposition(""), ItemDisposition::Forbidden);
    }

    #[test]
    fn agent_text_is_read_only_from_an_agent_message() {
        let message = json!({ "type": "agentMessage", "text": "{\"ready\":true}" });
        assert_eq!(agent_text(&message).as_deref(), Some("{\"ready\":true}"));

        let command = json!({ "type": "commandExecution", "text": "rm -rf /" });
        assert_eq!(agent_text(&command), None);
    }

    #[test]
    fn the_thread_is_ephemeral_read_only_and_confined() {
        let params = thread_params("gpt-5.6-luna", Path::new("/app-data/work"), "instructions");

        assert_eq!(params["ephemeral"], true);
        assert_eq!(params["sandbox"], "read-only");
        assert_eq!(params["cwd"], "/app-data/work");
        assert_eq!(params["model"], "gpt-5.6-luna");
        // "never" would mean never ask and always run, which is the opposite
        // of what this needs.
        assert_eq!(params["approvalPolicy"], "untrusted");
        assert_ne!(params["approvalPolicy"], "never");
    }

    #[test]
    fn the_output_schema_is_set_per_turn_not_per_thread() {
        let task = crate::ai::prompts::connection_test();
        let thread = thread_params("gpt-5.6-luna", Path::new("/work"), task.instructions);
        let turn = turn_params("thread-1", "input text", task);

        assert!(thread.get("outputSchema").is_none());
        assert_eq!(turn["outputSchema"], task.schema);
        assert_eq!(turn["threadId"], "thread-1");
        assert_eq!(turn["input"][0]["text"], "input text");
        assert_eq!(turn["input"][0]["type"], "text");
    }

    #[test]
    fn a_completed_turn_returns_its_collected_answer() {
        let turn = json!({ "status": "completed" });
        assert_eq!(turn_outcome(&turn, "  {\"ready\":true}  ").unwrap(), "{\"ready\":true}");
    }

    #[test]
    fn a_completed_turn_with_no_text_is_a_malformed_result() {
        let turn = json!({ "status": "completed" });
        assert!(matches!(
            turn_outcome(&turn, "   "),
            Err(AppError::AiMalformed(_))
        ));
    }

    #[test]
    fn an_interrupted_turn_reads_as_a_cancellation() {
        let turn = json!({ "status": "interrupted" });
        assert!(matches!(turn_outcome(&turn, "partial"), Err(AppError::AiCancelled)));
    }

    #[test]
    fn a_usage_limit_gets_its_own_message_and_names_the_alternative() {
        let turn = json!({
            "status": "failed",
            "error": { "message": "quota", "codexErrorInfo": "usageLimitExceeded" },
        });

        let error = turn_outcome(&turn, "").unwrap_err();
        assert_eq!(error.kind(), "ai_rate_limit");
        assert!(error.to_string().contains("usage limit"));
        assert!(error.to_string().contains("API key"));
    }

    #[test]
    fn an_expired_connection_asks_the_user_to_reconnect() {
        let turn = json!({
            "status": "failed",
            "error": { "message": "401", "codexErrorInfo": "unauthorized" },
        });

        let error = turn_outcome(&turn, "").unwrap_err();
        assert_eq!(error.kind(), "ai_auth");
        assert!(error.to_string().contains("Connect ChatGPT again"));
    }

    #[test]
    fn an_object_shaped_error_variant_is_read_by_its_key() {
        let turn = json!({
            "status": "failed",
            "error": {
                "message": "connection failed",
                "codexErrorInfo": { "httpConnectionFailed": { "httpStatusCode": 502 } },
            },
        });

        assert_eq!(turn_outcome(&turn, "").unwrap_err().kind(), "ai_network");
    }

    #[test]
    fn a_failure_never_repeats_codex_error_text() {
        let turn = json!({
            "status": "failed",
            "error": {
                "message": "POST https://api.openai.com/v1/responses failed for account acct_123",
                "codexErrorInfo": "other",
            },
        });

        let message = turn_outcome(&turn, "").unwrap_err().to_string();
        assert!(!message.contains("://"));
        assert!(!message.contains("acct_123"));
    }

    #[test]
    fn a_failure_with_no_error_body_still_maps_cleanly() {
        let turn = json!({ "status": "failed" });
        assert_eq!(turn_outcome(&turn, "").unwrap_err().kind(), "ai_response");
    }

    #[test]
    fn a_forbidden_item_names_what_was_attempted() {
        let error = forbidden_item_error("commandExecution");
        assert!(error.to_string().contains("commandExecution"));
        assert!(error.to_string().contains("discarded"));
    }

    #[test]
    fn streamed_text_is_bounded() {
        let mut collected = String::new();
        append_bounded(&mut collected, "hello").unwrap();
        assert_eq!(collected, "hello");

        let flood = "x".repeat(MAX_ANSWER_CHARS);
        assert!(matches!(
            append_bounded(&mut collected, &flood),
            Err(AppError::AiResponseTooLarge)
        ));
        // The overflowing chunk is not appended.
        assert_eq!(collected, "hello");
    }
}
