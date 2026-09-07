//! The local explanation cache. One row per entry, kept honest by the entry's
//! content hash: when the command changes, the stored explanation is still
//! readable but marked stale, and it stops feeding search until it is
//! refreshed.

use rusqlite::{params, Connection, OptionalExtension};

use crate::db::{now, search};
use crate::error::{AppError, AppResult};

/// A cached explanation as it comes back out of the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredExplanation {
    pub explanation_json: String,
    pub model: String,
    /// The command changed after this explanation was written.
    pub stale: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Everything one explanation write needs. `content_hash` is the hash of the
/// content that was actually explained, not whatever the row holds now.
pub struct NewExplanation<'a> {
    pub command_id: i64,
    pub content_hash: &'a str,
    pub model: &'a str,
    pub explanation_json: &'a str,
    pub search_text: &'a str,
}

/// Stores or replaces the explanation for one entry and reindexes it. The write
/// and the reindex share a transaction, so the cache and the search row can
/// never disagree about what the entry currently says.
pub fn save(conn: &mut Connection, record: NewExplanation<'_>) -> AppResult<StoredExplanation> {
    // An explanation for an entry that is already gone would fail on the
    // foreign key; saying so plainly is more use than a database error.
    if command_hash(conn, record.command_id)?.is_none() {
        return Err(AppError::not_found("That command"));
    }
    let timestamp = now();

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO ai_explanations
            (command_id, content_hash, model, explanation_json, search_text,
             created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
         ON CONFLICT (command_id) DO UPDATE SET
            content_hash     = excluded.content_hash,
            model            = excluded.model,
            explanation_json = excluded.explanation_json,
            search_text      = excluded.search_text,
            updated_at       = excluded.updated_at",
        params![
            record.command_id,
            record.content_hash,
            record.model,
            record.explanation_json,
            record.search_text,
            timestamp,
        ],
    )?;

    search::reindex(&tx, record.command_id)?;
    tx.commit()?;

    load(conn, record.command_id)?.ok_or_else(|| AppError::not_found("That explanation"))
}

/// Reads the cached explanation for one entry, if there is one.
pub fn load(conn: &Connection, command_id: i64) -> AppResult<Option<StoredExplanation>> {
    let stored = conn
        .query_row(
            "SELECT e.explanation_json, e.model, e.created_at, e.updated_at,
                    e.content_hash <> c.content_hash
             FROM ai_explanations e
             JOIN commands c ON c.id = e.command_id
             WHERE e.command_id = ?1",
            params![command_id],
            |row| {
                Ok(StoredExplanation {
                    explanation_json: row.get(0)?,
                    model: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    stale: row.get::<_, i64>(4)? == 1,
                })
            },
        )
        .optional()?;

    Ok(stored)
}

