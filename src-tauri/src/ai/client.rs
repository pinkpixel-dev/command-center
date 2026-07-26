//! Minimal Responses API client used by every future AI workflow.

use std::time::Duration;

use reqwest::{redirect::Policy, Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const MAX_OUTPUT_TOKENS: u16 = 512;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct OpenAiClient {
    client: Client,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionResult {
    pub model: String,
}

impl OpenAiClient {
    pub fn new() -> AppResult<Self> {
        let client = Client::builder()
            .https_only(true)
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(45))
            .user_agent(concat!("command-center/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| AppError::ai_network("Could not initialize the secure OpenAI client."))?;
        Ok(Self { client })
    }

    pub async fn test_connection(
        &self,
        api_key: &str,
        model: &str,
    ) -> AppResult<AiConnectionResult> {
        let mut response = self
            .client
            .post(RESPONSES_URL)
            .bearer_auth(api_key)
            .json(&connection_test_request(model))
            .send()
            .await
            .map_err(map_transport_error)?;

        let status = response.status();
        let body = read_bounded_body(&mut response).await?;

        if !status.is_success() {
            let provider_error = serde_json::from_slice::<ProviderErrorEnvelope>(&body)
                .ok()
                .and_then(|envelope| envelope.error);
            return Err(map_provider_error(status, provider_error.as_ref()));
        }

        let parsed: ResponsesApiResponse = serde_json::from_slice(&body).map_err(|_| {
            AppError::AiMalformed("OpenAI returned a response the app could not read.".into())
        })?;
        validate_connection_response(&parsed)?;

        Ok(AiConnectionResult {
            model: model.to_owned(),
        })
    }
}

#[derive(Debug, Serialize)]
struct ResponsesApiRequest<'a> {
    model: &'a str,
    instructions: &'static str,
    input: &'static str,
    store: bool,
    max_output_tokens: u16,
    text: TextConfiguration,
}

#[derive(Debug, Serialize)]
struct TextConfiguration {
    format: StructuredOutputFormat,
}

#[derive(Debug, Serialize)]
struct StructuredOutputFormat {
    #[serde(rename = "type")]
    kind: &'static str,
    name: &'static str,
    strict: bool,
    schema: Value,
}

fn connection_test_request(model: &str) -> ResponsesApiRequest<'_> {
    ResponsesApiRequest {
        model,
        instructions: "Return the requested connection readiness object.",
        input: "Confirm that this Responses API request succeeded.",
        store: false,
        max_output_tokens: MAX_OUTPUT_TOKENS,
        text: TextConfiguration {
            format: StructuredOutputFormat {
                kind: "json_schema",
                name: "command_center_connection_test",
                strict: true,
                schema: json!({
                    "type": "object",
                    "properties": {
                        "ready": { "type": "boolean", "enum": [true] }
                    },
                    "required": ["ready"],
                    "additionalProperties": false
                }),
            },
        },
    }
}

#[derive(Debug, Deserialize)]
struct ResponsesApiResponse {
    status: String,
    #[serde(default)]
    output: Vec<ResponseOutputItem>,
}

#[derive(Debug, Deserialize)]
struct ResponseOutputItem {
    #[serde(default)]
    content: Vec<ResponseContentItem>,
}

#[derive(Debug, Deserialize)]
struct ResponseContentItem {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    refusal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConnectionTestOutput {
    ready: bool,
}

fn validate_connection_response(response: &ResponsesApiResponse) -> AppResult<()> {
    if response.status != "completed" {
        return Err(AppError::AiIncomplete(
            "OpenAI did not complete the connection test.".into(),
        ));
    }

    if response
        .output
        .iter()
        .flat_map(|item| &item.content)
        .any(|content| content.kind == "refusal" || content.refusal.is_some())
    {
        return Err(AppError::AiRefusal);
    }

    let output_text = response
        .output
        .iter()
        .flat_map(|item| &item.content)
        .find(|content| content.kind == "output_text")
        .and_then(|content| content.text.as_deref())
        .ok_or_else(|| {
            AppError::AiMalformed("OpenAI returned no structured connection-test output.".into())
        })?;

    let output: ConnectionTestOutput = serde_json::from_str(output_text).map_err(|_| {
        AppError::AiMalformed("OpenAI returned invalid structured connection-test output.".into())
    })?;
    if !output.ready {
        return Err(AppError::AiMalformed(
            "OpenAI did not confirm the connection test.".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ProviderErrorEnvelope {
    error: Option<ProviderError>,
}

#[derive(Debug, Deserialize)]
struct ProviderError {
    #[serde(default)]
    code: Option<String>,
}

fn map_transport_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() && error.is_connect() {
        AppError::ai_network("Connecting to OpenAI timed out. Try again.")
    } else if error.is_timeout() {
        AppError::ai_network("OpenAI did not respond before the request timed out. Try again.")
    } else {
        AppError::ai_network("Could not reach OpenAI. Check the network connection and try again.")
    }
}

async fn read_bounded_body(response: &mut reqwest::Response) -> AppResult<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(AppError::AiResponseTooLarge);
    }

    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_transport_error)? {
        append_response_chunk(&mut body, &chunk)?;
    }
    Ok(body)
}

fn append_response_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> AppResult<()> {
    if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
        return Err(AppError::AiResponseTooLarge);
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn map_provider_error(status: StatusCode, error: Option<&ProviderError>) -> AppError {
    let code = error.and_then(|details| details.code.as_deref());
    if code == Some("model_not_found") {
        return AppError::ai_model(
            "The selected model was not found or is not available to this OpenAI account.",
        );
    }
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            AppError::ai_auth("OpenAI rejected the stored API key or its permissions.")
        }
        StatusCode::TOO_MANY_REQUESTS => AppError::ai_rate_limit(
            "OpenAI rate-limited the request or the account has no available quota.",
        ),
        StatusCode::BAD_REQUEST => AppError::ai_response(
            "The selected model rejected the required Responses API structured-output request.",
        ),
        status if status.is_server_error() => {
            AppError::ai_response("OpenAI is temporarily unavailable. Try again later.")
        }
        _ => AppError::ai_response("OpenAI rejected the connection test."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_request_is_bounded_private_and_strictly_structured() {
        let request = serde_json::to_value(connection_test_request("gpt-test")).unwrap();

        assert_eq!(request["model"], "gpt-test");
        assert_eq!(request["store"], false);
        assert_eq!(request["max_output_tokens"], MAX_OUTPUT_TOKENS);
        assert_eq!(request["text"]["format"]["type"], "json_schema");
        assert_eq!(request["text"]["format"]["strict"], true);
        assert_eq!(
            request["text"]["format"]["schema"]["additionalProperties"],
            false
        );
        assert!(request.get("api_key").is_none());
    }

    #[test]
    fn completed_structured_output_is_required() {
        let valid: ResponsesApiResponse = serde_json::from_value(json!({
            "status": "completed",
            "output": [{
                "content": [{
                    "type": "output_text",
                    "text": "{\"ready\":true}"
                }]
            }]
        }))
        .unwrap();
        assert!(validate_connection_response(&valid).is_ok());

        let incomplete: ResponsesApiResponse = serde_json::from_value(json!({
            "status": "incomplete",
            "output": []
        }))
        .unwrap();
        assert!(validate_connection_response(&incomplete).is_err());
    }

    #[test]
    fn provider_failures_have_distinct_error_kinds() {
        let model_error = ProviderError {
            code: Some("model_not_found".into()),
        };
        assert_eq!(
            map_provider_error(StatusCode::NOT_FOUND, Some(&model_error)).kind(),
            "ai_model"
        );
        assert_eq!(
            map_provider_error(StatusCode::UNAUTHORIZED, None).kind(),
            "ai_auth"
        );
        assert_eq!(
            map_provider_error(StatusCode::TOO_MANY_REQUESTS, None).kind(),
            "ai_rate_limit"
        );
    }

    #[test]
    fn response_body_limit_is_enforced_without_allocating_past_the_cap() {
        let mut body = vec![0; MAX_RESPONSE_BYTES - 2];
        append_response_chunk(&mut body, &[1, 2]).unwrap();
        assert_eq!(body.len(), MAX_RESPONSE_BYTES);

        let error = append_response_chunk(&mut body, &[3]).unwrap_err();
        assert_eq!(error.kind(), "ai_response_too_large");
        assert_eq!(body.len(), MAX_RESPONSE_BYTES);
    }
}
