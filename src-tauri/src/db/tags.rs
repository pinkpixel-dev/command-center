use rusqlite::{params, Connection};

use crate::db::now;
use crate::error::AppResult;
use crate::models::Tag;

/// Every tag in use, with how many entries carry it.
pub fn list(conn: &Connection) -> AppResult<Vec<Tag>> {
    let mut statement = conn.prepare(
        "SELECT t.id, t.name, COUNT(ct.command_id) AS command_count
         FROM tags t
         LEFT JOIN command_tags ct ON ct.tag_id = t.id
         GROUP BY t.id, t.name
         HAVING command_count > 0
         ORDER BY command_count DESC, t.name ASC",
    )?;

    let tags = statement
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                command_count: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<Tag>, _>>()?;

    Ok(tags)
}

/// Names attached to one entry, alphabetically.
pub fn for_command(conn: &Connection, command_id: i64) -> AppResult<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT t.name FROM command_tags ct
         JOIN tags t ON t.id = ct.tag_id
         WHERE ct.command_id = ?1
         ORDER BY t.name",
    )?;

    let names = statement
        .query_map(params![command_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<String>, _>>()?;

    Ok(names)
}

/// Replaces an entry's tags with exactly the names given.
pub fn set_for_command(conn: &Connection, command_id: i64, names: &[String]) -> AppResult<()> {
    conn.execute(
        "DELETE FROM command_tags WHERE command_id = ?1",
        params![command_id],
    )?;

    for name in names {
        let tag_id = ensure(conn, name)?;
        conn.execute(
            "INSERT OR IGNORE INTO command_tags (command_id, tag_id) VALUES (?1, ?2)",
            params![command_id, tag_id],
        )?;
    }

    prune_orphans(conn)?;
    Ok(())
}

/// Looks up a tag by name, creating it when it is new.
pub fn ensure(conn: &Connection, name: &str) -> AppResult<i64> {
    let name = name.trim().to_lowercase();
    conn.execute(
        "INSERT OR IGNORE INTO tags (name, created_at) VALUES (?1, ?2)",
        params![name, now()],
    )?;
    let id = conn.query_row("SELECT id FROM tags WHERE name = ?1", params![name], |row| {
        row.get(0)
    })?;
    Ok(id)
}

/// Removes tags nothing points at any more, so the sidebar stays honest.
pub fn prune_orphans(conn: &Connection) -> AppResult<usize> {
    let removed = conn.execute(
        "DELETE FROM tags
         WHERE id NOT IN (SELECT DISTINCT tag_id FROM command_tags)",
        [],
    )?;
    Ok(removed)
}
