//! Schema versions. Each entry runs once, in order, inside a transaction, and
//! `PRAGMA user_version` records how far the database has come.

use rusqlite::Connection;

use crate::error::AppResult;

/// Add new schema steps to the end of this list. Never edit an existing one.
const MIGRATIONS: &[&str] = &[
    // 1 - initial library schema
    r#"
    CREATE TABLE commands (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        title             TEXT    NOT NULL,
        content           TEXT    NOT NULL,
        description       TEXT    NOT NULL DEFAULT '',
        kind              TEXT    NOT NULL DEFAULT 'command',
        language          TEXT,
        shell             TEXT,
        operating_system  TEXT,
        risk_level        TEXT    NOT NULL DEFAULT 'safe',
        favorite          INTEGER NOT NULL DEFAULT 0,
        working_directory TEXT,
        source_url        TEXT,
        notes             TEXT    NOT NULL DEFAULT '',
        content_hash      TEXT    NOT NULL,
        copy_count        INTEGER NOT NULL DEFAULT 0,
        last_copied_at    TEXT,
        created_at        TEXT    NOT NULL,
        updated_at        TEXT    NOT NULL
    );

    CREATE INDEX idx_commands_updated  ON commands (updated_at DESC);
    CREATE INDEX idx_commands_favorite ON commands (favorite, updated_at DESC);
    CREATE INDEX idx_commands_kind     ON commands (kind);
    CREATE INDEX idx_commands_hash     ON commands (content_hash);

    CREATE TABLE tags (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        name       TEXT NOT NULL UNIQUE,
        created_at TEXT NOT NULL
    );

    CREATE TABLE command_tags (
        command_id INTEGER NOT NULL REFERENCES commands (id) ON DELETE CASCADE,
        tag_id     INTEGER NOT NULL REFERENCES tags (id)     ON DELETE CASCADE,
        PRIMARY KEY (command_id, tag_id)
    );

    CREATE INDEX idx_command_tags_tag ON command_tags (tag_id);

    CREATE TABLE collections (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        name        TEXT NOT NULL UNIQUE,
        description TEXT NOT NULL DEFAULT '',
        created_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL
    );

    CREATE TABLE command_collections (
        command_id    INTEGER NOT NULL REFERENCES commands (id)    ON DELETE CASCADE,
        collection_id INTEGER NOT NULL REFERENCES collections (id) ON DELETE CASCADE,
        PRIMARY KEY (command_id, collection_id)
    );

    CREATE INDEX idx_command_collections_collection ON command_collections (collection_id);

    CREATE TABLE usage_history (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        command_id INTEGER NOT NULL REFERENCES commands (id) ON DELETE CASCADE,
        action     TEXT    NOT NULL,
        used_at    TEXT    NOT NULL
    );

    CREATE INDEX idx_usage_history_command ON usage_history (command_id, used_at DESC);

    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE VIRTUAL TABLE commands_fts USING fts5 (
        title,
        content,
        description,
        notes,
        tags,
        collections,
        command_id UNINDEXED,
        tokenize = "unicode61 remove_diacritics 2"
    );
    "#,
    // 2 - cached AI explanations, plus the search column that carries them
    r#"
    CREATE TABLE ai_explanations (
        id               INTEGER PRIMARY KEY AUTOINCREMENT,
        command_id       INTEGER NOT NULL UNIQUE REFERENCES commands (id) ON DELETE CASCADE,
        content_hash     TEXT    NOT NULL,
        model            TEXT    NOT NULL,
        explanation_json TEXT    NOT NULL,
        search_text      TEXT    NOT NULL,
        created_at       TEXT    NOT NULL,
        updated_at       TEXT    NOT NULL
    );

    DROP TABLE commands_fts;

    CREATE VIRTUAL TABLE commands_fts USING fts5 (
        title,
        content,
        description,
        notes,
        tags,
        collections,
        explanation,
        command_id UNINDEXED,
        tokenize = "unicode61 remove_diacritics 2"
    );

    INSERT INTO commands_fts
        (title, content, description, notes, tags, collections, explanation, command_id)
    SELECT
        c.title,
        c.content,
        c.description,
        c.notes,
        COALESCE((
            SELECT group_concat(t.name, ' ') FROM command_tags ct
            JOIN tags t ON t.id = ct.tag_id
            WHERE ct.command_id = c.id
        ), ''),
        COALESCE((
            SELECT group_concat(col.name, ' ') FROM command_collections cc
            JOIN collections col ON col.id = cc.collection_id
            WHERE cc.command_id = c.id
        ), ''),
        '',
        c.id
    FROM commands c;
    "#,
];

