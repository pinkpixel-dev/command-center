//! Reading and writing entries. Writes go through a transaction so an entry,
//! its tags, its collections and its search row can never drift apart.

use std::collections::{BTreeSet, HashMap};

use rusqlite::{params, params_from_iter, Connection, Row};

use crate::db::query::{self, ListQuery};
use crate::db::{collections, now, search, tags};
use crate::error::{AppError, AppResult};
use crate::models::{CollectionRef, Command, CommandInput, CommandKind, LibraryStats, RiskLevel};
use crate::normalize::{content_hash, extract_variables};
use crate::risk;

/// Returns the entries matching a filter, with tags and collections attached.
pub fn list(conn: &Connection, filter: &ListQuery) -> AppResult<Vec<Command>> {
    let built = query::build_list(filter);
    let mut statement = conn.prepare(&built.sql)?;
    let rows = statement
        .query_map(params_from_iter(built.params.iter()), map_row)?
        .collect::<Result<Vec<PartialCommand>, _>>()?;

    hydrate(conn, rows)
}

/// Every entry without the interactive library's result limit, used for a
/// complete export rather than a screen-sized query.
pub fn list_all(conn: &Connection) -> AppResult<Vec<Command>> {
    let sql = format!(
        "SELECT {} FROM commands c ORDER BY c.title COLLATE NOCASE ASC, c.id ASC",
        query::COLUMNS
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement
        .query_map([], map_row)?
        .collect::<Result<Vec<PartialCommand>, _>>()?;

    hydrate(conn, rows)
}

/// Every entry in one collection without the interactive library's result
/// limit, used for a complete collection export.
pub fn list_for_collection(conn: &Connection, collection_id: i64) -> AppResult<Vec<Command>> {
    let sql = format!(
        "SELECT {} FROM commands c
         WHERE EXISTS (
             SELECT 1 FROM command_collections cc
             WHERE cc.command_id = c.id AND cc.collection_id = ?1
         )
         ORDER BY c.title COLLATE NOCASE ASC, c.id ASC",
        query::COLUMNS
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement
        .query_map(params![collection_id], map_row)?
        .collect::<Result<Vec<PartialCommand>, _>>()?;

    hydrate(conn, rows)
}

/// One entry by id.
pub fn get(conn: &Connection, id: i64) -> AppResult<Command> {
    let sql = format!("SELECT {} FROM commands c WHERE c.id = ?1", query::COLUMNS);
    let partial = conn
        .query_row(&sql, params![id], map_row)
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => AppError::not_found("That command"),
            other => other.into(),
        })?;

    hydrate(conn, vec![partial])?
        .pop()
        .ok_or_else(|| AppError::not_found("That command"))
}

/// Creates an entry and returns the stored version.
pub fn create(conn: &mut Connection, input: CommandInput) -> AppResult<Command> {
    let input = input.normalized()?;
    let timestamp = now();
    let hash = content_hash(&input.content);
    let risk_level = input
        .risk_level
        .unwrap_or_else(|| risk::assess(&input.content).0);

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO commands (
            title, content, description, kind, language, shell, operating_system,
            risk_level, favorite, working_directory, source_url, notes, content_hash,
            copy_count, last_copied_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0, NULL, ?14, ?14)",
        params![
            input.title,
            input.content,
            input.description,
            input.kind.as_str(),
            input.language,
            input.shell,
            input.operating_system,
            risk_level.as_str(),
            input.favorite as i64,
            input.working_directory,
            input.source_url,
            input.notes,
            hash,
            timestamp,
        ],
    )?;

    let id = tx.last_insert_rowid();
    tags::set_for_command(&tx, id, &input.tags)?;
    collections::set_for_command(&tx, id, &input.collection_ids)?;
    search::reindex(&tx, id)?;
    tx.commit()?;

    get(conn, id)
}

