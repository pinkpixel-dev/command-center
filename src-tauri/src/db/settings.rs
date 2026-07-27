//! Application settings. Stored as a single JSON row, which keeps adding a
//! preference to one place instead of three.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

const SETTINGS_KEY: &str = "app_settings";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    /// "dark", "high-contrast", "light" or "system".
    pub theme: String,
    /// "compact" or "cards".
    pub command_view_mode: String,
    /// Ask before deleting an entry.
    pub confirm_before_delete: bool,
    /// Register Command Center with the operating system's login startup.
    pub launch_at_startup: bool,
    /// Hide the main window instead of exiting when it is closed.
    pub close_to_tray: bool,
    /// Allow network-backed AI actions. Off unless the user explicitly opts in.
    pub ai_enabled: bool,
    /// A curated or custom OpenAI model ID. None uses the Rust-owned default.
    pub ai_model: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            command_view_mode: "cards".into(),
            confirm_before_delete: true,
            launch_at_startup: false,
            close_to_tray: false,
            ai_enabled: false,
            ai_model: None,
        }
    }
}

impl AppSettings {
    fn sanitized(mut self) -> Self {
        if !matches!(
            self.theme.as_str(),
            "dark" | "high-contrast" | "light" | "system"
        ) {
            self.theme = "dark".into();
        }
        if !matches!(self.command_view_mode.as_str(), "compact" | "cards") {
            self.command_view_mode = "cards".into();
        }
        self.ai_model = self
            .ai_model
            .as_deref()
            .and_then(crate::ai::normalize_model_id);
        self
    }

    pub fn effective_ai_model(&self) -> &str {
        self.ai_model.as_deref().unwrap_or(crate::ai::DEFAULT_MODEL)
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
    if let Some(model) = settings.ai_model.as_deref() {
        let has_non_empty_value = !model.trim().is_empty();
        if has_non_empty_value && crate::ai::normalize_model_id(model).is_none() {
            return Err(crate::error::AppError::invalid(
                "Enter a valid OpenAI model ID without spaces.",
            ));
        }
    }
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
        assert_eq!(settings.command_view_mode, "cards");
        assert!(!settings.launch_at_startup);
        assert!(!settings.close_to_tray);
        assert!(!settings.ai_enabled);
        assert_eq!(settings.ai_model, None);
        assert_eq!(settings.effective_ai_model(), crate::ai::DEFAULT_MODEL);
    }

    #[test]
    fn settings_round_trip() {
        let db = Database::open_in_memory().unwrap();
        let settings = AppSettings {
            theme: "high-contrast".into(),
            command_view_mode: "cards".into(),
            confirm_before_delete: false,
            launch_at_startup: true,
            close_to_tray: true,
            ai_enabled: true,
            ai_model: Some("gpt-5.6-terra".into()),
        };

        db.with(|conn| save(conn, settings.clone())).unwrap();
        let loaded = db.with(load).unwrap();

        assert_eq!(loaded.theme, "high-contrast");
        assert_eq!(loaded.command_view_mode, "cards");
        assert!(!loaded.confirm_before_delete);
        assert!(loaded.launch_at_startup);
        assert!(loaded.close_to_tray);
        assert!(loaded.ai_enabled);
        assert_eq!(loaded.ai_model.as_deref(), Some("gpt-5.6-terra"));
    }

    #[test]
    fn nonsense_values_are_repaired() {
        let db = Database::open_in_memory().unwrap();
        let settings = AppSettings {
            theme: "neon".into(),
            command_view_mode: "masonry".into(),
            ..AppSettings::default()
        };

        let saved = db.with(|conn| save(conn, settings.clone())).unwrap();
        assert_eq!(saved.theme, "dark");
        assert_eq!(saved.command_view_mode, "cards");
    }

    #[test]
    fn legacy_settings_ignore_retired_preferences_and_keep_known_values() {
        let db = Database::open_in_memory().unwrap();
        db.with(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                params![
                    SETTINGS_KEY,
                    r#"{
                        "theme": "light",
                        "quickAddShortcut": "CommandOrControl+Alt+K",
                        "closeQuickAddAfterSave": false,
                        "confirmBeforeDelete": false,
                        "defaultCollectionId": 7
                    }"#
                ],
            )?;
            Ok(())
        })
        .unwrap();

        let loaded = db.with(load).unwrap();
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.command_view_mode, "cards");
        assert!(!loaded.confirm_before_delete);
        assert!(!loaded.ai_enabled);
        assert_eq!(loaded.ai_model, None);
    }

    #[test]
    fn saved_compact_preference_is_preserved() {
        let db = Database::open_in_memory().unwrap();
        let settings = AppSettings {
            command_view_mode: "compact".into(),
            ..AppSettings::default()
        };

        db.with(|conn| save(conn, settings)).unwrap();
        let loaded = db.with(load).unwrap();

        assert_eq!(loaded.command_view_mode, "compact");
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

    #[test]
    fn custom_model_is_trimmed_and_invalid_model_is_rejected() {
        let db = Database::open_in_memory().unwrap();
        let saved = db
            .with(|conn| {
                save(
                    conn,
                    AppSettings {
                        ai_model: Some("  ft:gpt-5:team:commands  ".into()),
                        ..AppSettings::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(saved.ai_model.as_deref(), Some("ft:gpt-5:team:commands"));

        let invalid = db.with(|conn| {
            save(
                conn,
                AppSettings {
                    ai_model: Some("gpt-5 invalid".into()),
                    ..AppSettings::default()
                },
            )
        });
        assert!(invalid.is_err());
    }
}
