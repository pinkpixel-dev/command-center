//! Human-readable exports and consistent copies of the SQLite library.

use std::fmt::Write as _;
use std::path::Path;

use crate::db::{commands, Database};
use crate::error::{AppError, AppResult};
use crate::models::Command;

pub fn markdown(entries: &[Command], exported_at: &str) -> String {
    let mut output = String::from("# Command Center library\n\n");
    let _ = writeln!(output, "Exported {exported_at}.\n");

    if entries.is_empty() {
        output.push_str("No saved entries.\n");
        return output;
    }

    for entry in entries {
        let _ = writeln!(output, "## {}\n", entry.title);

        if !entry.description.is_empty() {
            let _ = writeln!(output, "{}\n", entry.description);
        }

        let _ = writeln!(output, "- Type: {}", entry.kind.as_str());
        if let Some(shell) = &entry.shell {
            let _ = writeln!(output, "- Shell: {shell}");
        }
        if let Some(os) = &entry.operating_system {
            let _ = writeln!(output, "- Operating system: {os}");
        }
        if !entry.tags.is_empty() {
            let _ = writeln!(output, "- Tags: {}", entry.tags.join(", "));
        }
        if !entry.collections.is_empty() {
            let names = entry
                .collections
                .iter()
                .map(|collection| collection.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(output, "- Collections: {names}");
        }

        let fence = markdown_fence(&entry.content);
        let language = entry
            .language
            .as_deref()
            .or(entry.shell.as_deref())
            .unwrap_or("");
        let _ = writeln!(output, "\n{fence}{language}");
        output.push_str(&entry.content);
        if !entry.content.ends_with('\n') {
            output.push('\n');
        }
        let _ = writeln!(output, "{fence}\n");

        if !entry.notes.is_empty() {
            let _ = writeln!(output, "### Notes\n\n{}\n", entry.notes);
        }
        if let Some(source) = &entry.source_url {
            let _ = writeln!(output, "Source: {source}\n");
        }
    }

    output
}

fn markdown_fence(content: &str) -> String {
    let longest = content
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    "`".repeat((longest + 1).max(3))
}

pub fn export_markdown(db: &Database, destination: &Path) -> AppResult<()> {
    validate_destination(destination, &["md", "markdown"])?;
    let entries = db.with(commands::list_all)?;
    let timestamp = chrono::Utc::now().format("%B %-d, %Y at %H:%M UTC");
    let content = markdown(&entries, &timestamp.to_string());
    std::fs::write(destination, content)
        .map_err(|error| AppError::runtime(format!("could not write Markdown export: {error}")))
}

pub fn backup_database(db: &Database, source: &Path, destination: &Path) -> AppResult<()> {
    validate_destination(destination, &["db", "sqlite", "sqlite3"])?;

    if same_path(source, destination) {
        return Err(AppError::invalid(
            "Choose a different file so the active library is not overwritten",
        ));
    }

    db.with(|connection| {
        connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        std::fs::copy(source, destination).map_err(|error| {
            AppError::runtime(format!("could not back up the library: {error}"))
        })?;
        Ok(())
    })
}

fn validate_destination(path: &Path, extensions: &[&str]) -> AppResult<()> {
    let extension = path.extension().and_then(|value| value.to_str());
    if extension.is_some_and(|value| {
        extensions
            .iter()
            .any(|candidate| value.eq_ignore_ascii_case(candidate))
    }) {
        return Ok(());
    }

    Err(AppError::invalid(format!(
        "Choose a file ending in {}",
        extensions
            .iter()
            .map(|extension| format!(".{extension}"))
            .collect::<Vec<_>>()
            .join(", "),
    )))
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommandInput, CommandKind};

    fn input(title: &str, content: &str) -> CommandInput {
        CommandInput {
            title: title.into(),
            content: content.into(),
            description: "A useful command".into(),
            kind: CommandKind::Command,
            language: None,
            shell: Some("bash".into()),
            operating_system: Some("linux".into()),
            risk_level: None,
            favorite: false,
            working_directory: None,
            source_url: Some("https://example.com/docs".into()),
            notes: "Check the target first.".into(),
            tags: vec!["ssh".into()],
            collection_ids: vec![],
        }
    }

    #[test]
    fn markdown_contains_complete_entries_and_safe_fences() {
        let db = Database::open_in_memory().unwrap();
        let entry = db
            .with_mut(|connection| {
                commands::create(connection, input("SSH in", "echo '```'\nssh host"))
            })
            .unwrap();

        let rendered = markdown(&[entry], "July 25, 2026");
        assert!(rendered.contains("# Command Center library"));
        assert!(rendered.contains("## SSH in"));
        assert!(rendered.contains("````bash\necho '```'\nssh host\n````"));
        assert!(rendered.contains("Check the target first."));
        assert!(rendered.contains("https://example.com/docs"));
    }

    #[test]
    fn markdown_export_and_database_backup_write_real_files() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("library.db");
        let export = dir.path().join("library.md");
        let backup = dir.path().join("library-backup.db");
        let db = Database::open(&source).unwrap();

        db.with_mut(|connection| commands::create(connection, input("List", "ls -la")))
            .unwrap();

        export_markdown(&db, &export).unwrap();
        backup_database(&db, &source, &backup).unwrap();

        assert!(std::fs::read_to_string(export).unwrap().contains("## List"));
        let restored = Database::open(&backup).unwrap();
        assert_eq!(restored.with(commands::stats).unwrap().total, 1);
    }

    #[test]
    fn backup_refuses_to_overwrite_the_active_library() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("library.db");
        let db = Database::open(&source).unwrap();

        let error = backup_database(&db, &source, &source).unwrap_err();
        assert_eq!(error.kind(), "invalid");
    }
}
