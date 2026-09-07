//! OS credential-manager access. Secrets never enter SQLite or frontend state
//! after the save command completes.

use keyring::{Entry, Error as KeyringError};

use crate::error::{AppError, AppResult};

const SERVICE: &str = "dev.pinkpixel.commandcenter";
const ACCOUNT: &str = "openai-api-key";
const MAX_KEY_LENGTH: usize = 4096;

#[derive(Debug, Clone, Default)]
pub struct CredentialStore;

impl CredentialStore {
    pub fn save(&self, value: &str) -> AppResult<()> {
        let value = normalize_key(value)?;
        entry()?.set_password(value).map_err(credential_error)
    }

    pub fn load(&self) -> AppResult<Option<String>> {
        match entry()?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(credential_error(error)),
        }
    }

    pub fn remove(&self) -> AppResult<()> {
        match entry()?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(credential_error(error)),
        }
    }

    pub fn has_key(&self) -> AppResult<bool> {
        Ok(self.load()?.is_some())
    }
}

fn entry() -> AppResult<Entry> {
    Entry::new(SERVICE, ACCOUNT).map_err(credential_error)
}

fn normalize_key(value: &str) -> AppResult<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid("Enter an OpenAI API key first."));
    }
    if trimmed.len() > MAX_KEY_LENGTH || trimmed.chars().any(char::is_whitespace) {
        return Err(AppError::invalid(
            "The OpenAI API key has an invalid format.",
        ));
    }
    Ok(trimmed)
}

fn credential_error(_error: KeyringError) -> AppError {
    match _error {
        KeyringError::NoStorageAccess(_) => AppError::credential(
            "Unlock the operating system credential manager and try again. The key was not stored.",
        ),
        KeyringError::Ambiguous(_) => AppError::credential(
            "More than one matching credential exists. Remove the duplicate operating system entries and try again.",
        ),
        _ => AppError::credential(
            "The operating system credential manager is unavailable. The key was not stored.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_validation_accepts_a_pasted_key_and_trims_its_edges() {
        assert_eq!(normalize_key("  sk-example  ").unwrap(), "sk-example");
    }

    #[test]
    fn key_validation_rejects_empty_or_internal_whitespace() {
        assert!(normalize_key("  ").is_err());
        assert!(normalize_key("sk-example value").is_err());
    }

    #[test]
    fn credential_identity_is_stable_and_app_specific() {
        assert_eq!(SERVICE, "dev.pinkpixel.commandcenter");
        assert_eq!(ACCOUNT, "openai-api-key");
    }

    #[test]
    fn locked_storage_has_an_actionable_error_without_platform_details() {
        let error = KeyringError::NoStorageAccess(Box::new(std::io::Error::other(
            "private platform detail",
        )));
        let message = credential_error(error).to_string();

        assert!(message.contains("Unlock the operating system credential manager"));
        assert!(!message.contains("private platform detail"));
    }
}
