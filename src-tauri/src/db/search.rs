//! Full-text search plumbing. The FTS5 table is maintained by hand instead of
//! by triggers because a searchable row also needs the entry's tags and
//! collections, which live in join tables.

use rusqlite::{params, Connection};

use crate::error::AppResult;

/// Rebuilds the search row for one entry. Safe to call for an id that has no
/// row yet, and safe to call repeatedly.
pub fn reindex(conn: &Connection, command_id: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM commands_fts WHERE command_id = ?1",
        params![command_id],
    )?;

    let row = conn.query_row(
        "SELECT title, content, description, notes FROM commands WHERE id = ?1",
        params![command_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        },
    );

    let (title, content, description, notes) = match row {
        Ok(values) => values,
        // The entry was deleted; removing its search row was the whole job.
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    let tags = joined(
        conn,
        "SELECT t.name FROM command_tags ct
         JOIN tags t ON t.id = ct.tag_id
         WHERE ct.command_id = ?1 ORDER BY t.name",
        command_id,
    )?;

    let collections = joined(
        conn,
        "SELECT c.name FROM command_collections cc
         JOIN collections c ON c.id = cc.collection_id
         WHERE cc.command_id = ?1 ORDER BY c.name",
        command_id,
    )?;

    conn.execute(
        "INSERT INTO commands_fts
            (title, content, description, notes, tags, collections, command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![title, content, description, notes, tags, collections, command_id],
    )?;

    Ok(())
}

/// Drops the search row for an entry that is going away.
pub fn remove(conn: &Connection, command_id: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM commands_fts WHERE command_id = ?1",
        params![command_id],
    )?;
    Ok(())
}

fn joined(conn: &Connection, sql: &str, command_id: i64) -> AppResult<String> {
    let mut statement = conn.prepare(sql)?;
    let values = statement
        .query_map(params![command_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(values.join(" "))
}

/// Turns whatever the user typed into a safe FTS5 MATCH expression.
///
/// Every token is quoted so punctuation-heavy searches like `rm -rf` or
/// `docker:compose` cannot become syntax errors, and each token gets a prefix
/// wildcard so results appear while typing. Returns `None` when there is
/// nothing searchable, which callers treat as "no text filter".
pub fn build_match_query(raw: &str) -> Option<String> {
    let tokens: Vec<String> = raw
        .split_whitespace()
        .filter_map(|token| {
            let cleaned: String = token
                .chars()
                .filter(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-' | '/'))
                .collect();
            let usable = cleaned.trim_matches(|c: char| !c.is_alphanumeric());
            if usable.chars().any(char::is_alphanumeric) {
                Some(format!("\"{}\"*", usable.replace('"', "")))
            } else {
                None
            }
        })
        .collect();

    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" AND "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_words_become_prefix_terms() {
        assert_eq!(
            build_match_query("docker cleanup"),
            Some("\"docker\"* AND \"cleanup\"*".to_string())
        );
    }

    #[test]
    fn punctuation_is_neutralized() {
        assert_eq!(build_match_query("rm -rf"), Some("\"rm\"* AND \"rf\"*".to_string()));
        assert_eq!(
            build_match_query("docker:compose"),
            Some("\"dockercompose\"*".to_string())
        );
        assert_eq!(build_match_query("\"quoted\""), Some("\"quoted\"*".to_string()));
    }

    #[test]
    fn tokens_with_no_letters_or_digits_are_dropped() {
        assert_eq!(build_match_query("git -- ***"), Some("\"git\"*".to_string()));
        assert_eq!(build_match_query("   "), None);
        assert_eq!(build_match_query("^*()"), None);
    }

    #[test]
    fn numbers_and_paths_survive() {
        assert_eq!(build_match_query("port 3000"), Some("\"port\"* AND \"3000\"*".to_string()));
        assert_eq!(
            build_match_query("/etc/hosts"),
            Some("\"etc/hosts\"*".to_string())
        );
    }
}
