//! Everything the frontend calls to read or change the library.

use tauri::{AppHandle, Emitter, State};

use crate::db::query::ListQuery;
use crate::db::{
    collections as collections_db, commands as commands_db, tags as tags_db, Database,
};
use crate::error::AppResult;
use crate::models::{Collection, CollectionInput, Command, CommandInput, LibraryStats, Tag};

/// Event the frontend listens to so library changes refresh the current view.
pub const LIBRARY_CHANGED: &str = "library-changed";

fn announce(app: &AppHandle) {
    // A failed notification should never fail the write that triggered it.
    let _ = app.emit(LIBRARY_CHANGED, ());
}

/// Lets other command modules, like the importer, refresh the library view.
pub fn announce_change(app: &AppHandle) {
    announce(app);
}

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
    let created = db.with_mut(|conn| commands_db::create(conn, input))?;
    announce(&app);
    Ok(created)
}

#[tauri::command]
pub fn update_command(
    app: AppHandle,
    db: State<'_, Database>,
    id: i64,
    input: CommandInput,
) -> AppResult<Command> {
    let updated = db.with_mut(|conn| commands_db::update(conn, id, input))?;
    announce(&app);
    Ok(updated)
}

#[tauri::command]
pub fn delete_command(app: AppHandle, db: State<'_, Database>, id: i64) -> AppResult<()> {
    db.with_mut(|conn| commands_db::delete(conn, id))?;
    announce(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_commands(
    app: AppHandle,
    db: State<'_, Database>,
    command_ids: Vec<i64>,
) -> AppResult<()> {
    db.with_mut(|conn| commands_db::delete_many(conn, &command_ids))?;
    announce(&app);
    Ok(())
}

#[tauri::command]
pub fn toggle_favorite(app: AppHandle, db: State<'_, Database>, id: i64) -> AppResult<bool> {
    let favorite = db.with(|conn| commands_db::toggle_favorite(conn, id))?;
    announce(&app);
    Ok(favorite)
}

/// Called after the frontend puts a command on the clipboard.
#[tauri::command]
pub fn record_copy(app: AppHandle, db: State<'_, Database>, id: i64) -> AppResult<Command> {
    let updated = db.with(|conn| commands_db::record_copy(conn, id))?;
    announce(&app);
    Ok(updated)
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
    let list = db.with(|conn| {
        collections_db::create(conn, input)?;
        collections_db::list(conn)
    })?;
    announce(&app);
    Ok(list)
}

#[tauri::command]
pub fn update_collection(
    app: AppHandle,
    db: State<'_, Database>,
    id: i64,
    input: CollectionInput,
) -> AppResult<Vec<Collection>> {
    let list = db.with(|conn| {
        collections_db::update(conn, id, input)?;
        collections_db::list(conn)
    })?;
    announce(&app);
    Ok(list)
}

#[tauri::command]
pub fn delete_collection(
    app: AppHandle,
    db: State<'_, Database>,
    id: i64,
) -> AppResult<Vec<Collection>> {
    let list = db.with(|conn| {
        collections_db::delete(conn, id)?;
        collections_db::list(conn)
    })?;
    announce(&app);
    Ok(list)
}

#[tauri::command]
pub fn add_commands_to_collection(
    app: AppHandle,
    db: State<'_, Database>,
    command_ids: Vec<i64>,
    collection_id: i64,
) -> AppResult<()> {
    db.with_mut(|conn| {
        collections_db::add_commands_to_collection(conn, &command_ids, collection_id)
    })?;
    announce(&app);
    Ok(())
}
