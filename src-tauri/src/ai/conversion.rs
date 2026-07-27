//! Shell conversion. One saved entry and a target shell go out, a rewritten
//! command and an account of what changed underneath it come back.
//!
//! A conversion is never presented as a guaranteed equivalent. Two shells can
//! print the same output and still differ on globbing, quoting, exit status, or
//! what happens when a file is missing, so the result carries how close it is
//! rather than whether it is correct. The converted command is a proposal like
//! any other: the local rules judge it, and nothing saves itself.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::client::{OpenAiClient, StructuredCall};
use crate::ai::prompts::{self, MAX_CONVERSION_NOTES};
use crate::ai::proposal::{self, CommandProposal, RawProposal};
use crate::ai::redaction;
use crate::error::{AppError, AppResult};
use crate::models::CommandKind;
use crate::normalize::normalize_command;

/// Long enough for a real script, short enough that one conversion cannot turn
/// into a document-sized request.
pub const MAX_COMMAND_BYTES: usize = 16 * 1024;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// The same reasoning floor the other workflows need: a small reasoning model
/// can spend tens of thousands of tokens before writing any JSON, and an unused
/// ceiling is not billed.
const MIN_OUTPUT_TOKENS: u32 = 25_000;
const MAX_OUTPUT_TOKENS: u32 = 40_000;
const MAX_NOTE_CHARS: usize = 240;
const MAX_NOTES_CHARS: usize = 600;

/// The shells conversion can target. This is a fixed list rather than free
/// text: the prompt names them, the parser trusts none of them, and the panel
/// offers exactly these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetShell {
    Bash,
    Fish,
    Zsh,
    PowerShell,
}

impl TargetShell {
    /// The identifier stored on a saved entry and sent in the request.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Fish => "fish",
            Self::Zsh => "zsh",
            Self::PowerShell => "powershell",
        }
    }

    /// How the shell is written when a person reads it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Fish => "fish",
            Self::Zsh => "zsh",
            Self::PowerShell => "PowerShell",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "bash" | "sh" => Ok(Self::Bash),
            "fish" => Ok(Self::Fish),
            "zsh" => Ok(Self::Zsh),
            "powershell" | "pwsh" | "posh" => Ok(Self::PowerShell),
            other => Err(AppError::invalid(format!(
                "Command Center cannot convert to {other} yet."
            ))),
        }
    }

    /// Recognizes a saved entry's shell field, which is free text and may be
    /// anything the user typed. `None` means "not one of the four", which is a
    /// perfectly normal answer.
    pub fn detect(value: Option<&str>) -> Option<Self> {
        Self::parse(value?).ok()
    }
}

/// How much of the original behaviour survived. There is deliberately no value
/// meaning "exact": claiming that is the one thing this feature must not do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Equivalence {
    /// Same job, same result on a normal machine.
    Close,
    /// Some of the original behaviour did not carry over.
    Partial,
    /// The model was not confident the rewrite is right.
    Uncertain,
}

impl Equivalence {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "close" => Self::Close,
            "partial" => Self::Partial,
            // Anything unrecognized reads as uncertain. Overstating how well a
            // conversion held up is the failure that matters here.
            _ => Self::Uncertain,
        }
    }
}

/// One conversion, after the local rules have reviewed the converted command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellConversion {
    /// The shell the entry was saved as, when the entry named one.
    pub source_shell: Option<String>,
    pub target_shell: TargetShell,
    pub original: String,
    pub converted: CommandProposal,
    pub equivalence: Equivalence,
    pub differences: Vec<String>,
    pub unsupported: Vec<String>,
    pub notes: String,
}

/// The context one conversion request needs, straight from the stored entry.
pub struct ConversionRequest<'a> {
    pub content: &'a str,
    pub title: &'a str,
    pub kind: CommandKind,
    pub source_shell: Option<&'a str>,
    pub target: TargetShell,
}

/// Rejects a conversion that cannot produce anything useful before any network
/// work starts.
pub fn check(request: &ConversionRequest<'_>) -> AppResult<()> {
    if request.content.trim().is_empty() {
        return Err(AppError::invalid("There is nothing in that entry to convert."));
    }
    if request.content.len() > MAX_COMMAND_BYTES {
        return Err(AppError::invalid(format!(
            "That entry is {} KB. Convert works on entries up to {} KB.",
            request.content.len() / 1024,
            MAX_COMMAND_BYTES / 1024
        )));
    }
    if TargetShell::detect(request.source_shell) == Some(request.target) {
        return Err(AppError::invalid(format!(
            "That entry is already saved as {}.",
            request.target.label()
        )));
    }
    Ok(())
}

