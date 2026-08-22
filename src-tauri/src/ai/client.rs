//! Responses API client. Every AI workflow goes through one structured call,
//! so timeouts, privacy defaults, and response validation stay in one place.

use std::time::Duration;

use reqwest::{redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai::prompts::{self, StructuredTask};
use crate::ai::CONNECTION_TEST_TIMEOUT;
use crate::ai::transport::{map_provider_error, map_transport_error, read_bounded_body};
use crate::error::{AppError, AppResult};

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
/// The connection test returns one boolean, but the budget also has to cover
/// whatever the model spends on reasoning before it writes that boolean. A
/// small reasoning model will happily spend thousands of tokens on it, and an
/// unused ceiling is not billed.
const CONNECTION_TEST_TOKENS: u32 = 25_000;

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

        crate::ai::parse_connection_test(&output, model, "OpenAI")
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
    incomplete_details: Option<IncompleteDetails>,
    #[serde(default)]
    output: Vec<ResponseOutputItem>,
}

#[derive(Debug, Deserialize)]
struct IncompleteDetails {
    #[serde(default)]
    reason: Option<String>,
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

/// `max_output_tokens` is a ceiling on reasoning tokens as well as visible
/// output, so a reasoning model can exhaust the budget on a short document.
/// Blaming the document size would send the user down the wrong path.
fn incomplete_error(details: Option<&IncompleteDetails>) -> AppError {
    match details.and_then(|details| details.reason.as_deref()) {
        Some("max_output_tokens") => AppError::AiIncomplete(
            "The model used its whole output budget before finishing, which reasoning models can \
             do on even a short document. Try a smaller section, or a model that spends less of \
             the budget on reasoning."
                .into(),
        ),
        Some("content_filter") => AppError::AiIncomplete(
            "OpenAI stopped this response with its content filter.".into(),
        ),
        _ => AppError::AiIncomplete("OpenAI did not finish the response. Try again.".into()),
    }
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
        return Err(incomplete_error(response.incomplete_details.as_ref()));
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
        // Supported effort values differ by model family, and any custom model
        // ID is allowed, so the app never guesses one.
        assert!(request.get("reasoning").is_none());
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
    fn an_exhausted_output_budget_does_not_blame_the_document_size() {
        let exhausted = response(json!({
            "status": "incomplete",
            "incomplete_details": { "reason": "max_output_tokens" },
            "output": []
        }));

        let error = structured_output(&exhausted).unwrap_err();
        assert_eq!(error.kind(), "ai_incomplete");
        assert!(error.to_string().contains("output budget"));
        assert!(!error.to_string().contains("smaller document"));

        let filtered = response(json!({
            "status": "incomplete",
            "incomplete_details": { "reason": "content_filter" },
            "output": []
        }));
        assert!(structured_output(&filtered)
            .unwrap_err()
            .to_string()
            .contains("content filter"));
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
