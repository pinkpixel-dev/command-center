//! Terminal error analysis. The user pastes what their terminal printed, and
//! this asks for the likely cause, the lines that carry it, and what to check
//! next.
//!
//! Two things make this different from the other workflows. The input is
//! someone else's output rather than something the user wrote, so it goes
//! through the same disclosure and redaction path Import uses before anything
//! leaves the machine. And the answer is usually incomplete by nature: pasted
//! output is a fragment, so confidence and uncertainty are part of the result
//! rather than something the UI has to infer.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::providers::{ProviderCall, ProviderClient};
use crate::ai::prompts::{self, MAX_ANALYSIS_ITEMS};
use crate::ai::proposal::{self, CommandProposal, RawProposal};
use crate::error::{AppError, AppResult};

/// A stack trace with a long dependency chain is a real paste. This is the
/// limit for leaving the machine, not for what the box will hold.
pub const MAX_OUTPUT_BYTES: usize = 32 * 1024;

/// A proposal here is a next step, not a repair kit.
const MAX_ANALYSIS_PROPOSALS: usize = 3;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(150);
/// The same reasoning floor the other workflows need: a small reasoning model
/// can spend tens of thousands of tokens before writing any JSON, and an unused
/// ceiling is not billed.
const MIN_OUTPUT_TOKENS: u32 = 25_000;
const MAX_OUTPUT_TOKENS: u32 = 48_000;
const MAX_SUMMARY_CHARS: usize = 240;
const MAX_CAUSE_CHARS: usize = 800;
const MAX_LINE_CHARS: usize = 240;
const MAX_UNCERTAINTY_CHARS: usize = 400;

/// How much the output actually supports the reading. Pasted output is usually
/// a fragment, so this is shown rather than buried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "high" => Self::High,
            "medium" => Self::Medium,
            // Anything unrecognized reads as a guess, which is the safe way to
            // be wrong about how sure the model was.
            _ => Self::Low,
        }
    }
}

/// One analysis, after local review of everything it proposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorAnalysis {
    pub summary: String,
    pub cause: String,
    pub confidence: Confidence,
    pub relevant_lines: Vec<RelevantLine>,
    pub checks: Vec<SuggestedCheck>,
    pub proposals: Vec<CommandProposal>,
    pub uncertainty: String,
}

/// A line from the paste that carries part of the diagnosis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelevantLine {
    pub line: Option<u32>,
    pub quote: String,
    pub why: String,
}

/// Something to do next, and what doing it would tell them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedCheck {
    pub check: String,
    pub why: String,
}

/// Rejects a paste that is empty or too large before any network work starts.
pub fn check_output_size(output: &str) -> AppResult<()> {
    if output.trim().is_empty() {
        return Err(AppError::invalid("Paste the terminal output first."));
    }
    if output.len() > MAX_OUTPUT_BYTES {
        return Err(AppError::invalid(format!(
            "That output is {} KB. Send at most {} KB at a time, or paste the part around the \
             failure.",
            output.len() / 1024,
            MAX_OUTPUT_BYTES / 1024
        )));
    }
    Ok(())
}

/// Numbered lines let the analysis point at the paste without the app having
/// to match text back to the source. Follow-up questions in the assistant use
/// the same numbering, so a line reference means the same thing in both.
pub fn numbered_lines(output: &str) -> String {
    let mut numbered = String::with_capacity(output.len() + output.len() / 8 + 16);
    for (index, line) in output.lines().enumerate() {
        numbered.push_str(&format!("{}|{}\n", index + 1, line));
    }
    numbered
}

/// The budget follows the paste size, with a floor sized for reasoning rather
/// than for the length of the answer.
pub fn output_token_budget(output_bytes: usize) -> u32 {
    20_000u32
        .saturating_add(output_bytes as u32)
        .clamp(MIN_OUTPUT_TOKENS, MAX_OUTPUT_TOKENS)
}

/// Builds the request body from text that has already been redacted by the
/// disclosure step, so what is sent is exactly what the user was shown.
pub fn build_input(redacted: &str) -> String {
    let mut input = String::with_capacity(redacted.len() + 128);
    input.push_str("Terminal output the user pasted:\n");
    input.push_str(&numbered_lines(redacted));
    input
}

pub async fn analyze(
    provider: &ProviderClient,
    model: &str,
    redacted: &str,
) -> AppResult<ErrorAnalysis> {
    let input = build_input(redacted);

    let output = provider
        .structured_json(
            ProviderCall {
                model,
                task: prompts::error_analysis(),
                input: &input,
                max_output_tokens: output_token_budget(redacted.len()),
                timeout: REQUEST_TIMEOUT,
            },
        )
        .await?;

    parse(&output)
}

/// One analysis exactly as the model wrote it.
#[derive(Debug, Deserialize)]
struct RawAnalysis {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    cause: String,
    #[serde(default)]
    confidence: String,
    #[serde(default)]
    relevant_lines: Vec<RawLine>,
    #[serde(default)]
    checks: Vec<RawCheck>,
    #[serde(default)]
    commands: Vec<RawProposal>,
    #[serde(default)]
    uncertainty: String,
}

#[derive(Debug, Deserialize)]
struct RawLine {
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    quote: String,
    #[serde(default)]
    why: String,
}

#[derive(Debug, Deserialize)]
struct RawCheck {
    #[serde(default)]
    check: String,
    #[serde(default)]
    why: String,
}

/// Bounds everything the model wrote and runs the local rules over every
/// command it suggested.
pub fn parse(output: &str) -> AppResult<ErrorAnalysis> {
    let raw: RawAnalysis = serde_json::from_str(output).map_err(|_| {
        AppError::AiMalformed("OpenAI returned an analysis the app could not read.".into())
    })?;

    let summary = proposal::clamp(&raw.summary, MAX_SUMMARY_CHARS);
    if summary.is_empty() {
        return Err(AppError::AiMalformed(
            "OpenAI returned an analysis with nothing in it.".into(),
        ));
    }

    Ok(ErrorAnalysis {
        summary,
        cause: proposal::clamp(&raw.cause, MAX_CAUSE_CHARS),
        confidence: Confidence::parse(&raw.confidence),
        relevant_lines: raw
            .relevant_lines
            .into_iter()
            .filter_map(|line| {
                let quote = proposal::clamp(&line.quote, MAX_LINE_CHARS);
                (!quote.is_empty()).then(|| RelevantLine {
                    line: line.line,
                    quote,
                    why: proposal::clamp(&line.why, MAX_LINE_CHARS),
                })
            })
            .take(MAX_ANALYSIS_ITEMS)
            .collect(),
        checks: raw
            .checks
            .into_iter()
            .filter_map(|check| {
                let action = proposal::clamp(&check.check, MAX_LINE_CHARS);
                (!action.is_empty()).then(|| SuggestedCheck {
                    check: action,
                    why: proposal::clamp(&check.why, MAX_LINE_CHARS),
                })
            })
            .take(MAX_ANALYSIS_ITEMS)
            .collect(),
        proposals: proposal::review_all(raw.commands, MAX_ANALYSIS_PROPOSALS),
        uncertainty: proposal::clamp(&raw.uncertainty, MAX_UNCERTAINTY_CHARS),
    })
}

#[cfg(test)]
mod tests;
