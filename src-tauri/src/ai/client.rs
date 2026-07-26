//! Responses API client. Every AI workflow goes through one structured call,
//! so timeouts, privacy defaults, and response validation stay in one place.

use std::time::Duration;

use reqwest::{redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai::prompts::{self, StructuredTask};
use crate::ai::transport::{map_provider_error, map_transport_error, read_bounded_body};
use crate::error::{AppError, AppResult};

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const CONNECTION_TEST_TOKENS: u32 = 512;
const CONNECTION_TEST_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone)]
pub struct OpenAiClient {
    client: Client,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionResult {
    pub model: String,
}

/// One strictly structured request. The caller owns the task definition, the
/// budget, and how long it is willing to wait.
pub struct StructuredCall<'a> {
    pub model: &'a str,
    pub task: &'a StructuredTask,
    pub input: &'a str,
    pub max_output_tokens: u32,
    pub timeout: Duration,
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

    /// Sends one request and returns the structured JSON text the model
    /// produced. Refusals, incomplete runs, and missing output are errors.
    pub async fn structured_json(&self, api_key: &str, call: StructuredCall<'_>) -> AppResult<String> {
        let mut response = self
            .client
            .post(RESPONSES_URL)
            .timeout(call.timeout)
            .bearer_auth(api_key)
            .json(&structured_request(&call))
            .send()
            .await
            .map_err(map_transport_error)?;

        let status = response.status();
        let body = read_bounded_body(&mut response).await?;

        if !status.is_success() {
            return Err(map_provider_error(status, &body));
        }

        let parsed: ResponsesApiResponse = serde_json::from_slice(&body).map_err(|_| {
            AppError::AiMalformed("OpenAI returned a response the app could not read.".into())
        })?;
        structured_output(&parsed).map(str::to_owned)
    }

    pub async fn test_connection(
        &self,
        api_key: &str,
        model: &str,
    ) -> AppResult<AiConnectionResult> {
        let output = self
            .structured_json(
                api_key,
                StructuredCall {
                    model,
                    task: prompts::connection_test(),
                    input: prompts::CONNECTION_TEST_INPUT,
                    max_output_tokens: CONNECTION_TEST_TOKENS,
                    timeout: CONNECTION_TEST_TIMEOUT,
                },
            )
            .await?;

        let parsed: ConnectionTestOutput = serde_json::from_str(&output).map_err(|_| {
            AppError::AiMalformed("OpenAI returned invalid structured connection-test output.".into())
        })?;
        if !parsed.ready {
            return Err(AppError::AiMalformed(
                "OpenAI did not confirm the connection test.".into(),
            ));
        }

        Ok(AiConnectionResult {
            model: model.to_owned(),
        })
    }
}

#[derive(Debug, Serialize)]
struct ResponsesApiRequest<'a> {
    model: &'a str,
    instructions: &'a str,
    input: &'a str,
    store: bool,
    max_output_tokens: u32,
    text: TextConfiguration<'a>,
}

#[derive(Debug, Serialize)]
struct TextConfiguration<'a> {
    format: StructuredOutputFormat<'a>,
}

#[derive(Debug, Serialize)]
struct StructuredOutputFormat<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    name: &'static str,
    strict: bool,
    schema: &'a Value,
}

fn structured_request<'a>(call: &'a StructuredCall<'a>) -> ResponsesApiRequest<'a> {
    ResponsesApiRequest {
        model: call.model,
        instructions: call.task.instructions,
        input: call.input,
        // Command Center keeps no history with the provider.
        store: false,
        max_output_tokens: call.max_output_tokens,
        text: TextConfiguration {
            format: StructuredOutputFormat {
                kind: "json_schema",
                name: call.task.name,
                strict: true,
                schema: &call.task.schema,
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

fn structured_output(response: &ResponsesApiResponse) -> AppResult<&str> {
    if response
        .output
        .iter()
        .flat_map(|item| &item.content)
        .any(|content| content.kind == "refusal" || content.refusal.is_some())
    {
        return Err(AppError::AiRefusal);
    }

    if response.status != "completed" {
        return Err(AppError::AiIncomplete(
            "OpenAI stopped before it finished the response. Try a smaller document.".into(),
        ));
    }

    response
        .output
        .iter()
        .flat_map(|item| &item.content)
        .find(|content| content.kind == "output_text")
        .and_then(|content| content.text.as_deref())
        .ok_or_else(|| AppError::AiMalformed("OpenAI returned no structured output.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    fn call<'a>(model: &'a str, input: &'a str) -> StructuredCall<'a> {
        StructuredCall {
            model,
            task: prompts::connection_test(),
            input,
            max_output_tokens: CONNECTION_TEST_TOKENS,
            timeout: CONNECTION_TEST_TIMEOUT,
        }
    }

    #[test]
    fn requests_are_bounded_private_and_strictly_structured() {
        let outgoing = call("gpt-test", prompts::CONNECTION_TEST_INPUT);
        let request = serde_json::to_value(structured_request(&outgoing)).unwrap();

        assert_eq!(request["model"], "gpt-test");
        assert_eq!(request["store"], false);
        assert_eq!(request["max_output_tokens"], CONNECTION_TEST_TOKENS);
        assert_eq!(request["text"]["format"]["type"], "json_schema");
        assert_eq!(request["text"]["format"]["strict"], true);
        assert_eq!(
            request["text"]["format"]["schema"]["additionalProperties"],
            false
        );
        assert!(request.get("api_key").is_none());
        assert!(request.get("previous_response_id").is_none());
    }

    #[test]
    fn the_caller_decides_the_budget_and_the_wait() {
        let task = prompts::import_extraction();
        let outgoing = StructuredCall {
            model: "gpt-test",
            task,
            input: "document",
            max_output_tokens: 4_096,
            timeout: Duration::from_secs(120),
        };
        let request = serde_json::to_value(structured_request(&outgoing)).unwrap();

        assert_eq!(request["max_output_tokens"], 4_096);
        assert_eq!(
            request["text"]["format"]["name"],
            "command_center_import_extraction"
        );
        assert_eq!(request["instructions"], task.instructions);
    }

    fn response(value: serde_json::Value) -> ResponsesApiResponse {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn completed_structured_output_is_required() {
        let valid = response(json!({
            "status": "completed",
            "output": [{ "content": [{ "type": "output_text", "text": "{\"ready\":true}" }] }]
        }));
        assert_eq!(structured_output(&valid).unwrap(), "{\"ready\":true}");

        let incomplete = response(json!({ "status": "incomplete", "output": [] }));
        assert_eq!(
            structured_output(&incomplete).unwrap_err().kind(),
            "ai_incomplete"
        );

        let empty = response(json!({ "status": "completed", "output": [] }));
        assert_eq!(structured_output(&empty).unwrap_err().kind(), "ai_malformed");
    }

    #[test]
    fn a_refusal_is_reported_as_a_refusal_even_when_the_run_completed() {
        let refused = response(json!({
            "status": "completed",
            "output": [{ "content": [{ "type": "refusal", "refusal": "no" }] }]
        }));
        assert_eq!(structured_output(&refused).unwrap_err().kind(), "ai_refusal");
    }
}
