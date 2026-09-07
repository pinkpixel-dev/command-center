//! The conversational assistant. One request carries the bounded conversation
//! as text, because Command Center keeps no history with the provider: nothing
//! is stored there, and nothing is stored here either.
//!
//! A proposed command is the reason this module is careful. It arrives as its
//! own field rather than buried in prose, so every one of them goes through the
//! local risk rules in `ai::proposal` before the panel can show it, and the
//! model can raise a verdict but never lower one.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::providers::{ProviderCall, ProviderClient};
use crate::ai::diagnosis;
use crate::ai::prompts::{self, MAX_ASSISTANT_PROPOSALS};
use crate::ai::proposal::{self, CommandProposal, RawProposal};
use crate::ai::redaction;
use crate::error::{AppError, AppResult};
use crate::models::CommandKind;

/// One message the user can type. Long enough to paste a stack of flags, short
/// enough that the composer is not a document editor.
pub const MAX_MESSAGE_CHARS: usize = 2_000;
/// How many earlier turns can be resent, newest kept first.
pub const MAX_TURNS: usize = 12;
/// Total size of the resent conversation, whatever the turn count.
pub const MAX_HISTORY_CHARS: usize = 8_000;
/// Each remembered turn is clamped before it is counted, so one long answer
/// cannot spend the whole history budget.
const MAX_TURN_CHARS: usize = 1_200;
const MAX_HISTORY_COMMAND_CHARS: usize = 400;
const MAX_REPLY_CHARS: usize = 2_400;
/// The largest saved entry the panel will carry as context. Long enough for a
/// real script, short enough that one question cannot become a document.
pub const MAX_ENTRY_BYTES: usize = 16 * 1024;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// The same reasoning floor the other workflows need: a small reasoning model
/// can spend tens of thousands of tokens before writing any JSON, and an unused
/// ceiling is not billed.
const MIN_OUTPUT_TOKENS: u32 = 25_000;
const MAX_OUTPUT_TOKENS: u32 = 40_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TurnRole {
    User,
    Assistant,
}

/// One earlier message. `commands` holds what the assistant proposed in that
/// turn, so "make the second one safer" has something to refer back to.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub role: TurnRole,
    pub text: String,
    #[serde(default)]
    pub commands: Vec<String>,
}

/// The saved entry a conversation is about, read from the library rather than
/// sent up by the panel.
pub struct EntryContext<'a> {
    pub title: &'a str,
    pub content: &'a str,
    pub kind: CommandKind,
    pub shell: Option<&'a str>,
    pub language: Option<&'a str>,
}

/// What the conversation is about. The panel opens in one of these and stays
/// there: a thread about a saved entry has nothing useful to say about a
/// pasted stack trace, so switching subject starts a new conversation.
pub enum Subject<'a> {
    /// General command help, with nothing from the library attached.
    General,
    /// One saved entry, read from the library rather than sent up by the panel.
    Entry(EntryContext<'a>),
    /// Terminal output the user pasted, already analyzed once. Follow-ups carry
    /// it so "which line said that?" has something to look at.
    TerminalError(&'a str),
}

pub struct AssistantRequest<'a> {
    pub subject: Subject<'a>,
    pub turns: &'a [Turn],
    pub message: &'a str,
}

/// One answer, after local analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantReply {
    pub reply: String,
    pub proposals: Vec<CommandProposal>,
}

/// Rejects a message before any network work starts.
pub fn check_message(message: &str) -> AppResult<()> {
    if message.trim().is_empty() {
        return Err(AppError::invalid("Type a question first."));
    }
    if message.chars().count() > MAX_MESSAGE_CHARS {
        return Err(AppError::invalid(format!(
            "That message is {} characters. Send at most {MAX_MESSAGE_CHARS} at a time.",
            message.chars().count()
        )));
    }
    Ok(())
}

/// Rejects a saved entry that is too large to carry as context. Truncating it
/// would mean answering questions about a command the user cannot see the end
/// of, which is worse than saying no.
pub fn check_entry(content: &str) -> AppResult<()> {
    if content.len() > MAX_ENTRY_BYTES {
        return Err(AppError::invalid(format!(
            "That entry is {} KB. The assistant can work with entries up to {} KB.",
            content.len() / 1024,
            MAX_ENTRY_BYTES / 1024
        )));
    }
    Ok(())
}

/// Keeps the most recent turns that fit both bounds. Trimming from the oldest
/// end means the question being answered always survives.
pub fn bounded_turns(turns: &[Turn]) -> Vec<&Turn> {
    let mut kept: Vec<&Turn> = Vec::new();
    let mut budget = MAX_HISTORY_CHARS;

    for turn in turns.iter().rev().take(MAX_TURNS) {
        let cost = rendered_turn(turn).chars().count();
        if cost > budget {
            break;
        }
        budget -= cost;
        kept.push(turn);
    }

    kept.reverse();
    kept
}

