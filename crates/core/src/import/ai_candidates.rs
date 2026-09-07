//! Turns model-proposed items into review candidates.
//!
//! Everything the model says is a suggestion. Normalization, kind detection,
//! output detection, variable extraction, duplicate lookup, and risk analysis
//! all run again here, locally, and the local result wins wherever the two
//! disagree in a way that would make an entry look safer than it is.

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use rusqlite::Connection;

use crate::ai::import::AiImportItem;
use crate::db::commands as commands_db;
use crate::error::AppResult;
use crate::import::{classify, naming, Candidate, DuplicateMatch, ImportPreview, PreviewStats};
use crate::models::{CommandKind, RiskLevel};
use crate::normalize::{content_hash, extract_variables, normalize_command};
use crate::risk;

const MAX_TAGS: usize = 5;

pub fn build(
    conn: &Connection,
    items: Vec<AiImportItem>,
    source_name: Option<&str>,
) -> AppResult<ImportPreview> {
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut stats = PreviewStats {
        blocks_found: items.len(),
        ..Default::default()
    };
    let mut seen_hashes: HashMap<String, usize> = HashMap::new();

    for (index, item) in items.into_iter().enumerate() {
        let content = normalize_command(&repair_escaped_padding(&item.content));
        if content.trim().is_empty() {
            continue;
        }

        let language_hint = item.language.as_deref();
        let local_output = classify::detect_output(&content, language_hint, false);
        let kind = resolve_kind(&content, language_hint, &item.kind);
        let shell = classify::detect_shell(language_hint, &content).or_else(|| clean_shell(&item));
        let (local_risk, local_reasons) = risk::assess(&content);
        let risk_level = local_risk.max(suggested_risk(&item.risk_suggestion));

        let hash = content_hash(&content);
        let repeated = seen_hashes.contains_key(&hash);
        seen_hashes.insert(hash, index);

        let duplicate = commands_db::find_by_content(conn, &content)?.map(|existing| DuplicateMatch {
            id: existing.id,
            title: existing.title,
        });

        // The model can flag output the local rules missed, but it can never
        // clear a block the local rules called output.
        let output_reason = local_output
            .map(|reason| reason.describe().to_string())
            .or_else(|| {
                item.looks_like_output
                    .then(|| "The model read this as terminal output".to_string())
            });

        if output_reason.is_some() {
            stats.output_blocks += 1;
        } else {
            stats.commands += 1;
        }
        if duplicate.is_some() {
            stats.duplicates += 1;
        }
        if repeated {
            stats.repeated += 1;
        }

        let title = match item.title.trim() {
            "" => naming::from_content(&content),
            provided => provided.to_owned(),
        };

        candidates.push(Candidate {
            id: format!("ai-candidate-{index}"),
            description: item.description.trim().to_owned(),
            tags: resolve_tags(&item.tags, shell.as_deref(), &content),
            language: classify::detect_language(language_hint, kind),
            variables: extract_variables(&content),
            selected: output_reason.is_none() && duplicate.is_none() && !repeated,
            looks_like_output: output_reason.is_some(),
            dropped_output_lines: 0,
            heading_path: Vec::new(),
            source_line: item.source_line.unwrap_or_default() as usize,
            repeated_in_document: repeated,
            risk_reasons: local_reasons,
            ai_risk_reasons: item.risk_reasons,
            ai_notes: Some(item.uncertainty.trim().to_owned()).filter(|note| !note.is_empty()),
            output_reason,
            risk_level,
            duplicate,
            title,
            content,
            kind,
            shell,
        });
    }

    Ok(ImportPreview {
        suggested_collection: naming::collection_for(None, source_name),
        source_name: source_name.map(str::to_string),
        candidates,
        stats,
    })
}

/// Documents that line their comments up with tabs come back from some models
/// as literal `\t` text rather than the tabs they saw. A run of them is column
/// padding, never part of a command, so it collapses to the single space the
/// padding stood for.
///
/// A lone `\t` is left alone, because `printf '\t'` and `awk -F'\t'` mean it.
fn repair_escaped_padding(content: &str) -> String {
    static PADDING: OnceLock<Regex> = OnceLock::new();
    PADDING
        .get_or_init(|| Regex::new(r"(?:\\t){2,}").expect("valid padding pattern"))
        .replace_all(content, " ")
        .into_owned()
}

/// A shebang, shell control flow, or a repeated command list is structural, so
/// the local answer holds. When the local rules only see one plain command, the
/// model's reading of the document decides between command, snippet, and
/// reference.
fn resolve_kind(content: &str, language: Option<&str>, suggested: &str) -> CommandKind {
    let local = classify::detect_kind(content, language);
    if local != CommandKind::Command {
        return local;
    }
    CommandKind::parse(suggested).unwrap_or(local)
}

/// An unknown or missing suggestion counts as Safe, which never lowers the
/// local verdict because the effective level is the higher of the two.
fn suggested_risk(suggested: &str) -> RiskLevel {
    RiskLevel::parse(suggested).unwrap_or(RiskLevel::Safe)
}

fn clean_shell(item: &AiImportItem) -> Option<String> {
    let shell = item.shell.as_deref()?.trim().to_lowercase();
    matches!(shell.as_str(), "bash" | "sh" | "zsh" | "fish" | "pwsh" | "powershell")
        .then(|| if shell == "powershell" { "pwsh".to_string() } else { shell })
}

