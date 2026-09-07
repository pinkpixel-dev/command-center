//! Writing reviewed candidates into the library.
//!
//! Each item is applied on its own, so one bad entry cannot take the whole
//! import down with it. What failed comes back in the summary.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::commands as commands_db;
use crate::error::AppResult;
use crate::models::CommandInput;

/// What to do when the library already holds the same command.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DuplicateAction {
    /// Save it anyway, as a second entry.
    #[default]
    Create,
    /// Leave the library alone.
    Skip,
    /// Overwrite the existing entry with the imported version.
    Replace,
    /// Keep the existing entry, but fold in the new tags, collections, and any
    /// description or notes it was missing.
    Merge,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItem {
    pub input: CommandInput,
    #[serde(default)]
    pub duplicate_action: DuplicateAction,
    /// Which existing entry `Replace` and `Merge` act on.
    #[serde(default)]
    pub existing_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFailure {
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub created: usize,
    pub replaced: usize,
    pub merged: usize,
    pub skipped: usize,
    pub failures: Vec<ImportFailure>,
}

impl ImportSummary {
    pub fn touched(&self) -> usize {
        self.created + self.replaced + self.merged
    }
}

pub fn run(conn: &mut Connection, items: Vec<ImportItem>) -> AppResult<ImportSummary> {
    let mut summary = ImportSummary::default();

    for item in items {
        let title = item.input.title.clone();
        let outcome = apply_one(conn, item);

        match outcome {
            Ok(Applied::Created) => summary.created += 1,
            Ok(Applied::Replaced) => summary.replaced += 1,
            Ok(Applied::Merged) => summary.merged += 1,
            Ok(Applied::Skipped) => summary.skipped += 1,
            Err(error) => summary.failures.push(ImportFailure {
                title: if title.trim().is_empty() {
                    "Untitled entry".to_string()
                } else {
                    title
                },
                message: error.to_string(),
            }),
        }
    }

    Ok(summary)
}

enum Applied {
    Created,
    Replaced,
    Merged,
    Skipped,
}

fn apply_one(conn: &mut Connection, item: ImportItem) -> AppResult<Applied> {
    match (item.duplicate_action, item.existing_id) {
        (DuplicateAction::Skip, _) => Ok(Applied::Skipped),

        (DuplicateAction::Replace, Some(id)) => {
            commands_db::update(conn, id, item.input)?;
            Ok(Applied::Replaced)
        }

        (DuplicateAction::Merge, Some(id)) => {
            let existing = commands_db::get(conn, id)?;
            let merged = merge_inputs(existing.to_input(), item.input);
            commands_db::update(conn, id, merged)?;
            Ok(Applied::Merged)
        }

        // Replace or Merge without a target is just a create.
        _ => {
            commands_db::create(conn, item.input)?;
            Ok(Applied::Created)
        }
    }
}

/// Keeps the stored command as the source of truth and adds what the import
/// knows that the library did not.
fn merge_inputs(mut existing: CommandInput, incoming: CommandInput) -> CommandInput {
    for tag in incoming.tags {
        if !existing.tags.contains(&tag) {
            existing.tags.push(tag);
        }
    }
    existing.tags.sort();

    for collection_id in incoming.collection_ids {
        if !existing.collection_ids.contains(&collection_id) {
            existing.collection_ids.push(collection_id);
        }
    }
    existing.collection_ids.sort_unstable();

    if existing.description.trim().is_empty() {
        existing.description = incoming.description;
    }

    if existing.notes.trim().is_empty() {
        existing.notes = incoming.notes;
    } else if !incoming.notes.trim().is_empty()
        && !existing.notes.contains(incoming.notes.trim())
    {
        existing.notes = format!("{}\n\n{}", existing.notes.trim_end(), incoming.notes.trim());
    }

    existing.source_url = existing.source_url.or(incoming.source_url);
    existing.shell = existing.shell.or(incoming.shell);
    existing.operating_system = existing.operating_system.or(incoming.operating_system);
    existing.language = existing.language.or(incoming.language);

    existing
}
