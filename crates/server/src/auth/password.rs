//! The password the server checks logins against.
//!
//! It comes from the container's configuration, the same way the OpenAI key
//! does, because a server on a NAS has nowhere better to keep it. Unlike the
//! key it is read once at startup rather than on every use: changing it should
//! also end the sessions it granted, and a restart is what does that.

use std::path::PathBuf;

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub const PASSWORD_VAR: &str = "COMMAND_CENTER_PASSWORD";
pub const PASSWORD_FILE_VAR: &str = "COMMAND_CENTER_PASSWORD_FILE";

/// Short enough to type on a phone, long enough that the login backoff has
/// something to protect.
pub const MINIMUM_LENGTH: usize = 8;

/// The configured password, held as a digest. Comparing digests rather than
/// the text itself keeps the check to a fixed number of bytes, so how long it
/// takes says nothing about how much of a guess was right.
pub struct Password {
    digest: [u8; 32],
}

impl Password {
    pub fn from_env() -> Result<Self, String> {
        let configured = match read_configured()? {
            Some(value) => value,
            None => return Err(missing()),
        };

        if configured.chars().count() < MINIMUM_LENGTH {
            return Err(too_short());
        }

        Ok(Self::from_text(&configured))
    }

    fn from_text(value: &str) -> Self {
        Self {
            digest: Sha256::digest(value.as_bytes()).into(),
        }
    }

    pub fn matches(&self, submitted: &str) -> bool {
        let attempt: [u8; 32] = Sha256::digest(submitted.as_bytes()).into();
        self.digest.ct_eq(&attempt).into()
    }
}

/// The variable wins over the file, so an operator overriding a mounted secret
/// does not have to unmount it first.
fn read_configured() -> Result<Option<String>, String> {
    if let Some(value) = std::env::var(PASSWORD_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(Some(value.trim().to_owned()));
    }

    let Some(path) = password_file() else {
        return Ok(None);
    };

    let contents = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "The password file at {} could not be read: {error}",
            path.display()
        )
    })?;

    // Trimmed, because a file written with `echo` ends in a newline nobody
    // means to type.
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "The password file at {} is empty.",
            path.display()
        ));
    }
    Ok(Some(trimmed.to_owned()))
}

/// Written by hand so the digest never reaches a log line, however this ends
/// up being printed.
impl std::fmt::Debug for Password {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Password(configured)")
    }
}

fn password_file() -> Option<PathBuf> {
    std::env::var(PASSWORD_FILE_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

fn too_short() -> String {
    format!(
        "The configured password is shorter than {MINIMUM_LENGTH} characters. This server is reachable over the network, so set {PASSWORD_VAR} to a longer one."
    )
}

fn missing() -> String {
    format!(
        "No password is configured. Set {PASSWORD_VAR} to one, or {PASSWORD_FILE_VAR} to a file holding one, and start the server again."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn the_configured_password_is_the_only_one_that_matches() {
        let password = Password::from_text("a good long one");

        assert!(password.matches("a good long one"));
        assert!(!password.matches("a good long on"));
        assert!(!password.matches("a good long one "));
        assert!(!password.matches(""));
    }

    /// The plaintext must not survive anywhere it could be read back out.
    #[test]
    fn only_the_digest_is_kept() {
        let password = Password::from_text("correct horse battery");

        assert_ne!(password.digest, [0u8; 32]);
        assert_ne!(&password.digest[..], b"correct horse battery".as_slice());
    }

    /// These read the process environment, which every test in this binary
    /// shares, so they take turns.
    static ENV: Mutex<()> = Mutex::new(());

    #[test]
    fn a_short_password_is_refused_with_the_reason() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        std::env::set_var(PASSWORD_VAR, "short");
        std::env::remove_var(PASSWORD_FILE_VAR);

        let error = Password::from_env().unwrap_err();
        std::env::remove_var(PASSWORD_VAR);

        assert!(error.contains(PASSWORD_VAR));
        assert!(error.contains(&MINIMUM_LENGTH.to_string()));
    }

    #[test]
    fn a_password_file_is_read_and_trimmed() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("password");
        std::fs::write(&path, "  from a mounted file\n").unwrap();

        std::env::remove_var(PASSWORD_VAR);
        std::env::set_var(PASSWORD_FILE_VAR, &path);

        let password = Password::from_env().unwrap();
        std::env::remove_var(PASSWORD_FILE_VAR);

        assert!(password.matches("from a mounted file"));
    }

    #[test]
    fn no_password_at_all_stops_the_server_starting() {
        let _guard = ENV.lock().unwrap_or_else(|error| error.into_inner());
        std::env::remove_var(PASSWORD_VAR);
        std::env::remove_var(PASSWORD_FILE_VAR);

        let error = Password::from_env().unwrap_err();

        assert!(error.contains(PASSWORD_VAR));
        assert!(error.contains(PASSWORD_FILE_VAR));
    }

}
