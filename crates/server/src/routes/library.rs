//! Reading and changing the library.

use axum::extract::State;
use axum::routing::post;
use axum::Router;
use serde::Deserialize;

use command_center_core::db::query::ListQuery;
use command_center_core::db::{
    collections as collections_db, commands as commands_db, tags as tags_db,
};
use command_center_core::library;
use command_center_core::models::{
    Collection, CollectionInput, Command, CommandInput, LibraryStats, Tag,
};

use crate::error::{ok, ApiResult};
use crate::json::Json;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/list_commands", post(list_commands))
        .route("/get_command", post(get_command))
        .route("/create_command", post(create_command))
        .route("/update_command", post(update_command))
        .route("/delete_command", post(delete_command))
        .route("/delete_commands", post(delete_commands))
        .route("/toggle_favorite", post(toggle_favorite))
        .route("/record_copy", post(record_copy))
        .route("/find_duplicate", post(find_duplicate))
        .route("/library_stats", post(library_stats))
        .route("/list_tags", post(list_tags))
        .route("/list_collections", post(list_collections))
        .route("/create_collection", post(create_collection))
        .route("/update_collection", post(update_collection))
        .route("/delete_collection", post(delete_collection))
        .route(
            "/add_commands_to_collection",
            post(add_commands_to_collection),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterBody {
    #[serde(default)]
    filter: ListQuery,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdBody {
    id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandBody {
    input: CommandInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCommandBody {
    id: i64,
    input: CommandInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandIdsBody {
    command_ids: Vec<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentBody {
    content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionBody {
    input: CollectionInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCollectionBody {
    id: i64,
    input: CollectionInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddToCollectionBody {
    command_ids: Vec<i64>,
    collection_id: i64,
}

async fn list_commands(
    State(state): State<AppState>,
    Json(body): Json<FilterBody>,
) -> ApiResult<Vec<Command>> {
    ok(state.db.with(|conn| commands_db::list(conn, &body.filter))?)
}

async fn get_command(State(state): State<AppState>, Json(body): Json<IdBody>) -> ApiResult<Command> {
    ok(state.db.with(|conn| commands_db::get(conn, body.id))?)
}

async fn create_command(
    State(state): State<AppState>,
    Json(body): Json<CommandBody>,
) -> ApiResult<Command> {
    ok(library::create_command(
        &state.db,
        state.events.as_ref(),
        body.input,
    )?)
}

async fn update_command(
    State(state): State<AppState>,
    Json(body): Json<UpdateCommandBody>,
) -> ApiResult<Command> {
    ok(library::update_command(
        &state.db,
        state.events.as_ref(),
        body.id,
        body.input,
    )?)
}

async fn delete_command(State(state): State<AppState>, Json(body): Json<IdBody>) -> ApiResult<()> {
    ok(library::delete_command(
        &state.db,
        state.events.as_ref(),
        body.id,
    )?)
}

async fn delete_commands(
    State(state): State<AppState>,
    Json(body): Json<CommandIdsBody>,
) -> ApiResult<()> {
    ok(library::delete_commands(
        &state.db,
        state.events.as_ref(),
        &body.command_ids,
    )?)
}

async fn toggle_favorite(State(state): State<AppState>, Json(body): Json<IdBody>) -> ApiResult<bool> {
    ok(library::toggle_favorite(
        &state.db,
        state.events.as_ref(),
        body.id,
    )?)
}

async fn record_copy(State(state): State<AppState>, Json(body): Json<IdBody>) -> ApiResult<Command> {
    ok(library::record_copy(
        &state.db,
        state.events.as_ref(),
        body.id,
    )?)
}

async fn find_duplicate(
    State(state): State<AppState>,
    Json(body): Json<ContentBody>,
) -> ApiResult<Option<Command>> {
    ok(state
        .db
        .with(|conn| commands_db::find_by_content(conn, &body.content))?)
}

async fn library_stats(State(state): State<AppState>) -> ApiResult<LibraryStats> {
    ok(state.db.with(commands_db::stats)?)
}

async fn list_tags(State(state): State<AppState>) -> ApiResult<Vec<Tag>> {
    ok(state.db.with(tags_db::list)?)
}

async fn list_collections(State(state): State<AppState>) -> ApiResult<Vec<Collection>> {
    ok(state.db.with(collections_db::list)?)
}

async fn create_collection(
    State(state): State<AppState>,
    Json(body): Json<CollectionBody>,
) -> ApiResult<Vec<Collection>> {
    ok(library::create_collection(
        &state.db,
        state.events.as_ref(),
        body.input,
    )?)
}

async fn update_collection(
    State(state): State<AppState>,
    Json(body): Json<UpdateCollectionBody>,
) -> ApiResult<Vec<Collection>> {
    ok(library::update_collection(
        &state.db,
        state.events.as_ref(),
        body.id,
        body.input,
    )?)
}

async fn delete_collection(
    State(state): State<AppState>,
    Json(body): Json<IdBody>,
) -> ApiResult<Vec<Collection>> {
    ok(library::delete_collection(
        &state.db,
        state.events.as_ref(),
        body.id,
    )?)
}

async fn add_commands_to_collection(
    State(state): State<AppState>,
    Json(body): Json<AddToCollectionBody>,
) -> ApiResult<()> {
    ok(library::add_commands_to_collection(
        &state.db,
        state.events.as_ref(),
        &body.command_ids,
        body.collection_id,
    )?)
}
