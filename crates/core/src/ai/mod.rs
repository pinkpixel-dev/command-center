//! Shared AI configuration and the Rust-owned OpenAI boundary.

pub mod assistant;
pub(crate) mod client;
pub mod conversion;
pub mod credentials;
pub mod diagnosis;
pub mod disclosure;
pub mod explanation;
pub mod import;
pub mod inflight;
pub mod prompts;
pub mod providers;
pub mod proposal;
pub mod redaction;
mod transport;

pub use client::{AiConnectionResult, OpenAiClient, StructuredCall};
pub use credentials::{CredentialStore, KeyringCredentials};
pub use inflight::InFlight;
pub use proposal::CommandProposal;

/// The connection test returns one boolean, but the budget also has to cover
/// whatever the model spends on reasoning before it writes that boolean.
pub const CONNECTION_TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

/// Reads the one-boolean connection-test reply.
///
/// Shared by both providers so the schema and the confirmation rule cannot
/// drift apart. `provider_label` keeps each message accurate about who
/// answered.
pub fn parse_connection_test(
    output: &str,
    model: &str,
    provider_label: &str,
) -> crate::error::AppResult<AiConnectionResult> {
    #[derive(serde::Deserialize)]
    struct ConnectionTestOutput {
        ready: bool,
    }

    let parsed: ConnectionTestOutput = serde_json::from_str(output).map_err(|_| {
        crate::error::AppError::AiMalformed(format!(
            "{provider_label} returned invalid structured connection-test output."
        ))
    })?;
    if !parsed.ready {
        return Err(crate::error::AppError::AiMalformed(format!(
            "{provider_label} did not confirm the connection test."
        )));
    }

    Ok(AiConnectionResult {
        model: model.to_owned(),
    })
}

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
    /// Where the OpenAI API key lives. The desktop app hands over the
    /// operating system credential manager; the server hands over its
    /// environment.
    pub credentials: std::sync::Arc<dyn CredentialStore>,
    /// Assistant requests that can still be cancelled.
    pub inflight: InFlight,
}

impl AiService {
    pub fn new(credentials: std::sync::Arc<dyn CredentialStore>) -> crate::error::AppResult<Self> {
        Ok(Self {
            client: OpenAiClient::new()?,
            credentials,
            inflight: InFlight::default(),
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
