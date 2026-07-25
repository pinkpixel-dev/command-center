use rusqlite::{params, Connection};

use crate::db::{now, search};
use crate::error::{AppError, AppResult};
use crate::models::{Collection, CollectionInput, CollectionRef};

/// All collections with their entry counts, newest activity first.
pub fn list(conn: &Connection) -> AppResult<Vec<Collection>> {
    let mut statement = conn.prepare(
        "SELECT c.id, c.name, c.description, COUNT(cc.command_id) AS command_count,
                c.created_at, c.updated_at
         FROM collections c
         LEFT JOIN command_collections cc ON cc.collection_id = c.id
         GROUP BY c.id, c.name, c.description, c.created_at, c.updated_at
         ORDER BY c.name COLLATE NOCASE ASC",
    )?;

    let collections = statement
        .query_map([], |row| {
            Ok(Collection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                command_count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<Collection>, _>>()?;

    Ok(collections)
}

pub fn create(conn: &Connection, input: CollectionInput) -> AppResult<i64> {
    let input = input.normalized()?;
    let timestamp = now();

    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM collections WHERE name = ?1 COLLATE NOCASE",
            params![input.name],
            |row| row.get(0),
        )
        .ok();

    if existing.is_some() {
        return Err(AppError::invalid(format!(
            "A collection named \"{}\" already exists",
            input.name
        )));
    }

    conn.execute(
        "INSERT INTO collections (name, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)",
        params![input.name, input.description, timestamp],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, input: CollectionInput) -> AppResult<()> {
    let input = input.normalized()?;

    let clashes: bool = conn.query_row(
        "SELECT EXISTS (
             SELECT 1 FROM collections WHERE name = ?1 COLLATE NOCASE AND id <> ?2
         )",
        params![input.name, id],
        |row| row.get(0),
    )?;

    if clashes {
        return Err(AppError::invalid(format!(
            "A collection named \"{}\" already exists",
            input.name
        )));
    }

    let changed = conn.execute(
        "UPDATE collections SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
        params![input.name, input.description, now(), id],
    )?;

    if changed == 0 {
        return Err(AppError::not_found("That collection"));
    }

    reindex_members(conn, id)?;
    Ok(())
}

/// Deletes the collection itself. Entries inside it are kept; only the
/// membership rows disappear.
pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let members = member_ids(conn, id)?;

    let changed = conn.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::not_found("That collection"));
    }

    for command_id in members {
        search::reindex(conn, command_id)?;
    }

    Ok(())
}

/// Collections one entry belongs to.
pub fn for_command(conn: &Connection, command_id: i64) -> AppResult<Vec<CollectionRef>> {
    let mut statement = conn.prepare(
        "SELECT c.id, c.name FROM command_collections cc
         JOIN collections c ON c.id = cc.collection_id
         WHERE cc.command_id = ?1
         ORDER BY c.name COLLATE NOCASE",
    )?;

    let refs = statement
        .query_map(params![command_id], |row| {
            Ok(CollectionRef {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<CollectionRef>, _>>()?;

    Ok(refs)
}

/// Replaces an entry's collection membership. Unknown ids are ignored rather
/// than failing the whole save.
pub fn set_for_command(conn: &Connection, command_id: i64, collection_ids: &[i64]) -> AppResult<()> {
    conn.execute(
        "DELETE FROM command_collections WHERE command_id = ?1",
        params![command_id],
    )?;

    for collection_id in collection_ids {
        conn.execute(
            "INSERT OR IGNORE INTO command_collections (command_id, collection_id)
             SELECT ?1, id FROM collections WHERE id = ?2",
            params![command_id, collection_id],
        )?;
    }

    Ok(())
}

fn member_ids(conn: &Connection, collection_id: i64) -> AppResult<Vec<i64>> {
    let mut statement =
        conn.prepare("SELECT command_id FROM command_collections WHERE collection_id = ?1")?;
    let ids = statement
        .query_map(params![collection_id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<i64>, _>>()?;
    Ok(ids)
}

/// A renamed collection changes what its entries match on, so their search rows
/// need rebuilding.
fn reindex_members(conn: &Connection, collection_id: i64) -> AppResult<()> {
    for command_id in member_ids(conn, collection_id)? {
        search::reindex(conn, command_id)?;
    }
    Ok(())
}