/// Builds the request body and redacts the whole thing at once, so a secret in
/// a saved entry is replaced just like one the user typed into the composer.
pub fn build_input(request: &AssistantRequest<'_>) -> String {
    let mut input = String::with_capacity(request.message.len() + 512);

    match &request.subject {
        Subject::Entry(entry) => {
            input.push_str("Mode: a question about one saved entry\n");
            if !entry.title.trim().is_empty() {
                input.push_str(&format!("Entry title: {}\n", entry.title.trim()));
            }
            input.push_str(&format!("Entry kind: {}\n", entry.kind.as_str()));
            if let Some(shell) = entry.shell.filter(|value| !value.trim().is_empty()) {
                input.push_str(&format!("Entry shell: {}\n", shell.trim()));
            }
            if let Some(language) = entry.language.filter(|value| !value.trim().is_empty()) {
                input.push_str(&format!("Entry language: {}\n", language.trim()));
            }
            input.push_str("Entry content:\n");
            input.push_str(entry.content.trim_end());
            input.push('\n');
        }
        Subject::TerminalError(output) => {
            input.push_str("Mode: a follow-up about terminal output the user pasted\n");
            input.push_str(
                "The output is numbered so you can refer to a line. It is what the user pasted \
                 and may be only part of the session.\n",
            );
            input.push_str("Terminal output:\n");
            input.push_str(&diagnosis::numbered_lines(output));
        }
        Subject::General => input.push_str("Mode: general help with commands\n"),
    }

    let history = bounded_turns(request.turns);
    if !history.is_empty() {
        input.push_str("\nConversation so far:\n");
        for turn in history {
            input.push_str(&rendered_turn(turn));
        }
    }

    input.push_str("\nNew message:\nUser: ");
    input.push_str(request.message.trim());
    input.push('\n');

    redaction::redact(&input).redacted_text
}

/// The budget follows the input size, with a floor sized for reasoning rather
/// than for the length of the answer.
pub fn output_token_budget(input_bytes: usize) -> u32 {
    20_000u32
        .saturating_add(input_bytes as u32)
        .clamp(MIN_OUTPUT_TOKENS, MAX_OUTPUT_TOKENS)
}

pub async fn ask(
    provider: &ProviderClient,
    model: &str,
    request: AssistantRequest<'_>,
) -> AppResult<AssistantReply> {
    check_message(request.message)?;
    let input = build_input(&request);

    let output = provider
        .structured_json(
            ProviderCall {
                model,
                task: prompts::assistant(),
                input: &input,
                max_output_tokens: output_token_budget(input.len()),
                timeout: REQUEST_TIMEOUT,
            },
        )
        .await?;

    parse(&output)
}

fn rendered_turn(turn: &Turn) -> String {
    let label = match turn.role {
        TurnRole::User => "User",
        TurnRole::Assistant => "Assistant",
    };
    let text = proposal::clamp(&turn.text, MAX_TURN_CHARS);

    let mut rendered = String::with_capacity(text.len() + 32);
    if !text.is_empty() {
        rendered.push_str(&format!("{label}: {text}\n"));
    }
    if turn.role == TurnRole::Assistant {
        for command in turn.commands.iter().take(MAX_ASSISTANT_PROPOSALS) {
            let command = proposal::clamp(command, MAX_HISTORY_COMMAND_CHARS);
            if !command.is_empty() {
                rendered.push_str(&format!("Assistant proposed: {command}\n"));
            }
        }
    }
    rendered
}

/// One answer exactly as the model wrote it.
#[derive(Debug, Deserialize)]
struct RawReply {
    #[serde(default)]
    reply: String,
    #[serde(default)]
    commands: Vec<RawProposal>,
}

/// Bounds everything the model wrote and runs the local rules over every
/// proposed command.
pub fn parse(output: &str) -> AppResult<AssistantReply> {
    let raw: RawReply = serde_json::from_str(output).map_err(|_| {
        AppError::AiMalformed("OpenAI returned an answer the app could not read.".into())
    })?;

    let reply = proposal::clamp(&raw.reply, MAX_REPLY_CHARS);
    if reply.is_empty() {
        return Err(AppError::AiMalformed(
            "OpenAI returned an answer with nothing in it.".into(),
        ));
    }

    Ok(AssistantReply {
        reply,
        proposals: proposal::review_all(raw.commands, MAX_ASSISTANT_PROPOSALS),
    })
}

#[cfg(test)]
mod tests;