/// The searchable text of a *current* explanation. A stale one contributes
/// nothing, which is what keeps outdated wording out of search results.
pub fn current_search_text(conn: &Connection, command_id: i64) -> AppResult<String> {
    let text = conn
        .query_row(
            "SELECT e.search_text FROM ai_explanations e
             JOIN commands c ON c.id = e.command_id
             WHERE e.command_id = ?1 AND e.content_hash = c.content_hash",
            params![command_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    Ok(text.unwrap_or_default())
}

/// Forgets one entry's explanation. Reports whether there was one to forget.
pub fn delete(conn: &mut Connection, command_id: i64) -> AppResult<bool> {
    let tx = conn.transaction()?;
    let removed = tx.execute(
        "DELETE FROM ai_explanations WHERE command_id = ?1",
        params![command_id],
    )?;

    if removed > 0 {
        search::reindex(&tx, command_id)?;
    }
    tx.commit()?;

    Ok(removed > 0)
}

/// Empties the cache and reindexes every entry that had an explanation.
pub fn clear_all(conn: &mut Connection) -> AppResult<usize> {
    let tx = conn.transaction()?;

    let mut statement = tx.prepare("SELECT command_id FROM ai_explanations")?;
    let command_ids = statement
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<i64>, _>>()?;
    drop(statement);

    tx.execute("DELETE FROM ai_explanations", [])?;
    for command_id in &command_ids {
        search::reindex(&tx, *command_id)?;
    }
    tx.commit()?;

    Ok(command_ids.len())
}

fn command_hash(conn: &Connection, command_id: i64) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT content_hash FROM commands WHERE id = ?1",
            params![command_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::commands as commands_db;
    use crate::db::Database;
    use crate::models::CommandInput;

    fn command(db: &Database, title: &str, content: &str) -> i64 {
        let input: CommandInput = serde_json::from_value(serde_json::json!({
            "title": title,
            "content": content
        }))
        .unwrap();
        db.with_mut(|conn| commands_db::create(conn, input)).unwrap().id
    }

    fn write(db: &Database, command_id: i64, hash: &str, search_text: &str) -> StoredExplanation {
        db.with_mut(|conn| {
            save(
                conn,
                NewExplanation {
                    command_id,
                    content_hash: hash,
                    model: "gpt-test",
                    explanation_json: r#"{"summary":"Lists files"}"#,
                    search_text,
                },
            )
        })
        .unwrap()
    }

    fn current_hash(db: &Database, command_id: i64) -> String {
        db.with(|conn| Ok(command_hash(conn, command_id)?.unwrap()))
            .unwrap()
    }

    fn search_hits(db: &Database, expression: &str) -> i64 {
        db.with(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM commands_fts WHERE commands_fts MATCH ?1",
                params![expression],
                |row| row.get(0),
            )?)
        })
        .unwrap()
    }

    #[test]
    fn an_explanation_round_trips_and_is_current_for_unchanged_content() {
        let db = Database::open_in_memory().unwrap();
        let id = command(&db, "List files", "ls -la");

        let saved = write(&db, id, &current_hash(&db, id), "lists files in long form");

        assert!(!saved.stale);
        assert_eq!(saved.model, "gpt-test");
        assert_eq!(saved.created_at, saved.updated_at);
        assert_eq!(db.with(|conn| load(conn, id)).unwrap(), Some(saved));
    }

    #[test]
    fn refreshing_replaces_the_row_instead_of_adding_one() {
        let db = Database::open_in_memory().unwrap();
        let id = command(&db, "List files", "ls -la");
        let hash = current_hash(&db, id);

        write(&db, id, &hash, "first wording");
        write(&db, id, &hash, "second wording");

        let rows: i64 = db
            .with(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM ai_explanations", [], |row| row.get(0))?)
            })
            .unwrap();
        assert_eq!(rows, 1);
        assert_eq!(search_hits(&db, "\"second\"*"), 1);
        assert_eq!(search_hits(&db, "\"first\"*"), 0);
    }

    #[test]
    fn editing_the_command_marks_the_explanation_stale_and_drops_it_from_search() {
        let db = Database::open_in_memory().unwrap();
        let id = command(&db, "List files", "ls -la");
        write(&db, id, &current_hash(&db, id), "lists files in long form");
        assert_eq!(search_hits(&db, "\"long\"*"), 1);

        let edited: CommandInput = serde_json::from_value(serde_json::json!({
            "title": "List files",
            "content": "ls -lah"
        }))
        .unwrap();
        db.with_mut(|conn| commands_db::update(conn, id, edited)).unwrap();

        let stored = db.with(|conn| load(conn, id)).unwrap().unwrap();
        assert!(stored.stale, "the cached explanation is kept but marked stale");
        assert_eq!(
            search_hits(&db, "\"long\"*"),
            0,
            "stale wording stops matching"
        );
        assert_eq!(search_hits(&db, "\"ls\"*"), 1, "the entry itself still matches");
    }

    #[test]
    fn writing_an_explanation_for_content_that_already_moved_on_is_stale_immediately() {
        let db = Database::open_in_memory().unwrap();
        let id = command(&db, "List files", "ls -la");

        // The command was edited while the request was in flight.
        let stored = write(&db, id, "hash-from-an-older-request", "lists files");

        assert!(stored.stale);
        assert_eq!(db.with(|conn| current_search_text(conn, id)).unwrap(), "");
    }

    #[test]
    fn deleting_the_command_deletes_its_explanation() {
        let db = Database::open_in_memory().unwrap();
        let id = command(&db, "List files", "ls -la");
        write(&db, id, &current_hash(&db, id), "lists files");

        db.with_mut(|conn| commands_db::delete(conn, id)).unwrap();

        assert_eq!(db.with(|conn| load(conn, id)).unwrap(), None);
        let rows: i64 = db
            .with(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM ai_explanations", [], |row| row.get(0))?)
            })
            .unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn an_explanation_cannot_be_stored_for_a_missing_command() {
        let db = Database::open_in_memory().unwrap();
        let result = db.with_mut(|conn| {
            save(
                conn,
                NewExplanation {
                    command_id: 404,
                    content_hash: "hash",
                    model: "gpt-test",
                    explanation_json: "{}",
                    search_text: "text",
                },
            )
        });

        assert_eq!(result.unwrap_err().kind(), "not_found");
    }

    #[test]
    fn clearing_the_cache_empties_it_and_reindexes_every_entry() {
        let db = Database::open_in_memory().unwrap();
        let first = command(&db, "List files", "ls -la");
        let second = command(&db, "Show status", "git status");
        write(&db, first, &current_hash(&db, first), "lists files");
        write(&db, second, &current_hash(&db, second), "shows the worktree");

        let cleared = db.with_mut(clear_all).unwrap();

        assert_eq!(cleared, 2);
        assert_eq!(db.with(|conn| load(conn, first)).unwrap(), None);
        assert_eq!(search_hits(&db, "\"worktree\"*"), 0);
        assert_eq!(search_hits(&db, "\"status\"*"), 1, "entries stay searchable");
        assert_eq!(db.with_mut(clear_all).unwrap(), 0, "clearing twice is harmless");
    }

    #[test]
    fn deleting_one_explanation_reports_whether_there_was_one() {
        let db = Database::open_in_memory().unwrap();
        let id = command(&db, "List files", "ls -la");
        write(&db, id, &current_hash(&db, id), "lists files");

        assert!(db.with_mut(|conn| delete(conn, id)).unwrap());
        assert!(!db.with_mut(|conn| delete(conn, id)).unwrap());
        assert_eq!(search_hits(&db, "\"lists\"*"), 0);
    }
}
