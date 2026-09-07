//! Where the OpenAI API key comes from.
//!
//! Secrets never enter SQLite or frontend state after the save command
//! completes. Where they do live depends on the application: the desktop app
//! uses the operating system credential manager, and a container has no such
//! thing, so the store is a trait with one implementation per shell.

use keyring::{Entry, Error as KeyringError};

use crate::error::{AppError, AppResult};

const SERVICE: &str = "dev.pinkpixel.commandcenter";
const ACCOUNT: &str = "openai-api-key";
const MAX_KEY_LENGTH: usize = 4096;

/// Read and write access to the one secret Command Center holds.
///
/// A store that cannot accept writes, such as one backed by a read-only
/// mounted secret, returns an error from `save` and `remove` that says where
/// the key should be changed instead.
pub trait CredentialStore: Send + Sync + 'static {
    fn save(&self, value: &str) -> AppResult<()>;
    fn load(&self) -> AppResult<Option<String>>;
    fn remove(&self) -> AppResult<()>;

    fn has_key(&self) -> AppResult<bool> {
        Ok(self.load()?.is_some())
    }
}

/// The desktop store: the operating system credential manager.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyringCredentials;

impl CredentialStore for KeyringCredentials {
    fn save(&self, value: &str) -> AppResult<()> {
        let value = normalize_key(value)?;
        entry()?.set_password(value).map_err(credential_error)
    }

    fn load(&self) -> AppResult<Option<String>> {
        match entry()?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(credential_error(error)),
        }
    }

    fn remove(&self) -> AppResult<()> {
        match entry()?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(credential_error(error)),
        }
    }
}

fn entry() -> AppResult<Entry> {
    Entry::new(SERVICE, ACCOUNT).map_err(credential_error)
}

/// Trims a pasted key and rejects anything that is not shaped like one.
///
/// Public because every store has to apply the same rule. A key that would be
/// refused on the desktop must not be accepted from an environment variable.
pub fn normalize_key(value: &str) -> AppResult<&str> {
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
    use std::sync::Mutex;

    /// Stands in for any store that is not the operating system's, which is
    /// what the server will be.
    #[derive(Default)]
    struct InMemory(Mutex<Option<String>>);

    impl CredentialStore for InMemory {
        fn save(&self, value: &str) -> AppResult<()> {
            *self.0.lock().unwrap() = Some(normalize_key(value)?.to_owned());
            Ok(())
        }
        fn load(&self) -> AppResult<Option<String>> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn remove(&self) -> AppResult<()> {
            *self.0.lock().unwrap() = None;
            Ok(())
        }
    }

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

    /// `has_key` is answered by `load`, so no store has to implement the
    /// question twice or answer it differently.
    #[test]
    fn a_store_reports_a_key_once_one_is_saved() {
        let store = InMemory::default();
        assert!(!store.has_key().unwrap());

        store.save("  sk-example  ").unwrap();
        assert_eq!(store.load().unwrap().as_deref(), Some("sk-example"));
        assert!(store.has_key().unwrap());

        store.remove().unwrap();
        assert!(!store.has_key().unwrap());
    }

    #[test]
    fn a_store_can_be_used_behind_a_trait_object() {
        let store: std::sync::Arc<dyn CredentialStore> =
            std::sync::Arc::new(InMemory::default());
        store.save("sk-example").unwrap();

        assert!(store.clone().has_key().unwrap());
    }
}
