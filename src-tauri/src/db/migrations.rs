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
