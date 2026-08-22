//! Structured command explanations. One request produces one explanation that
//! fills both the Quick and the Detailed view, so the app never pays for two
//! answers that disagree with each other.
//!
//! Everything the model says about safety is a suggestion. The local rules run
//! again here and keep the stricter verdict, and a proposed preview command is
//! dropped unless the local rules agree it is actually safer.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::providers::{ProviderCall, ProviderClient};
use crate::ai::prompts::{self, MAX_EXPLANATION_ITEMS};
use crate::ai::redaction;
use crate::error::{AppError, AppResult};
use crate::models::{CommandKind, RiskLevel};
use crate::normalize::normalize_command;
use crate::risk;

/// Long enough for a real script, short enough that one entry cannot turn into
/// a document-sized request.
pub const MAX_COMMAND_BYTES: usize = 16 * 1024;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// Same reasoning floor the importer needs: a small reasoning model can spend
/// tens of thousands of tokens before it writes any JSON, and an unused ceiling
/// is not billed.
const MIN_OUTPUT_TOKENS: u32 = 25_000;
const MAX_OUTPUT_TOKENS: u32 = 40_000;
const MAX_SUMMARY_CHARS: usize = 800;
const MAX_LINE_CHARS: usize = 240;
const MAX_FLAG_CHARS: usize = 60;

/// The explanation as the rest of the app sees it, after local analysis. This
/// is also the shape stored in the cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub summary: String,
    pub flags: Vec<FlagNote>,
    pub pipeline: Vec<PipelineStage>,
    pub side_effects: Vec<String>,
    pub safety: SafetyReview,
    pub preview_command: Option<PreviewCommand>,
    pub assumptions: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagNote {
    pub flag: String,
    pub meaning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    pub stage: String,
    pub purpose: String,
}

/// The effective level is the stricter of the two verdicts, and each side keeps
/// its own reasons so the UI never presents a guess as a local rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyReview {
    pub level: RiskLevel,
    pub local_reasons: Vec<String>,
    pub ai_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCommand {
    pub command: String,
    pub risk_level: RiskLevel,
    pub risk_reasons: Vec<String>,
}