/// Brings the database up to the newest schema version.
pub fn apply(conn: &mut Connection) -> AppResult<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let current = current.max(0) as usize;

    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)?;
        tx.commit()?;
    }

    Ok(())
}

/// Highest schema version this build knows about.
pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    /// A version 1 database with one entry, one tag and one collection, so the
    /// search rebuild in version 2 has something real to carry across.
    fn version_one_library() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory database");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

        let tx = conn.transaction().unwrap();
        tx.execute_batch(MIGRATIONS[0]).unwrap();
        tx.execute(
            "INSERT INTO commands (title, content, description, notes, content_hash,
                                   created_at, updated_at)
             VALUES ('Prune images', 'docker image prune -a', 'Frees disk space', '',
                     'hash-1', '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z')",
            [],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO tags (name, created_at) VALUES ('docker', '2026-07-01T00:00:00Z')",
            [],
        )
        .unwrap();
        tx.execute("INSERT INTO command_tags (command_id, tag_id) VALUES (1, 1)", [])
            .unwrap();
        tx.execute(
            "INSERT INTO collections (name, created_at, updated_at)
             VALUES ('Containers', '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z')",
            [],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO command_collections (command_id, collection_id) VALUES (1, 1)",
            [],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO commands_fts
                (title, content, description, notes, tags, collections, command_id)
             VALUES ('Prune images', 'docker image prune -a', 'Frees disk space', '',
                     'docker', 'Containers', 1)",
            [],
        )
        .unwrap();
        tx.pragma_update(None, "user_version", 1i64).unwrap();
        tx.commit().unwrap();

        conn
    }

    fn matches(conn: &Connection, expression: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM commands_fts WHERE commands_fts MATCH ?1",
            params![expression],
            |row| row.get(0),
        )
        .unwrap()
    }

    /// FTS5 has no ADD COLUMN, so version 2 rebuilds the table. Everything that
    /// was searchable before the upgrade has to still be searchable after it.
    #[test]
    fn upgrading_an_existing_library_keeps_its_search_rows() {
        let mut conn = version_one_library();
        apply(&mut conn).unwrap();

        assert_eq!(
            conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            latest_version()
        );
        assert_eq!(matches(&conn, "\"docker\"*"), 1, "tags survived the rebuild");
        assert_eq!(matches(&conn, "\"prune\"*"), 1, "titles survived the rebuild");
        assert_eq!(
            matches(&conn, "\"containers\"*"),
            1,
            "collections survived the rebuild"
        );
        assert_eq!(matches(&conn, "\"explanation\"*"), 0);
    }

    #[test]
    fn the_explanation_cache_follows_the_entry_it_describes() {
        let mut conn = version_one_library();
        apply(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO ai_explanations
                (command_id, content_hash, model, explanation_json, search_text,
                 created_at, updated_at)
             VALUES (1, 'hash-1', 'gpt-test', '{}', 'frees disk space',
                     '2026-07-02T00:00:00Z', '2026-07-02T00:00:00Z')",
            [],
        )
        .unwrap();

        conn.execute("DELETE FROM commands WHERE id = 1", []).unwrap();
        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM ai_explanations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining, 0, "the cascade cleans the cache up");
    }
}
