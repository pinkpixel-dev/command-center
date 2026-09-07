//! Sign-in for the self-hosted server.
//!
//! The desktop app has no concept of a user because it does not need one: the
//! operating system already decided who is at the keyboard. A server holding
//! the same library on a NAS did not, so it asks for a password once and
//! remembers the answer in a cookie.

pub mod cookie;
pub mod guard;
pub mod password;
pub mod sessions;

use axum::extract::{Request, State};
use axum::http::header::{COOKIE, HeaderMap};
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;
use crate::state::AppState;

pub use guard::LoginGuard;
pub use password::Password;
pub use sessions::{Sessions, SESSION_LIFETIME};

/// Everything the login route and the guard in front of the API need.
pub struct Auth {
    pub password: Password,
    pub sessions: Sessions,
    pub guard: LoginGuard,
}

impl Auth {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            password: Password::from_env()?,
            sessions: Sessions::default(),
            guard: LoginGuard::default(),
        })
    }
}

/// Whether the browser reached the server over HTTPS. The server speaks plain
/// HTTP and expects a reverse proxy to hold the certificate, so the proxy's
/// header is the only thing that knows.
pub fn is_secure(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        // A proxy chain sends a list, and the first entry is the one the
        // browser actually spoke.
        .map(|value| value.split(',').next().unwrap_or("").trim().eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

/// Stands in front of everything except health and the sign-in routes.
pub async fn require_session(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Scoped so the borrow of the headers ends before the request is handed on.
    let signed_in = {
        let header = request
            .headers()
            .get(COOKIE)
            .and_then(|value| value.to_str().ok());

        cookie::token_from(header).is_some_and(|token| state.auth.sessions.renew(token))
    };

    if signed_in {
        Ok(next.run(request).await)
    } else {
        Err(ApiError::Unauthorized(
            "Sign in to use this Command Center.".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(proto: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(value) = proto {
            headers.insert("x-forwarded-proto", HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    #[test]
    fn a_plain_lan_deployment_is_not_treated_as_https() {
        assert!(!is_secure(&headers(None)));
        assert!(!is_secure(&headers(Some("http"))));
    }

    #[test]
    fn a_proxy_holding_the_certificate_is() {
        assert!(is_secure(&headers(Some("https"))));
        assert!(is_secure(&headers(Some("HTTPS"))));
        // A chain of proxies reports every hop; the browser spoke to the first.
        assert!(is_secure(&headers(Some("https, http"))));
    }
}
