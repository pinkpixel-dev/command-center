//! The route table.
//!
//! Every command the frontend already calls becomes `POST /api/<name>` with
//! the same argument object as its body, so the browser client is the Tauri
//! client with `invoke` swapped for `fetch`. Reads are POSTs too: the app's
//! boundary is a set of calls with structured arguments, not a set of
//! documents at addressable URLs, and pretending otherwise would mean encoding
//! a nested filter object into a query string for no gain.

pub mod ai;
pub mod codex;
pub mod import;
pub mod library;
pub mod system;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::convert::Infallible;
use tokio_stream::Stream;

use crate::events;
use crate::state::AppState;

/// What the container's health check reads.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    status: &'static str,
    version: &'static str,
    schema_version: i64,
}

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .merge(library::routes())
        .merge(import::routes())
        .merge(system::routes())
        .merge(ai::routes())
        .merge(codex::routes())
        .route("/events", get(feed))
        .route("/health", get(health));

    Router::new().nest("/api", api).with_state(state)
}

/// The live feed the frontend subscribes to instead of Tauri's `listen`.
async fn feed(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    events::feed(state.events.subscribe())
}

/// Reports the schema version as well as liveness, because a server that
/// answers while its migrations failed is not actually healthy.
async fn health(State(state): State<AppState>) -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        schema_version: state.db.schema_version().unwrap_or(-1),
    })
}
