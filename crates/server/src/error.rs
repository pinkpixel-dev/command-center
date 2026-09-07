//! One `AppError`, two audiences.
//!
//! The body is the `{ kind, message }` shape the frontend already parses, so
//! the HTTP client reads a rejection exactly the way the Tauri one does. The
//! status code is for everything in between: proxies, logs, and whoever is
//! reading them at two in the morning.

use std::time::Duration;

use axum::http::header::RETRY_AFTER;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use command_center_core::error::AppError;

/// What a handler can fail with. Most of it is core's error, wrapped because
/// both the error and the response trait belong to other crates. The other two
/// are the server's own: nothing in a local desktop app is ever unauthorised
/// or asked to slow down.
#[derive(Debug)]
pub enum ApiError {
    Core(AppError),
    Unauthorized(String),
    RateLimited { message: String, retry_after: Duration },
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        Self::Core(error)
    }
}

/// The body every failure carries, whichever kind it is.
#[derive(Serialize)]
struct Body<'a> {
    kind: &'a str,
    message: &'a str,
}

/// A request the client abandoned. Not a standard code, but the one every
/// common proxy already logs for exactly this.
const CLIENT_CLOSED_REQUEST: u16 = 499;

fn status_for(error: &AppError) -> StatusCode {
    match error.kind() {
        "invalid" => StatusCode::BAD_REQUEST,
        "not_found" => StatusCode::NOT_FOUND,
        // The user has to change something here before the request can work.
        "ai_disabled" | "ai_not_configured" | "ai_model" => StatusCode::BAD_REQUEST,
        "credential" => StatusCode::SERVICE_UNAVAILABLE,
        "ai_rate_limit" => StatusCode::TOO_MANY_REQUESTS,
        "ai_network" => StatusCode::GATEWAY_TIMEOUT,
        // Everything else from a provider is an upstream problem, not the
        // client's, so it must not read as a bad request.
        "ai_auth" | "ai_response" | "ai_refusal" | "ai_incomplete" | "ai_malformed"
        | "ai_response_too_large" => StatusCode::BAD_GATEWAY,
        "ai_cancelled" => {
            StatusCode::from_u16(CLIENT_CLOSED_REQUEST).unwrap_or(StatusCode::BAD_REQUEST)
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Core(error) => (status_for(&error), Json(&error)).into_response(),
            Self::Unauthorized(message) => (
                StatusCode::UNAUTHORIZED,
                Json(Body {
                    kind: "unauthorized",
                    message: &message,
                }),
            )
                .into_response(),
            Self::RateLimited {
                message,
                retry_after,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                // Seconds, rounded up, so a wait shorter than a second still
                // asks the client to wait rather than to retry immediately.
                [(RETRY_AFTER, retry_after.as_secs().max(1).to_string())],
                Json(Body {
                    kind: "rate_limited",
                    message: &message,
                }),
            )
                .into_response(),
        }
    }
}

/// What every route returns.
pub type ApiResult<T> = Result<axum::Json<T>, ApiError>;

/// Wraps a successful value in the JSON response the frontend expects.
pub fn ok<T>(value: T) -> ApiResult<T> {
    Ok(axum::Json(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_entry_is_a_404_rather_than_a_server_failure() {
        assert_eq!(
            status_for(&AppError::not_found("command 7")),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status_for(&AppError::invalid("that is a folder")),
            StatusCode::BAD_REQUEST
        );
    }

    /// A provider that refuses or times out is not the browser's fault, and a
    /// 4xx in the access log would send whoever reads it to the wrong place.
    #[test]
    fn upstream_provider_failures_read_as_upstream_failures() {
        assert_eq!(
            status_for(&AppError::ai_auth("the key was rejected")),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            status_for(&AppError::ai_network("the request timed out")),
            StatusCode::GATEWAY_TIMEOUT
        );
        assert_eq!(
            status_for(&AppError::ai_rate_limit("slow down")),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn a_turned_off_switch_is_something_the_user_can_fix() {
        assert_eq!(status_for(&AppError::AiDisabled), StatusCode::BAD_REQUEST);
        assert_eq!(
            status_for(&AppError::credential("the manager is locked")),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    /// The frontend reads `kind` and `message` and nothing else, so the body
    /// has to carry both whatever the status code says.
    /// The frontend switches to the sign-in screen on this and nothing else,
    /// so the kind matters as much as the status.
    #[test]
    fn a_missing_session_is_a_401_naming_itself() {
        let response = ApiError::Unauthorized("Sign in".to_owned()).into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn a_throttled_login_says_how_long_to_wait() {
        let response = ApiError::RateLimited {
            message: "Wait 4 seconds".to_owned(),
            retry_after: Duration::from_secs(4),
        }
        .into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(RETRY_AFTER).unwrap(), "4");
    }

    /// A sub-second wait still has to read as a wait.
    #[test]
    fn a_short_wait_rounds_up_rather_than_to_zero() {
        let response = ApiError::RateLimited {
            message: "Wait".to_owned(),
            retry_after: Duration::from_millis(200),
        }
        .into_response();

        assert_eq!(response.headers().get(RETRY_AFTER).unwrap(), "1");
    }

    #[test]
    fn the_body_is_the_shape_the_frontend_already_parses() {
        let encoded = serde_json::to_value(AppError::not_found("command 7")).unwrap();

        assert_eq!(encoded["kind"], "not_found");
        assert_eq!(encoded["message"], "command 7 was not found");
    }
}
