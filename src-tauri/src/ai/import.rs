//! Structured import extraction. This module builds the request from an
//! already redacted document and hands back sanitized items. It makes no
//! judgement about risk, duplicates, or what may be saved: the local import
//! pipeline decides all of that afterwards.

use std::time::Duration;

use serde::Deserialize;

use crate::ai::client::{OpenAiClient, StructuredCall};
use crate::ai::prompts::{self, MAX_IMPORT_ITEMS};
use crate::error::{AppError, AppResult};

/// Bigger than any cheat sheet worth sending in one request. The local import
/// file limit stays 4 MB; this is the limit for leaving the machine.
pub const MAX_DOCUMENT_BYTES: usize = 64 * 1024;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);
const MIN_OUTPUT_TOKENS: u32 = 2_000;
const MAX_OUTPUT_TOKENS: u32 = 12_000;
const MAX_TITLE_CHARS: usize = 120;
const MAX_DESCRIPTION_CHARS: usize = 300;
const MAX_TAGS: usize = 5;
const MAX_TAG_CHARS: usize = 32;
const MAX_REASONS: usize = 5;
const MAX_REASON_CHARS: usize = 160;

/// One proposed item exactly as the model described it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AiImportItem {
    pub content: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_line: Option<u32>,
    #[serde(default)]
    pub looks_like_output: bool,
    #[serde(default)]
    pub risk_suggestion: String,
    #[serde(default)]
    pub risk_reasons: Vec<String>,
    #[serde(default)]
    pub uncertainty: String,
}

#[derive(Debug, Deserialize)]
struct ExtractionOutput {
    #[serde(default)]
    items: Vec<AiImportItem>,
}

/// Rejects a document that is too large to send before any network work starts.
pub fn check_document_size(document: &str) -> AppResult<()> {
    if document.trim().is_empty() {
        return Err(AppError::invalid("There is nothing in that document to read."));
    }
    if document.len() > MAX_DOCUMENT_BYTES {
        return Err(AppError::invalid(format!(
            "That document is {} KB. Send at most {} KB at a time.",
            document.len() / 1024,
            MAX_DOCUMENT_BYTES / 1024
        )));
    }
    Ok(())
}

/// Numbered lines let the model report where an item came from without the app
/// having to guess by matching text back to the source.
pub fn build_input(redacted: &str, source_name: Option<&str>) -> String {
    let mut input = String::with_capacity(redacted.len() + redacted.len() / 8 + 64);
    match source_name {
        Some(name) => input.push_str(&format!("Document: {name}\n\n")),
        None => input.push_str("Document: pasted text\n\n"),
    }
    for (index, line) in redacted.lines().enumerate() {
        input.push_str(&format!("{}|{}\n", index + 1, line));
    }
    input
}

/// Long documents produce more items, so the budget follows the source size
/// instead of a single guess that is wrong at both ends.
pub fn output_token_budget(document_bytes: usize) -> u32 {
    let estimate = 1_500u32.saturating_add((document_bytes / 4) as u32);
    estimate.clamp(MIN_OUTPUT_TOKENS, MAX_OUTPUT_TOKENS)
}

pub async fn extract(
    client: &OpenAiClient,
    api_key: &str,
    model: &str,
    redacted: &str,
    source_name: Option<&str>,
) -> AppResult<Vec<AiImportItem>> {
    let input = build_input(redacted, source_name);
    let output = client
        .structured_json(
            api_key,
            StructuredCall {
                model,
                task: prompts::import_extraction(),
                input: &input,
                max_output_tokens: output_token_budget(redacted.len()),
                timeout: REQUEST_TIMEOUT,
            },
        )
        .await?;

    parse_items(&output)
}

pub fn parse_items(output: &str) -> AppResult<Vec<AiImportItem>> {
    let parsed: ExtractionOutput = serde_json::from_str(output).map_err(|_| {
        AppError::AiMalformed("OpenAI returned extraction output the app could not read.".into())
    })?;

    Ok(parsed
        .items
        .into_iter()
        .filter_map(sanitize)
        .take(MAX_IMPORT_ITEMS)
        .collect())
}

/// Trims and bounds everything the model wrote. Command text itself is left
/// alone here; the local pipeline normalizes it next.
fn sanitize(item: AiImportItem) -> Option<AiImportItem> {
    let content = item.content.trim_end().to_owned();
    if content.trim().is_empty() {
        return None;
    }

    Some(AiImportItem {
        title: clamp_text(&item.title, MAX_TITLE_CHARS),
        description: clamp_text(&item.description, MAX_DESCRIPTION_CHARS),
        kind: item.kind.trim().to_lowercase(),
        shell: clamp_optional(item.shell, MAX_TAG_CHARS),
        language: clamp_optional(item.language, MAX_TAG_CHARS),
        tags: clamp_list(item.tags, MAX_TAGS, MAX_TAG_CHARS),
        risk_suggestion: item.risk_suggestion.trim().to_lowercase(),
        risk_reasons: clamp_list(item.risk_reasons, MAX_REASONS, MAX_REASON_CHARS),
        uncertainty: clamp_text(&item.uncertainty, MAX_DESCRIPTION_CHARS),
        source_line: item.source_line,
        looks_like_output: item.looks_like_output,
        content,
    })
}

