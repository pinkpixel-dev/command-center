//! The server's OpenAI key store.
//!
//! A container has no operating system credential manager, so the key comes
//! from the environment: either a variable or a mounted secrets file. The file
//! is read on every load rather than cached, so rotating a mounted secret takes
//! effect without a restart.

use std::path::PathBuf;

use command_center_core::ai::credentials::{normalize_key, CredentialStore};
use command_center_core::error::{AppError, AppResult};

pub const KEY_VAR: &str = "COMMAND_CENTER_OPENAI_API_KEY";
pub const KEY_FILE_VAR: &str = "COMMAND_CENTER_OPENAI_API_KEY_FILE";

/// Reads the key the operator configured. Writes are refused, because the
/// answer to "change the key" here is to change the container's configuration,
/// not to have the app write into its own environment.
#[derive(Debug, Clone, Copy, Default)]
pub struct EnvCredentials;

impl CredentialStore for EnvCredentials {
    fn save(&self, _value: &str) -> AppResult<()> {
        Err(read_only())
    }

    fn load(&self) -> AppResult<Option<String>> {
        if let Some(value) = std::env::var(KEY_VAR).ok().filter(|v| !v.trim().is_empty()) {
            return Ok(Some(normalize_key(&value)?.to_owned()));
        }

        let Some(path) = key_file() else {
            return Ok(None);
        };

        let contents = std::fs::read_to_string(&path).map_err(|error| {
            AppError::credential(format!(
                "The API key file at {} could not be read: {error}",
                path.display()
            ))
        })?;

        if contents.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(normalize_key(&contents)?.to_owned()))
    }

    fn remove(&self) -> AppResult<()> {
        Err(read_only())
    }
}

fn key_file() -> Option<PathBuf> {
    std::env::var(KEY_FILE_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

fn read_only() -> AppError {
    AppError::credential(format!(
        "This server reads its OpenAI API key from its own configuration. Set {KEY_VAR} or {KEY_FILE_VAR} and restart it."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// These tests read and write the process environment, which every test in
    /// this binary shares, so they take turns.
    static ENV: Mutex<()> = Mutex::new(());

    /// Writing has to fail with an instruction, not a silent success that
    /// leaves the operator thinking a key was saved.
    #[test]
    fn writes_are_refused_and_say_where_the_key_belongs() {
        let store = EnvCredentials;

        let error = store.save("sk-example").unwrap_err();
        assert_eq!(error.kind(), "credential");
        assert!(error.to_string().contains(KEY_VAR));
        assert!(error.to_string().contains(KEY_FILE_VAR));

        assert!(store.remove().is_err());
    }

    #[test]
    fn a_key_file_is_read_and_trimmed() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openai-key");
        std::fs::write(&path, "  sk-from-a-file\n").unwrap();

        std::env::set_var(KEY_FILE_VAR, &path);
        std::env::remove_var(KEY_VAR);

        assert_eq!(
            EnvCredentials.load().unwrap().as_deref(),
            Some("sk-from-a-file")
        );

        std::env::remove_var(KEY_FILE_VAR);
    }

    #[test]
    fn no_configured_key_reports_nothing_rather_than_failing() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        std::env::remove_var(KEY_VAR);
        std::env::remove_var(KEY_FILE_VAR);

        assert_eq!(EnvCredentials.load().unwrap(), None);
        assert!(!EnvCredentials.has_key().unwrap());
    }
}
