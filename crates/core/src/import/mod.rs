//! Markdown and plain-text import. Everything here runs locally: the parser,
//! the classification, and the duplicate check. Nothing is written to the
//! library until the user picks entries in the review screen.

pub mod ai_candidates;
pub mod apply;
pub mod classify;
pub mod document;
pub mod naming;
pub mod parser;

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Serialize;

use crate::db::commands as commands_db;
use crate::error::AppResult;
use crate::models::{CommandKind, RiskLevel};
use crate::normalize::{content_hash, extract_variables, normalize_command};
use crate::risk;

/// One proposed entry, ready for the review screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Stable within a single preview, so the frontend can key its list.
    pub id: String,
    pub title: String,
    pub content: String,
    pub description: String,
    pub kind: CommandKind,
    pub language: Option<String>,
    pub shell: Option<String>,
    pub tags: Vec<String>,
    pub risk_level: RiskLevel,
    pub risk_reasons: Vec<String>,
    /// Reasons the model gave, kept apart from the local ones so the review
    /// screen can label them and the user can tell them apart. Always empty for
    /// a local parse.
    pub ai_risk_reasons: Vec<String>,
    /// What the model said it was unsure about. None for a local parse.
    pub ai_notes: Option<String>,
    pub variables: Vec<String>,
    pub heading_path: Vec<String>,
    pub source_line: usize,
    /// True when this looks like pasted terminal output rather than a command.
    pub looks_like_output: bool,
    pub output_reason: Option<String>,
    /// How many output lines were stripped out of a shell session.
    pub dropped_output_lines: usize,
    /// Set when the library already holds this exact command.
    pub duplicate: Option<DuplicateMatch>,
    /// Set when an earlier candidate in this same document has the same content.
    pub repeated_in_document: bool,
    /// What the review screen should tick by default.
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateMatch {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewStats {
    pub blocks_found: usize,
    pub commands: usize,
    pub output_blocks: usize,
    pub duplicates: usize,
    pub repeated: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub source_name: Option<String>,
    pub suggested_collection: Option<String>,
    pub candidates: Vec<Candidate>,
    pub stats: PreviewStats,
}

/// Everything known about one snippet, recalculated after the user edits,
/// splits, or merges candidates in the review screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetAnalysis {
    pub kind: CommandKind,
    pub risk_level: RiskLevel,
    pub risk_reasons: Vec<String>,
    pub variables: Vec<String>,
    pub looks_like_output: bool,
    pub output_reason: Option<String>,
    pub duplicate: Option<DuplicateMatch>,
    pub suggested_title: String,
}

/// Parses a document and works out what could be imported from it.
pub fn preview(
    conn: &Connection,
    source: &str,
    source_name: Option<&str>,
) -> AppResult<ImportPreview> {
    let document = parser::parse(source);

    // A heading that covers exactly one block gets to name that block.
    let mut heading_counts: HashMap<String, usize> = HashMap::new();
    for block in &document.blocks {
        if let Some(heading) = block.heading_path.last() {
            *heading_counts.entry(heading.clone()).or_default() += 1;
        }
    }

    let mut candidates: Vec<Candidate> = Vec::new();
    let mut stats = PreviewStats {
        blocks_found: document.blocks.len(),
        ..Default::default()
    };
    let mut seen_hashes: HashMap<String, usize> = HashMap::new();

    for (index, block) in document.blocks.iter().enumerate() {
        let content = normalize_command(&block.content);
        if content.trim().is_empty() {
            continue;
        }

        let language = block.language.as_deref();
        let output = classify::detect_output(&content, language, block.from_prompt_session);
        let kind = classify::detect_kind(&content, language);
        let shell = classify::detect_shell(language, &content);
        let (risk_level, risk_reasons) = risk::assess(&content);

        let heading = block.heading_path.last().map(String::as_str);
        let exclusive = heading
            .and_then(|text| heading_counts.get(text))
            .is_some_and(|count| *count == 1);
        let title = naming::title_for(heading, exclusive, block.context.as_deref(), &content);

        let hash = content_hash(&content);
        let repeated = seen_hashes.contains_key(&hash);
        seen_hashes.insert(hash, index);

        let duplicate = commands_db::find_by_content(conn, &content)?.map(|existing| DuplicateMatch {
            id: existing.id,
            title: existing.title,
        });

        if output.is_some() {
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

        candidates.push(Candidate {
            id: format!("candidate-{index}"),
            description: naming::description_for(block.context.as_deref(), &title),
            tags: naming::tags_for(&block.heading_path, shell.as_deref(), &content),
            language: classify::detect_language(language, kind),
            variables: extract_variables(&content),
            // Output and duplicates start unticked; everything else is ready to go.
            selected: output.is_none() && duplicate.is_none() && !repeated,
            looks_like_output: output.is_some(),
            output_reason: output.map(|reason| reason.describe().to_string()),
            dropped_output_lines: block.dropped_output_lines,
            heading_path: block.heading_path.clone(),
            source_line: block.line,
            repeated_in_document: repeated,
            ai_risk_reasons: Vec::new(),
            ai_notes: None,
            risk_level,
            risk_reasons,
            duplicate,
            title,
            content,
            kind,
            shell,
        });
    }

    Ok(ImportPreview {
        suggested_collection: naming::collection_for(document.title.as_deref(), source_name),
        source_name: source_name.map(str::to_string),
        candidates,
        stats,
    })
}

/// Re-examines a single snippet. The review screen calls this after a split, a
/// merge, or an edit to the command text, so the risk label, kind, and
/// duplicate warning stay honest.
pub fn analyze(conn: &Connection, content: &str) -> AppResult<SnippetAnalysis> {
    let normalized = normalize_command(content);
    let kind = classify::detect_kind(&normalized, None);
    let output = classify::detect_output(&normalized, None, false);
    let (risk_level, risk_reasons) = risk::assess(&normalized);

    let duplicate = commands_db::find_by_content(conn, &normalized)?.map(|existing| DuplicateMatch {
        id: existing.id,
        title: existing.title,
    });

    Ok(SnippetAnalysis {
        suggested_title: naming::from_content(&normalized),
        variables: extract_variables(&normalized),
        looks_like_output: output.is_some(),
        output_reason: output.map(|reason| reason.describe().to_string()),
        risk_level,
        risk_reasons,
        duplicate,
        kind,
    })
}
