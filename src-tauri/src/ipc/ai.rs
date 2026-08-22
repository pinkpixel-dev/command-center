//! Settings-facing AI commands. Configuration calls work while AI is off;
//! the network test is protected by the backend opt-in gate.

use serde::Serialize;
use tauri::State;

use crate::ai::providers::codex::service::CodexService;
use crate::ai::providers::AiProvider;
use crate::ai::{
    prompts, AiConnectionResult, AiService, CONNECTION_TEST_TIMEOUT, CURATED_MODELS,
    DEFAULT_MODEL,
};
use crate::db::settings;
use crate::db::Database;
use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatus {
    pub key_stored: bool,
    pub credential_manager_available: bool,
    pub default_model: &'static str,
    pub effective_model: String,
    pub models: &'static [&'static str],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiKeyStatus {
    pub key_stored: bool,
}

#[tauri::command]
pub async fn get_ai_status(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
) -> AppResult<AiStatus> {
    let settings = db.with(settings::load)?;
    let credentials = ai.credentials.clone();
    let credential_status = run_credential_task(move || credentials.has_key()).await;
    let (key_stored, credential_manager_available) = match credential_status {
        Ok(stored) => (stored, true),
        Err(_) => (false, false),
    };

    Ok(AiStatus {
        key_stored,
        credential_manager_available,
        default_model: DEFAULT_MODEL,
        effective_model: settings.effective_ai_model().to_owned(),
        models: CURATED_MODELS,
    })
}

#[tauri::command]
pub async fn save_ai_key(ai: State<'_, AiService>, api_key: String) -> AppResult<AiKeyStatus> {
    let credentials = ai.credentials.clone();
    run_credential_task(move || credentials.save(&api_key)).await?;
    Ok(AiKeyStatus { key_stored: true })
}

#[tauri::command]
pub async fn remove_ai_key(ai: State<'_, AiService>) -> AppResult<AiKeyStatus> {
    let credentials = ai.credentials.clone();
    run_credential_task(move || credentials.remove()).await?;
    Ok(AiKeyStatus { key_stored: false })
}

#[tauri::command]
pub async fn test_ai_connection(
    db: State<'_, Database>,
    ai: State<'_, AiService>,
    codex: State<'_, CodexService>,
) -> AppResult<AiConnectionResult> {
    let settings = db.with(settings::load)?;
    if !settings.ai_enabled {
        return Err(AppError::AiDisabled);
    }

    match settings.provider() {
        AiProvider::OpenaiApi => {
            let model = settings.effective_ai_model().to_owned();
            let credentials = ai.credentials.clone();
            let api_key = run_credential_task(move || credentials.load())
                .await?
                .ok_or(AppError::AiNotConfigured)?;

            ai.client.test_connection(&api_key, &model).await
        }
        AiProvider::ChatgptCodex => {
            // There is no curated Codex default: the catalogue depends on the
            // connected plan, so a selection is required rather than guessed.
            let model = settings
                .codex_model
                .clone()
                .ok_or_else(|| AppError::ai_model("Choose a Codex model in Settings first."))?;

            let output = codex
                .run_structured_task(
                    settings.codex_path.as_deref(),
                    &model,
                    prompts::connection_test(),
                    prompts::CONNECTION_TEST_INPUT,
                    CONNECTION_TEST_TIMEOUT,
                )
                .await?;

            crate::ai::parse_connection_test(&output, &model, "Codex")
        }
    }
}

async fn run_credential_task<T, F>(task: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|_| AppError::credential("The operating system credential manager stopped."))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_payload_never_contains_a_key_field() {
        let status = AiStatus {
            key_stored: true,
            credential_manager_available: true,
            default_model: DEFAULT_MODEL,
            effective_model: DEFAULT_MODEL.into(),
            models: CURATED_MODELS,
        };
        let value = serde_json::to_value(status).unwrap();

        assert_eq!(value["keyStored"], true);
        assert_eq!(value["credentialManagerAvailable"], true);
        assert!(value.get("apiKey").is_none());
        assert!(value.get("key").is_none());
    }
}