/// Updates every editable field of an entry.
pub fn update(conn: &mut Connection, id: i64, input: CommandInput) -> AppResult<Command> {
    let input = input.normalized()?;
    let hash = content_hash(&input.content);
    let risk_level = input
        .risk_level
        .unwrap_or_else(|| risk::assess(&input.content).0);

    let tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE commands SET
            title = ?1, content = ?2, description = ?3, kind = ?4, language = ?5,
            shell = ?6, operating_system = ?7, risk_level = ?8, favorite = ?9,
            working_directory = ?10, source_url = ?11, notes = ?12, content_hash = ?13,
            updated_at = ?14
         WHERE id = ?15",
        params![
            input.title,
            input.content,
            input.description,
            input.kind.as_str(),
            input.language,
            input.shell,
            input.operating_system,
            risk_level.as_str(),
            input.favorite as i64,
            input.working_directory,
            input.source_url,
            input.notes,
            hash,
            now(),
            id,
        ],
    )?;

    if changed == 0 {
        return Err(AppError::not_found("That command"));
    }

    tags::set_for_command(&tx, id, &input.tags)?;
    collections::set_for_command(&tx, id, &input.collection_ids)?;
    search::reindex(&tx, id)?;
    tx.commit()?;

    get(conn, id)
}

pub fn delete(conn: &mut Connection, id: i64) -> AppResult<()> {
    let tx = conn.transaction()?;
    let changed = tx.execute("DELETE FROM commands WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::not_found("That command"));
    }
    search::remove(&tx, id)?;
    tags::prune_orphans(&tx)?;
    tx.commit()?;
    Ok(())
}

/// Deletes an explicitly selected set as one transaction. The complete
/// selection is checked first so one stale id cannot leave a partial delete.
pub fn delete_many(conn: &mut Connection, command_ids: &[i64]) -> AppResult<usize> {
    let tx = conn.transaction()?;
    let command_ids = validated_selection(&tx, command_ids)?;

    for command_id in &command_ids {
        tx.execute("DELETE FROM commands WHERE id = ?1", params![command_id])?;
        search::remove(&tx, *command_id)?;
    }
    tags::prune_orphans(&tx)?;
    tx.commit()?;

    Ok(command_ids.len())
}

/// Deduplicates a selected id list and proves every row exists before a bulk
/// write begins. Kept here so collection membership and deletion agree on what
/// an explicit selection means.
pub(crate) fn validated_selection(conn: &Connection, command_ids: &[i64]) -> AppResult<Vec<i64>> {
    let command_ids = command_ids.iter().copied().collect::<BTreeSet<_>>();
    if command_ids.is_empty() {
        return Err(AppError::invalid("Select at least one command"));
    }

    for command_id in &command_ids {
        let exists: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM commands WHERE id = ?1)",
            params![command_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(AppError::not_found(format!("Command {command_id}")));
        }
    }

    Ok(command_ids.into_iter().collect())
}

/// Flips the favorite flag and reports the new state.
pub fn toggle_favorite(conn: &Connection, id: i64) -> AppResult<bool> {
    let changed = conn.execute(
        "UPDATE commands SET favorite = CASE favorite WHEN 1 THEN 0 ELSE 1 END,
                             updated_at = ?1
         WHERE id = ?2",
        params![now(), id],
    )?;

    if changed == 0 {
        return Err(AppError::not_found("That command"));
    }

    let favorite: i64 = conn.query_row(
        "SELECT favorite FROM commands WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;

    Ok(favorite == 1)
}

/// Records that an entry was copied. This is what feeds Recent and the copy
/// counter on the card.
pub fn record_copy(conn: &Connection, id: i64) -> AppResult<Command> {
    let timestamp = now();
    let changed = conn.execute(
        "UPDATE commands SET copy_count = copy_count + 1, last_copied_at = ?1 WHERE id = ?2",
        params![timestamp, id],
    )?;

    if changed == 0 {
        return Err(AppError::not_found("That command"));
    }

    conn.execute(
        "INSERT INTO usage_history (command_id, action, used_at) VALUES (?1, 'copy', ?2)",
        params![id, timestamp],
    )?;

    get(conn, id)
}

/// Finds an existing entry with the same normalized content for duplicate
/// detection.
pub fn find_by_content(conn: &Connection, content: &str) -> AppResult<Option<Command>> {
    let hash = content_hash(content);
    let id: Option<i64> = conn
        .query_row(
            "SELECT id FROM commands WHERE content_hash = ?1 LIMIT 1",
            params![hash],
            |row| row.get(0),
        )
        .ok();

    match id {
        Some(id) => Ok(Some(get(conn, id)?)),
        None => Ok(None),
    }
}

/// Counts behind the sidebar entries.
pub fn stats(conn: &Connection) -> AppResult<LibraryStats> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))?;
    let favorites: i64 = conn.query_row(
        "SELECT COUNT(*) FROM commands WHERE favorite = 1",
        [],
        |row| row.get(0),
    )?;
    let scripts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM commands WHERE kind IN ('script', 'sequence')",
        [],
        |row| row.get(0),
    )?;
    let recent: i64 = conn.query_row(
        "SELECT COUNT(*) FROM commands WHERE last_copied_at IS NOT NULL",
        [],
        |row| row.get(0),
    )?;

    Ok(LibraryStats {
        total,
        favorites,
        scripts,
        recent,
    })
}