fn clamp_text(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

fn clamp_optional(value: Option<String>, limit: usize) -> Option<String> {
    let cleaned = clamp_text(value.as_deref().unwrap_or_default(), limit);
    (!cleaned.is_empty()).then_some(cleaned)
}

fn clamp_list(values: Vec<String>, count: usize, chars: usize) -> Vec<String> {
    let mut cleaned: Vec<String> = Vec::new();
    for value in values {
        let text = clamp_text(&value, chars);
        if !text.is_empty() && !cleaned.iter().any(|kept| kept.eq_ignore_ascii_case(&text)) {
            cleaned.push(text);
        }
        if cleaned.len() == count {
            break;
        }
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn documents_are_size_checked_before_any_request() {
        assert!(check_document_size("git status").is_ok());
        assert_eq!(check_document_size("   ").unwrap_err().kind(), "invalid");

        let oversized = "x".repeat(MAX_DOCUMENT_BYTES + 1);
        let error = check_document_size(&oversized).unwrap_err();
        assert_eq!(error.kind(), "invalid");
        assert!(error.to_string().contains("64 KB"));
    }

    #[test]
    fn input_carries_line_numbers_and_only_the_redacted_text() {
        let input = build_input("git status\npassword={{PASSWORD}}", Some("notes.md"));

        assert!(input.starts_with("Document: notes.md"));
        assert!(input.contains("1|git status\n"));
        assert!(input.contains("2|password={{PASSWORD}}\n"));
        assert!(!input.contains("hunter2"));
    }

    #[test]
    fn the_token_budget_follows_the_document_size_within_bounds() {
        assert_eq!(output_token_budget(0), MIN_OUTPUT_TOKENS);
        assert_eq!(output_token_budget(16 * 1024), 1_500 + 4_096);
        assert_eq!(output_token_budget(MAX_DOCUMENT_BYTES), MAX_OUTPUT_TOKENS);
    }

    #[test]
    fn items_are_trimmed_bounded_and_deduplicated() {
        let output = json!({
            "items": [{
                "content": "  git status   ",
                "title": "  Show status  ",
                "description": "",
                "kind": " Command ",
                "shell": "  ",
                "language": null,
                "tags": ["git", "GIT", "status", "vcs", "cli", "extra", "sixth"],
                "source_line": 4,
                "looks_like_output": false,
                "risk_suggestion": "Safe",
                "risk_reasons": [],
                "uncertainty": ""
            }, {
                "content": "   ",
                "title": "empty",
                "description": "",
                "kind": "command",
                "shell": null,
                "language": null,
                "tags": [],
                "source_line": null,
                "looks_like_output": false,
                "risk_suggestion": "safe",
                "risk_reasons": [],
                "uncertainty": ""
            }]
        })
        .to_string();

        let items = parse_items(&output).unwrap();

        assert_eq!(items.len(), 1, "empty content is dropped");
        assert_eq!(items[0].content, "  git status");
        assert_eq!(items[0].title, "Show status");
        assert_eq!(items[0].kind, "command");
        assert_eq!(items[0].shell, None);
        assert_eq!(items[0].tags, ["git", "status", "vcs", "cli", "extra"]);
        assert_eq!(items[0].risk_suggestion, "safe");
        assert_eq!(items[0].source_line, Some(4));
    }

    #[test]
    fn malformed_output_is_an_error_rather_than_a_partial_import() {
        assert_eq!(
            parse_items("not json").unwrap_err().kind(),
            "ai_malformed"
        );
        assert_eq!(
            parse_items(r#"{"items":[{"title":"no content"}]}"#)
                .unwrap_err()
                .kind(),
            "ai_malformed"
        );
    }

    #[test]
    fn the_item_ceiling_is_enforced_whatever_the_model_returns() {
        let items: Vec<serde_json::Value> = (0..MAX_IMPORT_ITEMS + 25)
            .map(|index| {
                json!({
                    "content": format!("echo {index}"),
                    "title": "",
                    "description": "",
                    "kind": "command",
                    "shell": null,
                    "language": null,
                    "tags": [],
                    "source_line": null,
                    "looks_like_output": false,
                    "risk_suggestion": "safe",
                    "risk_reasons": [],
                    "uncertainty": ""
                })
            })
            .collect();

        let parsed = parse_items(&json!({ "items": items }).to_string()).unwrap();
        assert_eq!(parsed.len(), MAX_IMPORT_ITEMS);
    }
}
