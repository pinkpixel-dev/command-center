pub mod collections;
pub mod commands;
pub mod explanations;
pub mod migrations;
pub mod query;
pub mod search;
pub mod settings;
pub mod tags;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Single shared SQLite connection. A personal library never has enough
/// concurrent traffic to justify a pool, and a mutex keeps writes ordered.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Opens (or creates) the library file and migrates it to the latest schema.
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| AppError::runtime(format!("could not create data directory: {err}")))?;
        }
        let conn = Connection::open(path)?;
        Self::prepare(conn)
    }

    /// In-memory library, used by the test suite.
    pub fn open_in_memory() -> AppResult<Self> {
        Self::prepare(Connection::open_in_memory()?)
    }

    fn prepare(mut conn: Connection) -> AppResult<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        migrations::apply(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Runs a closure against the connection. Poisoned locks are surfaced as a
    /// normal error rather than panicking the whole app.
    pub fn with<T>(&self, action: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::runtime("database lock was poisoned"))?;
        action(&guard)
    }

    /// Same as [`Database::with`], but the closure gets a mutable connection so
    /// it can open a transaction.
    pub fn with_mut<T>(&self, action: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::runtime("database lock was poisoned"))?;
        action(&mut guard)
    }

    pub fn schema_version(&self) -> AppResult<i64> {
        self.with(|conn| Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?))
    }
}

/// Timestamps are stored as RFC 3339 UTC strings so they sort lexicographically
/// and stay readable when someone opens the file in a SQLite browser.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_is_migrated_to_latest() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), migrations::latest_version());
    }

    #[test]
    fn migrations_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");

        let first = Database::open(&path).unwrap();
        let version = first.schema_version().unwrap();
        drop(first);

        let second = Database::open(&path).unwrap();
        assert_eq!(second.schema_version().unwrap(), version);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let db = Database::open_in_memory().unwrap();
        let result = db.with(|conn| {
            conn.execute(
                "INSERT INTO command_tags (command_id, tag_id) VALUES (999, 999)",
                [],
            )?;
            Ok(())
        });
        assert!(result.is_err());
    }
}