/// The budget follows the entry size, with a floor sized for reasoning rather
/// than for the length of the answer.
pub fn output_token_budget(content_bytes: usize) -> u32 {
    20_000u32
        .saturating_add(content_bytes as u32)
        .clamp(MIN_OUTPUT_TOKENS, MAX_OUTPUT_TOKENS)
}

/// Builds the request body. Everything is redacted together, so a secret in a
/// title is replaced just like one in the command itself.
pub fn build_input(request: &ConversionRequest<'_>) -> String {
    let mut input = String::with_capacity(request.content.len() + 256);

    match request.source_shell.filter(|value| !value.trim().is_empty()) {
        Some(shell) => input.push_str(&format!("Source shell: {}\n", shell.trim())),
        // Saying so plainly beats letting the model assume bash silently.
        None => input.push_str("Source shell: not recorded, infer it from the content\n"),
    }
    input.push_str(&format!("Target shell: {}\n", request.target.as_str()));
    input.push_str(&format!("Kind: {}\n", request.kind.as_str()));
    if !request.title.trim().is_empty() {
        input.push_str(&format!("Title: {}\n", request.title.trim()));
    }
    input.push_str("\nOriginal content:\n");
    input.push_str(request.content);
    input.push('\n');

    redaction::redact(&input).redacted_text
}

pub async fn convert(
    client: &OpenAiClient,
    api_key: &str,
    model: &str,
    request: ConversionRequest<'_>,
) -> AppResult<ShellConversion> {
    check(&request)?;
    let input = build_input(&request);

    let output = client
        .structured_json(
            api_key,
            StructuredCall {
                model,
                task: prompts::shell_conversion(),
                input: &input,
                max_output_tokens: output_token_budget(request.content.len()),
                timeout: REQUEST_TIMEOUT,
            },
        )
        .await?;

    parse(&output, &request)
}

/// One conversion exactly as the model wrote it.
#[derive(Debug, Deserialize)]
struct RawConversion {
    #[serde(default)]
    converted: RawProposal,
    #[serde(default)]
    equivalence: String,
    #[serde(default)]
    differences: Vec<String>,
    #[serde(default)]
    unsupported: Vec<String>,
    #[serde(default)]
    notes: String,
}

/// Bounds everything the model wrote and runs the local rules over the
/// converted command.
pub fn parse(output: &str, request: &ConversionRequest<'_>) -> AppResult<ShellConversion> {
    let raw: RawConversion = serde_json::from_str(output).map_err(|_| {
        AppError::AiMalformed("OpenAI returned a conversion the app could not read.".into())
    })?;

    let mut converted = proposal::review(raw.converted).ok_or_else(|| {
        AppError::AiMalformed("OpenAI returned a conversion with no command in it.".into())
    })?;
    // The target is the app's decision, not the model's. Trusting the returned
    // shell would let a mislabelled answer look like a conversion that ran.
    converted.shell = Some(request.target.as_str().to_owned());

    let stated = Equivalence::parse(&raw.equivalence);
    let unchanged = normalize_command(&converted.command) == normalize_command(request.content);

    Ok(ShellConversion {
        source_shell: proposal::clamp_optional(request.source_shell, 32),
        target_shell: request.target,
        original: request.content.to_owned(),
        converted,
        // A rewrite that changed nothing cannot be a partial or uncertain one:
        // it is the original command, and the notes say why it needed nothing.
        equivalence: if unchanged { Equivalence::Close } else { stated },
        differences: clamp_notes(raw.differences),
        unsupported: clamp_notes(raw.unsupported),
        notes: proposal::clamp(&raw.notes, MAX_NOTES_CHARS),
    })
}

fn clamp_notes(values: Vec<String>) -> Vec<String> {
    let mut cleaned: Vec<String> = Vec::new();
    for value in values {
        let text = proposal::clamp(&value, MAX_NOTE_CHARS);
        if !text.is_empty() && !cleaned.iter().any(|kept| kept.eq_ignore_ascii_case(&text)) {
            cleaned.push(text);
        }
        if cleaned.len() == MAX_CONVERSION_NOTES {
            break;
        }
    }
    cleaned
}

#[cfg(test)]
mod tests;
