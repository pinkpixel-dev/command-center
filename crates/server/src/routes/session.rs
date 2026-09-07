//! Signing in, signing out, and asking which of the two applies.
//!
//! These are the only routes in front of the session guard, so a browser that
//! has never signed in can still load the page and find out that it has to.

use axum::extract::State;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::auth::{cookie, is_secure, SESSION_LIFETIME};
use crate::error::ApiError;
use crate::json::Json;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/session", post(session))
        .route("/login", post(login))
        .route("/logout", post(logout))
}

#[derive(Deserialize)]
pub struct LoginBody {
    password: String,
}

/// The only thing the frontend needs before it decides what to render.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    authenticated: bool,
}

fn token_in(headers: &HeaderMap) -> Option<&str> {
    cookie::token_from(headers.get(COOKIE).and_then(|value| value.to_str().ok()))
}

async fn session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let authenticated = token_in(&headers).is_some_and(|token| state.auth.sessions.renew(token));

    axum::Json(SessionState { authenticated }).into_response()
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> Result<Response, ApiError> {
    // Checked before the password is looked at, so guessing costs the wait
    // whether or not the guess was close.
    if let Some(remaining) = state.auth.guard.wait() {
        let seconds = remaining.as_secs().max(1);
        return Err(ApiError::RateLimited {
            message: format!(
                "Too many wrong passwords. Try again in {seconds} second{}.",
                if seconds == 1 { "" } else { "s" }
            ),
            retry_after: remaining,
        });
    }

    if !state.auth.password.matches(&body.password) {
        state.auth.guard.failed();
        // Nothing about what was wrong with it, because there is only one
        // account and only one thing it could be.
        return Err(ApiError::Unauthorized(
            "That password was not right.".to_owned(),
        ));
    }

    state.auth.guard.succeeded();
    let token = state.auth.sessions.start();

    Ok((
        [(
            SET_COOKIE,
            cookie::grant(&token, SESSION_LIFETIME, is_secure(&headers)),
        )],
        axum::Json(SessionState {
            authenticated: true,
        }),
    )
        .into_response())
}

/// Ends the session the cookie names and clears the cookie. Signing out when
/// already signed out is a success, because the outcome the caller asked for
/// is the outcome they get.
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = token_in(&headers) {
        state.auth.sessions.end(token);
    }

    (
        [(SET_COOKIE, cookie::revoke(is_secure(&headers)))],
        axum::Json(SessionState {
            authenticated: false,
        }),
    )
        .into_response()
}
