//! Everything the frontend calls to read or change the library.
//!
//! Reads go straight to `db`. Writes go through `core::library`, which pairs
//! each one with the change notification the server also has to send.

use tauri::{AppHandle, State};

use crate::db::query::ListQuery;
use crate::db::{collections as collections_db, commands as commands_db, tags as tags_db, Database};
use crate::error::AppResult;
use crate::events::DesktopEvents;
use crate::library;
use crate::models::{Collection, CollectionInput, Command, CommandInput, LibraryStats, Tag};

#[tauri::command]
pub fn list_commands(db: State<'_, Database>, filter: ListQuery) -> AppResult<Vec<Command>> {
    db.with(|conn| commands_db::list(conn, &filter))
}

#[tauri::command]
pub fn get_command(db: State<'_, Database>, id: i64) -> AppResult<Command> {
    db.with(|conn| commands_db::get(conn, id))
}

#[tauri::command]
pub fn create_command(
    app: AppHandle,
    db: State<'_, Database>,
    input: CommandInput,
) -> AppResult<Command> {
    library::create_command(&db, &DesktopEvents(app), input)
}

#[tauri::command]
pub fn update_command(
    app: AppHandle,
    db: State<'_, Database>,
    id: i64,
    input: CommandInput,
) -> AppResult<Command> {
    library::update_command(&db, &DesktopEvents(app), id, input)
}

#[tauri::command]
pub fn delete_command(app: AppHandle, db: State<'_, Database>, id: i64) -> AppResult<()> {
    library::delete_command(&db, &DesktopEvents(app), id)
}

#[tauri::command]
pub fn delete_commands(
    app: AppHandle,
    db: State<'_, Database>,
    command_ids: Vec<i64>,
) -> AppResult<()> {
    library::delete_commands(&db, &DesktopEvents(app), &command_ids)
}

#[tauri::command]
pub fn toggle_favorite(app: AppHandle, db: State<'_, Database>, id: i64) -> AppResult<bool> {
    library::toggle_favorite(&db, &DesktopEvents(app), id)
}

/// Called after the frontend puts a command on the clipboard.
#[tauri::command]
pub fn record_copy(app: AppHandle, db: State<'_, Database>, id: i64) -> AppResult<Command> {
    library::record_copy(&db, &DesktopEvents(app), id)
}

/// Looks for an entry with the same normalized content.
#[tauri::command]
pub fn find_duplicate(db: State<'_, Database>, content: String) -> AppResult<Option<Command>> {
    db.with(|conn| commands_db::find_by_content(conn, &content))
}

#[tauri::command]
pub fn library_stats(db: State<'_, Database>) -> AppResult<LibraryStats> {
    db.with(commands_db::stats)
}

#[tauri::command]
pub fn list_tags(db: State<'_, Database>) -> AppResult<Vec<Tag>> {
    db.with(tags_db::list)
}

#[tauri::command]
pub fn list_collections(db: State<'_, Database>) -> AppResult<Vec<Collection>> {
    db.with(collections_db::list)
}

#[tauri::command]
pub fn create_collection(
    app: AppHandle,
    db: State<'_, Database>,
    input: CollectionInput,
) -> AppResult<Vec<Collection>> {
    library::create_collection(&db, &DesktopEvents(app), input)
}

#[tauri::command]
pub fn update_collection(
    app: AppHandle,
    db: State<'_, Database>,
    id: i64,
    input: CollectionInput,
) -> AppResult<Vec<Collection>> {
    library::update_collection(&db, &DesktopEvents(app), id, input)
}

#[tauri::command]
pub fn delete_collection(
    app: AppHandle,
    db: State<'_, Database>,
    id: i64,
) -> AppResult<Vec<Collection>> {
    library::delete_collection(&db, &DesktopEvents(app), id)
}

#[tauri::command]
pub fn add_commands_to_collection(
    app: AppHandle,
    db: State<'_, Database>,
    command_ids: Vec<i64>,
    collection_id: i64,
) -> AppResult<()> {
    library::add_commands_to_collection(&db, &DesktopEvents(app), &command_ids, collection_id)
}
