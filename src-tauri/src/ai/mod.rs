//! Shared AI configuration and the Rust-owned OpenAI boundary.

mod client;
mod credentials;
pub mod explanation;
pub mod import;
pub mod prompts;
pub mod redaction;
mod transport;

pub use client::{AiConnectionResult, OpenAiClient, StructuredCall};
pub use credentials::CredentialStore;

pub const DEFAULT_MODEL: &str = "gpt-5.6-luna";
pub const CURATED_MODELS: &[&str] = &[
    "gpt-5.6-luna",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.4-nano",
    "gpt-5.2",
    "gpt-5.1",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
];

const MAX_MODEL_ID_LENGTH: usize = 256;

/// Model IDs are provider identifiers, not free-form prompt text.
pub fn normalize_model_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_MODEL_ID_LENGTH
        || trimmed.chars().any(char::is_whitespace)
        || trimmed.chars().any(char::is_control)
    {
        return None;
    }
    Some(trimmed.to_owned())
}

pub struct AiService {
    pub client: OpenAiClient,
    pub credentials: CredentialStore,
}

impl AiService {
    pub fn new() -> crate::error::AppResult<Self> {
        Ok(Self {
            client: OpenAiClient::new()?,
            credentials: CredentialStore,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_list_has_the_default_first_and_no_duplicates() {
        assert_eq!(CURATED_MODELS.first().copied(), Some(DEFAULT_MODEL));

        let mut unique = std::collections::HashSet::new();
        assert!(CURATED_MODELS.iter().all(|model| unique.insert(model)));
    }

    #[test]
    fn custom_model_ids_are_trimmed_but_not_whitelisted() {
        assert_eq!(
            normalize_model_id("  ft:gpt-5:my-team:command-center  "),
            Some("ft:gpt-5:my-team:command-center".into())
        );
        assert_eq!(normalize_model_id(""), None);
        assert_eq!(normalize_model_id("gpt-5\nanother"), None);
    }
}
