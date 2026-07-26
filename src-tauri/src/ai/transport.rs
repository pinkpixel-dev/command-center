//! HTTP plumbing shared by every OpenAI request: bounded response reading and
//! provider error mapping. Nothing here ever touches the API key.

use reqwest::StatusCode;
use serde::Deserialize;

use crate::error::{AppError, AppResult};

/// A structured response bigger than this is not something the app can use, and
/// reading it would only waste memory.
pub const MAX_RESPONSE_BYTES: usize = 512 * 1024;

#[derive(Debug, Deserialize)]
pub struct ProviderErrorEnvelope {
    pub error: Option<ProviderError>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderError {
    #[serde(default)]
    pub code: Option<String>,
}

pub fn map_transport_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() && error.is_connect() {
        AppError::ai_network("Connecting to OpenAI timed out. Try again.")
    } else if error.is_timeout() {
        AppError::ai_network("OpenAI did not respond before the request timed out. Try again.")
    } else {
        AppError::ai_network("Could not reach OpenAI. Check the network connection and try again.")
    }
}

pub async fn read_bounded_body(response: &mut reqwest::Response) -> AppResult<Vec<u8>> {
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

pub fn append_response_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> AppResult<()> {
    if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
        return Err(AppError::AiResponseTooLarge);
    }
    body.extend_from_slice(chunk);
    Ok(())
}

/// Turns a failed provider response into an error the frontend can act on. The
/// message never carries the request body, the key, or the raw provider text.
pub fn map_provider_error(status: StatusCode, body: &[u8]) -> AppError {
    let parsed = serde_json::from_slice::<ProviderErrorEnvelope>(body)
        .ok()
        .and_then(|envelope| envelope.error);
    let code = parsed.as_ref().and_then(|details| details.code.as_deref());

    if code == Some("model_not_found") {
        return AppError::ai_model(
            "The selected model was not found or is not available to this OpenAI account.",
        );
    }
    if code == Some("context_length_exceeded") {
        return AppError::ai_response(
            "The document was longer than the selected model accepts. Send a smaller section.",
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
        _ => AppError::ai_response("OpenAI rejected the request."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_failures_have_distinct_error_kinds() {
        let model = br#"{"error":{"code":"model_not_found"}}"#;
        assert_eq!(
            map_provider_error(StatusCode::NOT_FOUND, model).kind(),
            "ai_model"
        );

        let context = br#"{"error":{"code":"context_length_exceeded"}}"#;
        assert_eq!(
            map_provider_error(StatusCode::BAD_REQUEST, context).kind(),
            "ai_response"
        );

        assert_eq!(
            map_provider_error(StatusCode::UNAUTHORIZED, b"").kind(),
            "ai_auth"
        );
        assert_eq!(
            map_provider_error(StatusCode::TOO_MANY_REQUESTS, b"").kind(),
            "ai_rate_limit"
        );
        assert_eq!(
            map_provider_error(StatusCode::INTERNAL_SERVER_ERROR, b"").kind(),
            "ai_response"
        );
    }

    #[test]
    fn provider_messages_never_repeat_the_provider_payload() {
        let body = br#"{"error":{"code":"invalid_api_key","message":"sk-not-a-real-key"}}"#;
        let error = map_provider_error(StatusCode::UNAUTHORIZED, body);
        assert!(!error.to_string().contains("sk-not-a-real-key"));
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
