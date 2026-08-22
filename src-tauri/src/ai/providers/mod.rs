//! Provider implementations behind the AI workflows.
//!
//! The existing OpenAI API-key client still lives in `crate::ai::client`. It
//! moves here when the workflows actually route through a provider router;
//! moving it early would be churn with no behaviour change.

pub mod codex;

use serde::{Deserialize, Serialize};

/// Which provider the AI workflows use.
///
/// The two are never interchangeable. They have different credentials, model
/// catalogues, limits, and request contracts, so a model ID is only meaningful
/// alongside the provider it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AiProvider {
    /// An OpenAI API key stored in the operating-system credential manager.
    #[default]
    OpenaiApi,
    /// A ChatGPT account connected through the user's Codex installation.
    ChatgptCodex,
}

impl AiProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenaiApi => "openaiApi",
            Self::ChatgptCodex => "chatgptCodex",
        }
    }

    /// Unknown values become the default rather than failing to load settings.
    /// An installation that read a newer settings row still starts.
    pub fn parse(value: &str) -> Self {
        match value {
            "chatgptCodex" => Self::ChatgptCodex,
            _ => Self::OpenaiApi,
        }
    }

    /// Whether this provider can run on the current build.
    ///
    /// Codex needs a local child process, which a mobile build cannot spawn,
    /// so the API-key path stays the only mobile provider.
    pub fn is_available_on_this_platform(self) -> bool {
        match self {
            Self::OpenaiApi => true,
            Self::ChatgptCodex => cfg!(desktop),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_installations_default_to_the_api_key_provider() {
        assert_eq!(AiProvider::default(), AiProvider::OpenaiApi);
        assert_eq!(AiProvider::parse(""), AiProvider::OpenaiApi);
    }

    #[test]
    fn a_saved_provider_round_trips_through_its_stored_name() {
        for provider in [AiProvider::OpenaiApi, AiProvider::ChatgptCodex] {
            assert_eq!(AiProvider::parse(provider.as_str()), provider);
        }
    }

    #[test]
    fn an_unknown_provider_falls_back_instead_of_breaking_settings() {
        assert_eq!(AiProvider::parse("anthropic"), AiProvider::OpenaiApi);
        assert_eq!(AiProvider::parse("chatgptcodex"), AiProvider::OpenaiApi);
    }

    #[test]
    fn the_api_key_provider_is_available_everywhere() {
        assert!(AiProvider::OpenaiApi.is_available_on_this_platform());
    }

    #[cfg(desktop)]
    #[test]
    fn codex_is_available_on_desktop() {
        assert!(AiProvider::ChatgptCodex.is_available_on_this_platform());
    }

    #[test]
    fn the_serialized_names_match_what_the_frontend_sends() {
        let encoded = serde_json::to_string(&AiProvider::ChatgptCodex).unwrap();
        assert_eq!(encoded, "\"chatgptCodex\"");
        assert_eq!(serde_json::to_string(&AiProvider::OpenaiApi).unwrap(), "\"openaiApi\"");
    }
}