impl Explanation {
    /// The text that feeds search. Commands are left out: search results should
    /// point at the library's own content, not at something a model wrote.
    pub fn search_text(&self) -> String {
        let mut parts: Vec<&str> = vec![&self.summary];
        parts.extend(self.flags.iter().flat_map(|note| [note.flag.as_str(), note.meaning.as_str()]));
        parts.extend(self.pipeline.iter().map(|stage| stage.purpose.as_str()));
        parts.extend(self.side_effects.iter().map(String::as_str));
        parts.extend(self.safety.ai_reasons.iter().map(String::as_str));
        parts.extend(self.assumptions.iter().map(String::as_str));
        parts.extend(self.caveats.iter().map(String::as_str));

        parts
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The context one explanation request needs, straight from the stored entry.
pub struct ExplanationRequest<'a> {
    pub content: &'a str,
    pub title: &'a str,
    pub description: &'a str,
    pub kind: CommandKind,
    pub shell: Option<&'a str>,
    pub language: Option<&'a str>,
}

/// Rejects an entry that is too large to send before any network work starts.
pub fn check_size(content: &str) -> AppResult<()> {
    if content.trim().is_empty() {
        return Err(AppError::invalid("There is nothing in that entry to explain."));
    }
    if content.len() > MAX_COMMAND_BYTES {
        return Err(AppError::invalid(format!(
            "That entry is {} KB. Explain works on entries up to {} KB.",
            content.len() / 1024,
            MAX_COMMAND_BYTES / 1024
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
/// title or a description is replaced just like one in the command itself.
pub fn build_input(request: &ExplanationRequest<'_>) -> String {
    let mut input = String::with_capacity(request.content.len() + 256);
    input.push_str(&format!("Kind: {}\n", request.kind.as_str()));
    if !request.title.trim().is_empty() {
        input.push_str(&format!("Title: {}\n", request.title.trim()));
    }
    if !request.description.trim().is_empty() {
        input.push_str(&format!("Saved description: {}\n", request.description.trim()));
    }
    if let Some(shell) = request.shell.filter(|value| !value.trim().is_empty()) {
        input.push_str(&format!("Shell: {}\n", shell.trim()));
    }
    if let Some(language) = request.language.filter(|value| !value.trim().is_empty()) {
        input.push_str(&format!("Language: {}\n", language.trim()));
    }
    input.push_str("\nContent:\n");
    input.push_str(request.content);
    input.push('\n');

    redaction::redact(&input).redacted_text
}

pub async fn generate(
    provider: &ProviderClient,
    model: &str,
    request: ExplanationRequest<'_>,
) -> AppResult<Explanation> {
    check_size(request.content)?;
    let input = build_input(&request);

    let output = provider
        .structured_json(
            ProviderCall {
                model,
                task: prompts::explanation(),
                input: &input,
                max_output_tokens: output_token_budget(request.content.len()),
                timeout: REQUEST_TIMEOUT,
            },
        )
        .await?;

    parse(&output, request.content)
}

/// One explanation exactly as the model wrote it.
#[derive(Debug, Deserialize)]
struct RawExplanation {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    flags: Vec<RawFlag>,
    #[serde(default)]
    pipeline: Vec<RawStage>,
    #[serde(default)]
    side_effects: Vec<String>,
    #[serde(default)]
    safety: RawSafety,
    #[serde(default)]
    preview_command: Option<String>,
    #[serde(default)]
    assumptions: Vec<String>,
    #[serde(default)]
    caveats: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawFlag {
    #[serde(default)]
    flag: String,
    #[serde(default)]
    meaning: String,
}

#[derive(Debug, Deserialize)]
struct RawStage {
    #[serde(default)]
    stage: String,
    #[serde(default)]
    purpose: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawSafety {
    #[serde(default)]
    level: String,
    #[serde(default)]
    reasons: Vec<String>,
}

/// Bounds everything the model wrote and re-runs the local rules against the
/// real, unredacted content.
pub fn parse(output: &str, content: &str) -> AppResult<Explanation> {
    let raw: RawExplanation = serde_json::from_str(output).map_err(|_| {
        AppError::AiMalformed("OpenAI returned an explanation the app could not read.".into())
    })?;

    let summary = clamp(&raw.summary, MAX_SUMMARY_CHARS);
    if summary.is_empty() {
        return Err(AppError::AiMalformed(
            "OpenAI returned an explanation with no summary.".into(),
        ));
    }

    let (local_level, local_reasons) = risk::assess(content);
    // An unknown or missing suggestion counts as Safe, which can never lower
    // the local verdict because the effective level is the higher of the two.
    let suggested = RiskLevel::parse(raw.safety.level.trim()).unwrap_or(RiskLevel::Safe);

    Ok(Explanation {
        summary,
        flags: raw
            .flags
            .into_iter()
            .filter_map(|note| {
                let flag = clamp(&note.flag, MAX_FLAG_CHARS);
                let meaning = clamp(&note.meaning, MAX_LINE_CHARS);
                (!flag.is_empty() && !meaning.is_empty()).then_some(FlagNote { flag, meaning })
            })
            .take(MAX_EXPLANATION_ITEMS)
            .collect(),
        pipeline: raw
            .pipeline
            .into_iter()
            .filter_map(|stage| {
                let name = clamp(&stage.stage, MAX_LINE_CHARS);
                let purpose = clamp(&stage.purpose, MAX_LINE_CHARS);
                (!name.is_empty() && !purpose.is_empty()).then_some(PipelineStage {
                    stage: name,
                    purpose,
                })
            })
            .take(MAX_EXPLANATION_ITEMS)
            .collect(),
        side_effects: clamp_list(raw.side_effects),
        safety: SafetyReview {
            level: local_level.max(suggested),
            local_reasons,
            ai_reasons: clamp_list(raw.safety.reasons),
        },
        preview_command: preview(raw.preview_command.as_deref(), content),
        assumptions: clamp_list(raw.assumptions),
        caveats: clamp_list(raw.caveats),
    })
}

/// A preview only earns its place when the local rules agree it is safer than
/// the command it previews. A destructive "preview" is a contradiction, and a
/// restatement of the original helps nobody.
fn preview(proposed: Option<&str>, content: &str) -> Option<PreviewCommand> {
    let command = proposed?.trim();
    if command.is_empty() || command.len() > MAX_COMMAND_BYTES {
        return None;
    }
    if normalize_command(command) == normalize_command(content) {
        return None;
    }

    let (level, reasons) = risk::assess(command);
    if level == RiskLevel::Destructive {
        return None;
    }

    Some(PreviewCommand {
        command: command.to_owned(),
        risk_level: level,
        risk_reasons: reasons,
    })
}

fn clamp(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

fn clamp_list(values: Vec<String>) -> Vec<String> {
    let mut cleaned: Vec<String> = Vec::new();
    for value in values {
        let text = clamp(&value, MAX_LINE_CHARS);
        if !text.is_empty() && !cleaned.iter().any(|kept| kept.eq_ignore_ascii_case(&text)) {
            cleaned.push(text);
        }
        if cleaned.len() == MAX_EXPLANATION_ITEMS {
            break;
        }
    }
    cleaned
}

#[cfg(test)]
mod tests;
