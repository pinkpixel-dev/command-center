//! The JSON extractor every route uses.
//!
//! Axum's own rejects a body it cannot read with plain text, which is the one
//! failure on this server that would not reach the frontend as the
//! `{ kind, message }` shape it parses. This is axum's extractor with that
//! rejection translated.

use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};

use command_center_core::error::AppError;

use crate::error::ApiError;

pub struct Json<T>(pub T);

impl<T, S> FromRequest<S> for Json<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        axum::Json::<T>::from_request(request, state)
            .await
            .map(|axum::Json(value)| Self(value))
            .map_err(|rejection| {
                // Only a client the server does not match can produce this, so
                // axum's description of what was wrong is the useful part.
                ApiError::Core(AppError::invalid(format!(
                    "The server could not read that request. {}",
                    rejection.body_text()
                )))
            })
    }
}