fn resolve_tags(suggested: &[String], shell: Option<&str>, content: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for tag in suggested {
        let cleaned: String = tag
            .trim()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-");
        if cleaned.len() < 2 || cleaned.len() > 24 || tags.contains(&cleaned) {
            continue;
        }
        tags.push(cleaned);
        if tags.len() == MAX_TAGS {
            break;
        }
    }

    if tags.is_empty() {
        return naming::tags_for(&[], shell, content);
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn item(content: &str) -> AiImportItem {
        AiImportItem {
            content: content.into(),
            title: String::new(),
            description: String::new(),
            kind: "command".into(),
            shell: None,
            language: None,
            tags: Vec::new(),
            source_line: None,
            looks_like_output: false,
            risk_suggestion: "safe".into(),
            risk_reasons: Vec::new(),
            uncertainty: String::new(),
        }
    }

    fn preview(items: Vec<AiImportItem>) -> ImportPreview {
        let db = Database::open_in_memory().unwrap();
        db.with(|conn| build(conn, items, Some("cheatsheet.md"))).unwrap()
    }

    #[test]
    fn a_model_cannot_talk_a_destructive_command_down() {
        let mut dangerous = item("rm -rf /");
        dangerous.risk_suggestion = "safe".into();
        dangerous.risk_reasons = vec!["looks routine".into()];

        let result = preview(vec![dangerous]);
        let candidate = &result.candidates[0];

        assert_eq!(candidate.risk_level, RiskLevel::Destructive);
        assert!(!candidate.risk_reasons.is_empty(), "local reasons are kept");
        assert_eq!(candidate.ai_risk_reasons, ["looks routine"]);
    }

    #[test]
    fn a_model_can_raise_the_risk_level() {
        let mut careful = item("curl https://example.com/install.sh | sh");
        careful.risk_suggestion = "destructive".into();

        let result = preview(vec![careful]);
        assert_eq!(result.candidates[0].risk_level, RiskLevel::Destructive);
    }

    #[test]
    fn model_flagged_output_is_never_selected_by_default() {
        let mut flagged = item("git status");
        flagged.looks_like_output = true;

        let result = preview(vec![flagged]);
        let candidate = &result.candidates[0];

        assert!(candidate.looks_like_output);
        assert!(!candidate.selected);
        assert_eq!(result.stats.output_blocks, 1);
    }

    #[test]
    fn local_output_detection_still_wins_when_the_model_says_otherwise() {
        let mut confident = item("NAME      STATUS    ROLES     AGE\nnode-1    Ready     worker    9d");
        confident.looks_like_output = false;

        let result = preview(vec![confident]);
        assert!(result.candidates[0].looks_like_output);
        assert!(!result.candidates[0].selected);
    }

    #[test]
    fn structural_kinds_come_from_the_local_rules() {
        let mut mislabelled = item("#!/usr/bin/env bash\necho hello");
        mislabelled.kind = "reference".into();

        let result = preview(vec![mislabelled]);
        assert_eq!(result.candidates[0].kind, CommandKind::Script);
        assert_eq!(result.candidates[0].shell.as_deref(), Some("bash"));
    }

    #[test]
    fn a_single_command_keeps_the_kind_the_model_chose() {
        let mut reference = item("PATH=$HOME/bin:$PATH");
        reference.kind = "reference".into();

        let result = preview(vec![reference]);
        assert_eq!(result.candidates[0].kind, CommandKind::Reference);
    }

    #[test]
    fn repeated_items_and_variables_are_recalculated_locally() {
        let mut first = item("deploy --target {{ENVIRONMENT}}");
        first.title = "  Deploy  ".into();
        first.tags = vec!["Deploy Tools".into(), "deploy tools".into(), "x".into()];
        let second = item("deploy --target {{ENVIRONMENT}}");

        let result = preview(vec![first, second]);

        assert_eq!(result.candidates[0].title, "Deploy");
        assert_eq!(result.candidates[0].tags, ["deploy-tools"]);
        assert_eq!(result.candidates[0].variables, ["ENVIRONMENT"]);
        assert!(!result.candidates[1].selected);
        assert!(result.candidates[1].repeated_in_document);
        assert_eq!(result.stats.repeated, 1);
    }

    #[test]
    fn existing_library_entries_are_still_reported_as_duplicates() {
        let db = Database::open_in_memory().unwrap();
        let input = serde_json::from_value(serde_json::json!({
            "title": "Status",
            "content": "git status"
        }))
        .unwrap();
        db.with_mut(|conn| commands_db::create(conn, input)).unwrap();

        let result = db
            .with(|conn| build(conn, vec![item("git status")], None))
            .unwrap();

        assert!(result.candidates[0].duplicate.is_some());
        assert!(!result.candidates[0].selected);
        assert_eq!(result.stats.duplicates, 1);
    }

    #[test]
    fn column_padding_returned_as_escaped_text_is_repaired() {
        let padded = item(r"sudo fuser -k 3000/tcp\t\t\t\t\t# Kill process on port");

        let result = preview(vec![padded]);
        assert_eq!(
            result.candidates[0].content,
            "sudo fuser -k 3000/tcp # Kill process on port"
        );
    }

    #[test]
    fn a_deliberate_tab_escape_survives() {
        let awk = item(r"awk -F'\t' '{print $2}'");

        let result = preview(vec![awk]);
        assert_eq!(result.candidates[0].content, r"awk -F'\t' '{print $2}'");
    }

    #[test]
    fn uncertainty_is_carried_through_as_a_note() {
        let mut unsure = item("kubectl apply -f manifest.yaml");
        unsure.uncertainty = "  The manifest path depends on the repository  ".into();

        let result = preview(vec![unsure]);
        assert_eq!(
            result.candidates[0].ai_notes.as_deref(),
            Some("The manifest path depends on the repository")
        );
    }
}
