//! Application settings. Stored as a single JSON row, which keeps adding a
//! preference to one place instead of three.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

const SETTINGS_KEY: &str = "app_settings";
pub const DEFAULT_QUICK_ADD_SHORTCUT: &str = "CommandOrControl+Shift+Space";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// "dark", "light" or "system".
    pub theme: String,
    /// Accelerator string understood by the global shortcut plugin.
    pub quick_add_shortcut: String,
    /// Whether the Quick Add window closes itself after a successful save.
    pub close_quick_add_after_save: bool,
    /// Ask before deleting an entry.
    pub confirm_before_delete: bool,
    /// Collection pre-selected for new entries.
    pub default_collection_id: Option<i64>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            quick_add_shortcut: DEFAULT_QUICK_ADD_SHORTCUT.into(),
            close_quick_add_after_save: true,
            confirm_before_delete: true,
            default_collection_id: None,
        }
    }
}

impl AppSettings {
    /// Falls back to the default shortcut when someone clears the field.
    fn sanitized(mut self) -> Self {
        self.quick_add_shortcut = self.quick_add_shortcut.trim().to_string();
        if self.quick_add_shortcut.is_empty() {
            self.quick_add_shortcut = DEFAULT_QUICK_ADD_SHORTCUT.into();
        }
        if !matches!(self.theme.as_str(), "dark" | "light" | "system") {
            self.theme = "dark".into();
        }
        self
    }
}

/// Reads settings, healing a corrupted row by returning defaults rather than
/// refusing to start.
pub fn load(conn: &Connection) -> AppResult<AppSettings> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![SETTINGS_KEY],
            |row| row.get(0),
        )
        .ok();

    Ok(stored
        .and_then(|raw| serde_json::from_str::<AppSettings>(&raw).ok())
        .unwrap_or_default()
        .sanitized())
}

pub fn save(conn: &Connection, settings: AppSettings) -> AppResult<AppSettings> {
    let settings = settings.sanitized();
    let encoded = serde_json::to_string(&settings)
        .map_err(|err| crate::error::AppError::runtime(format!("could not encode settings: {err}")))?;

    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![SETTINGS_KEY, encoded],
    )?;

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn defaults_are_returned_for_a_fresh_database() {
        let db = Database::open_in_memory().unwrap();
        let settings = db.with(load).unwrap();
        assert_eq!(settings.theme, "dark");
        assert_eq!(settings.quick_add_shortcut, DEFAULT_QUICK_ADD_SHORTCUT);
    }

    #[test]
    fn settings_round_trip() {
        let db = Database::open_in_memory().unwrap();
        let settings = AppSettings {
            theme: "light".into(),
            quick_add_shortcut: "CommandOrControl+Alt+K".into(),
            confirm_before_delete: false,
            ..AppSettings::default()
        };

        db.with(|conn| save(conn, settings.clone())).unwrap();
        let loaded = db.with(load).unwrap();

        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.quick_add_shortcut, "CommandOrControl+Alt+K");
        assert!(!loaded.confirm_before_delete);
    }

    #[test]
    fn nonsense_values_are_repaired() {
        let db = Database::open_in_memory().unwrap();
        let settings = AppSettings {
            theme: "neon".into(),
            quick_add_shortcut: "   ".into(),
            ..AppSettings::default()
        };

        let saved = db.with(|conn| save(conn, settings.clone())).unwrap();
        assert_eq!(saved.theme, "dark");
        assert_eq!(saved.quick_add_shortcut, DEFAULT_QUICK_ADD_SHORTCUT);
    }

    #[test]
    fn corrupted_json_falls_back_to_defaults() {
        let db = Database::open_in_memory().unwrap();
        db.with(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, '{not json')",
                params![SETTINGS_KEY],
            )?;
            Ok(())
        })
        .unwrap();

        let loaded = db.with(load).unwrap();
        assert_eq!(loaded.theme, "dark");
    }
}
