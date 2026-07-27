//! The conversational assistant. One request carries the bounded conversation
//! as text, because Command Center keeps no history with the provider: nothing
//! is stored there, and nothing is stored here either.
//!
//! A proposed command is the reason this module is careful. It arrives as its
//! own field rather than buried in prose, so every one of them goes through the
//! local risk rules before the panel can show it, and the model can raise a
//! verdict but never lower one.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::client::{OpenAiClient, StructuredCall};
use crate::ai::prompts::{self, MAX_ASSISTANT_PROPOSALS};
use crate::ai::redaction;
use crate::error::{AppError, AppResult};
use crate::models::{CommandKind, RiskLevel};
use crate::normalize::normalize_command;
use crate::risk;

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
const MAX_TITLE_CHARS: usize = 120;
const MAX_WHY_CHARS: usize = 300;
const MAX_REASONS: usize = 5;
const MAX_REASON_CHARS: usize = 160;
/// A proposal is a command, not a program. The entry form is where anything
/// longer belongs.
const MAX_PROPOSAL_BYTES: usize = 4 * 1024;
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

pub struct AssistantRequest<'a> {
    pub entry: Option<EntryContext<'a>>,
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

/// A command the model suggested. The level is the stricter of the two
/// verdicts, and each side keeps its own reasons so the panel never presents a
/// guess as a local rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandProposal {
    pub command: String,
    pub title: String,
    pub why: String,
    pub kind: CommandKind,
    pub shell: Option<String>,
    pub risk_level: RiskLevel,
    pub local_reasons: Vec<String>,
    pub ai_reasons: Vec<String>,
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

    match &request.entry {
        Some(entry) => {
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
        None => input.push_str("Mode: general help with commands\n"),
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
    client: &OpenAiClient,
    api_key: &str,
    model: &str,
    request: AssistantRequest<'_>,
) -> AppResult<AssistantReply> {
    check_message(request.message)?;
    let input = build_input(&request);

    let output = client
        .structured_json(
            api_key,
            StructuredCall {
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
    let text = clamp(&turn.text, MAX_TURN_CHARS);

    let mut rendered = String::with_capacity(text.len() + 32);
    if !text.is_empty() {
        rendered.push_str(&format!("{label}: {text}\n"));
    }
    if turn.role == TurnRole::Assistant {
        for command in turn.commands.iter().take(MAX_ASSISTANT_PROPOSALS) {
            let command = clamp(command, MAX_HISTORY_COMMAND_CHARS);
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

#[derive(Debug, Deserialize)]
struct RawProposal {
    #[serde(default)]
    command: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    why: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    shell: Option<String>,
    #[serde(default)]
    risk_suggestion: String,
    #[serde(default)]
    risk_reasons: Vec<String>,
}

/// Bounds everything the model wrote and runs the local rules over every
/// proposed command.
pub fn parse(output: &str) -> AppResult<AssistantReply> {
    let raw: RawReply = serde_json::from_str(output).map_err(|_| {
        AppError::AiMalformed("OpenAI returned an answer the app could not read.".into())
    })?;

    let reply = clamp(&raw.reply, MAX_REPLY_CHARS);
    if reply.is_empty() {
        return Err(AppError::AiMalformed(
            "OpenAI returned an answer with nothing in it.".into(),
        ));
    }

    let mut proposals: Vec<CommandProposal> = Vec::new();
    for candidate in raw.commands {
        let Some(proposal) = review(candidate) else {
            continue;
        };
        if proposals
            .iter()
            .any(|kept| kept.command == proposal.command)
        {
            continue;
        }
        proposals.push(proposal);
        if proposals.len() == MAX_ASSISTANT_PROPOSALS {
            break;
        }
    }

    Ok(AssistantReply { reply, proposals })
}

/// Normalizes the command the way a saved entry would be, then lets the local
/// rules have the final word on how risky it is.
fn review(raw: RawProposal) -> Option<CommandProposal> {
    let command = normalize_command(&raw.command);
    if command.trim().is_empty() || command.len() > MAX_PROPOSAL_BYTES {
        return None;
    }

    let (local_level, local_reasons) = risk::assess(&command);
    // An unknown or missing suggestion counts as Safe, which can never lower
    // the local verdict because the effective level is the higher of the two.
    let suggested = RiskLevel::parse(raw.risk_suggestion.trim()).unwrap_or(RiskLevel::Safe);
    let title = clamp(&raw.title, MAX_TITLE_CHARS);

    Some(CommandProposal {
        title: if title.is_empty() {
            "Suggested command".to_owned()
        } else {
            title
        },
        why: clamp(&raw.why, MAX_WHY_CHARS),
        kind: CommandKind::parse(raw.kind.trim()).unwrap_or(CommandKind::Command),
        shell: clamp_optional(raw.shell.as_deref()),
        risk_level: local_level.max(suggested),
        local_reasons,
        ai_reasons: clamp_reasons(raw.risk_reasons),
        command,
    })
}

fn clamp(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

fn clamp_optional(value: Option<&str>) -> Option<String> {
    let cleaned = clamp(value.unwrap_or_default(), 32);
    (!cleaned.is_empty()).then_some(cleaned)
}

fn clamp_reasons(values: Vec<String>) -> Vec<String> {
    let mut cleaned: Vec<String> = Vec::new();
    for value in values {
        let text = clamp(&value, MAX_REASON_CHARS);
        if !text.is_empty() && !cleaned.iter().any(|kept| kept.eq_ignore_ascii_case(&text)) {
            cleaned.push(text);
        }
        if cleaned.len() == MAX_REASONS {
            break;
        }
    }
    cleaned
}

#[cfg(test)]
mod tests;
