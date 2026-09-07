//! Provider implementations behind the AI workflows.
//!
//! The existing OpenAI API-key client still lives in `crate::ai::client`. It
//! moves here when the workflows actually route through a provider router;
//! moving it early would be churn with no behaviour change.

pub mod codex;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ai::client::{OpenAiClient, StructuredCall};
use crate::ai::prompts::StructuredTask;
use crate::ai::AiService;
use crate::db::settings::AppSettings;
use crate::error::{AppError, AppResult};
use codex::service::CodexService;

/// One structured request, expressed without reference to how it is delivered.
///
/// This is the same information `OpenAiClient::structured_json` already takes.
/// The workflows build it; the provider decides what to do with it.
pub struct ProviderCall<'a> {
    pub model: &'a str,
    pub task: &'a StructuredTask,
    pub input: &'a str,
    /// An OpenAI Responses budget. Codex has no equivalent parameter, so it
    /// bounds the answer by length while it streams instead.
    pub max_output_tokens: u32,
    pub timeout: Duration,
}

/// A resolved provider, owned so it can move into the task that runs the
/// request. Cancellation aborts that task, which is why nothing here borrows.
#[derive(Clone)]
pub enum ProviderClient {
    OpenAi {
        client: OpenAiClient,
        api_key: String,
    },
    Codex {
        service: Arc<CodexService>,
        saved_path: Option<String>,
    },
}

impl ProviderClient {
    /// Runs one structured task and returns the JSON text it produced. Both
    /// providers return to the same workflow parser.
    pub async fn structured_json(&self, call: ProviderCall<'_>) -> AppResult<String> {
        match self {
            Self::OpenAi { client, api_key } => {
                client
                    .structured_json(
                        api_key,
                        StructuredCall {
                            model: call.model,
                            task: call.task,
                            input: call.input,
                            max_output_tokens: call.max_output_tokens,
                            timeout: call.timeout,
                        },
                    )
                    .await
            }
            Self::Codex {
                service,
                saved_path,
            } => {
                service
                    .run_structured_task(
                        saved_path.as_deref(),
                        call.model,
                        call.task,
                        call.input,
                        call.timeout,
                    )
                    .await
            }
        }
    }

    pub fn provider(&self) -> AiProvider {
        match self {
            Self::OpenAi { .. } => AiProvider::OpenaiApi,
            Self::Codex { .. } => AiProvider::ChatgptCodex,
        }
    }
}

/// Resolves the provider the settings select, along with its model.
///
/// This is the one place that decides what a workflow talks to, so a request
/// can never reach a provider the user did not choose.
pub async fn resolve(
    settings: &AppSettings,
    ai: &AiService,
    codex: &Arc<CodexService>,
) -> AppResult<(ProviderClient, String)> {
    if !settings.ai_enabled {
        return Err(AppError::AiDisabled);
    }

    let provider = settings.provider();
    if !provider.is_available_on_this_platform() {
        return Err(AppError::ai_model(
            "That AI provider is not available on this platform.",
        ));
    }

    match provider {
        AiProvider::OpenaiApi => {
            let credentials = ai.credentials.clone();
            let api_key = tokio::task::spawn_blocking(move || credentials.load())
                .await
                .map_err(|_| {
                    AppError::credential("The operating system credential manager stopped.")
                })??
                .ok_or(AppError::AiNotConfigured)?;

            Ok((
                ProviderClient::OpenAi {
                    client: ai.client.clone(),
                    api_key,
                },
                settings.effective_ai_model().to_owned(),
            ))
        }
        AiProvider::ChatgptCodex => {
            // No curated default exists for Codex: the catalogue depends on the
            // connected plan, so a missing selection has to stop here.
            let model = settings
                .codex_model
                .clone()
                .ok_or_else(|| AppError::ai_model("Choose a Codex model in Settings first."))?;

            Ok((
                ProviderClient::Codex {
                    service: Arc::clone(codex),
                    saved_path: settings.codex_path.clone(),
                },
                model,
            ))
        }
    }
}

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
            Self::ChatgptCodex => crate::IS_DESKTOP,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::settings::AppSettings;

    fn service() -> AiService {
        AiService::new(Arc::new(crate::ai::KeyringCredentials))
            .expect("the AI service should build")
    }

    fn codex_service() -> Arc<CodexService> {
        let dir = std::env::temp_dir().join("command-center-provider-tests");
        Arc::new(CodexService::new(&dir, "1.0.3"))
    }

    #[tokio::test]
    async fn ai_off_resolves_to_nothing_at_all() {
        let settings = AppSettings {
            ai_enabled: false,
            ..AppSettings::default()
        };

        let error = match resolve(&settings, &service(), &codex_service()).await {
            Ok(_) => panic!("a disabled provider must not resolve"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), "ai_disabled");
    }

    #[tokio::test]
    async fn codex_without_a_chosen_model_refuses_rather_than_guessing() {
        let settings = AppSettings {
            ai_enabled: true,
            ai_provider: AiProvider::ChatgptCodex.as_str().into(),
            // An OpenAI model is saved, and must not be borrowed for Codex.
            ai_model: Some("gpt-5.6-terra".into()),
            codex_model: None,
            ..AppSettings::default()
        };

        let error = match resolve(&settings, &service(), &codex_service()).await {
            Ok(_) => panic!("Codex has no default model to fall back on"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), "ai_model");
    }

    #[tokio::test]
    async fn codex_resolves_to_the_codex_provider_and_its_own_model() {
        let settings = AppSettings {
            ai_enabled: true,
            ai_provider: AiProvider::ChatgptCodex.as_str().into(),
            ai_model: Some("gpt-5.6-terra".into()),
            codex_model: Some("gpt-5.6-luna".into()),
            ..AppSettings::default()
        };

        let (client, model) = resolve(&settings, &service(), &codex_service())
            .await
            .expect("Codex should resolve");

        assert_eq!(client.provider(), AiProvider::ChatgptCodex);
        // The OpenAI selection is right there and is still not used.
        assert_eq!(model, "gpt-5.6-luna");
    }

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

    #[test]
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
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