/// A row straight from `commands`, before tags and collections are attached.
struct PartialCommand {
    id: i64,
    title: String,
    content: String,
    description: String,
    kind: String,
    language: Option<String>,
    shell: Option<String>,
    operating_system: Option<String>,
    risk_level: String,
    favorite: bool,
    working_directory: Option<String>,
    source_url: Option<String>,
    notes: String,
    content_hash: String,
    copy_count: i64,
    last_copied_at: Option<String>,
    created_at: String,
    updated_at: String,
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<PartialCommand> {
    Ok(PartialCommand {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        description: row.get(3)?,
        kind: row.get(4)?,
        language: row.get(5)?,
        shell: row.get(6)?,
        operating_system: row.get(7)?,
        risk_level: row.get(8)?,
        favorite: row.get::<_, i64>(9)? == 1,
        working_directory: row.get(10)?,
        source_url: row.get(11)?,
        notes: row.get(12)?,
        content_hash: row.get(13)?,
        copy_count: row.get(14)?,
        last_copied_at: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

/// Attaches tags and collections to a batch of rows using two extra queries,
/// rather than two queries per row.
fn hydrate(conn: &Connection, rows: Vec<PartialCommand>) -> AppResult<Vec<Command>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
    let placeholders = vec!["?"; ids.len()].join(", ");

    let mut tag_map: HashMap<i64, Vec<String>> = HashMap::new();
    let mut statement = conn.prepare(&format!(
        "SELECT ct.command_id, t.name FROM command_tags ct
         JOIN tags t ON t.id = ct.tag_id
         WHERE ct.command_id IN ({placeholders})
         ORDER BY t.name",
    ))?;
    let mut cursor = statement.query(params_from_iter(ids.iter()))?;
    while let Some(row) = cursor.next()? {
        tag_map.entry(row.get(0)?).or_default().push(row.get(1)?);
    }

    let mut collection_map: HashMap<i64, Vec<CollectionRef>> = HashMap::new();
    let mut statement = conn.prepare(&format!(
        "SELECT cc.command_id, c.id, c.name FROM command_collections cc
         JOIN collections c ON c.id = cc.collection_id
         WHERE cc.command_id IN ({placeholders})
         ORDER BY c.name COLLATE NOCASE",
    ))?;
    let mut cursor = statement.query(params_from_iter(ids.iter()))?;
    while let Some(row) = cursor.next()? {
        collection_map
            .entry(row.get(0)?)
            .or_default()
            .push(CollectionRef {
                id: row.get(1)?,
                name: row.get(2)?,
            });
    }

    rows.into_iter()
        .map(|row| {
            let risk_level = RiskLevel::parse(&row.risk_level)?;
            Ok(Command {
                risk_reasons: risk::reasons_for(&row.content, risk_level),
                variables: extract_variables(&row.content),
                tags: tag_map.remove(&row.id).unwrap_or_default(),
                collections: collection_map.remove(&row.id).unwrap_or_default(),
                kind: CommandKind::parse(&row.kind)?,
                risk_level,
                id: row.id,
                title: row.title,
                content: row.content,
                description: row.description,
                language: row.language,
                shell: row.shell,
                operating_system: row.operating_system,
                favorite: row.favorite,
                working_directory: row.working_directory,
                source_url: row.source_url,
                notes: row.notes,
                content_hash: row.content_hash,
                copy_count: row.copy_count,
                last_copied_at: row.last_copied_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
        })
        .collect()
}
