//! Turns the frontend's filter object into one parameterised SQL statement.
//! Kept separate from the row mapping so the shape of a query stays easy to
//! read and easy to test.

use rusqlite::types::ToSql;
use serde::Deserialize;

use crate::db::search;
use crate::models::{CommandKind, RiskLevel};

/// Which slice of the library the user is looking at.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Scope {
    #[default]
    All,
    Favorites,
    Recent,
    Scripts,
    Collection { id: i64 },
    Tag { name: String },
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    #[default]
    Updated,
    Created,
    Title,
    Copies,
    LastCopied,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ListQuery {
    pub search: Option<String>,
    pub scope: Scope,
    pub tags: Vec<String>,
    pub kinds: Vec<CommandKind>,
    pub risk: Option<RiskLevel>,
    pub sort: Sort,
    pub limit: Option<i64>,
}

pub struct BuiltQuery {
    pub sql: String,
    pub params: Vec<Box<dyn ToSql>>,
}

pub const COLUMNS: &str = "c.id, c.title, c.content, c.description, c.kind, c.language, c.shell, \
     c.operating_system, c.risk_level, c.favorite, c.working_directory, c.source_url, c.notes, \
     c.content_hash, c.copy_count, c.last_copied_at, c.created_at, c.updated_at";

const DEFAULT_LIMIT: i64 = 500;

/// Builds the list statement. Text search joins the FTS table so results can be
/// ordered by relevance; everything else is a plain filter on `commands`.
pub fn build_list(query: &ListQuery) -> BuiltQuery {
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    let mut sql = format!("SELECT {COLUMNS} FROM commands c");

    let match_expression = query
        .search
        .as_deref()
        .and_then(search::build_match_query);

    if let Some(expression) = match_expression {
        sql.push_str(
            " JOIN (SELECT CAST(command_id AS INTEGER) AS cid, rank FROM commands_fts \
             WHERE commands_fts MATCH ?) m ON m.cid = c.id",
        );
        params.push(Box::new(expression));
    }

    let mut conditions: Vec<String> = Vec::new();

    match &query.scope {
        Scope::All => {}
        Scope::Favorites => conditions.push("c.favorite = 1".into()),
        Scope::Recent => conditions.push("c.last_copied_at IS NOT NULL".into()),
        Scope::Scripts => conditions.push("c.kind IN ('script', 'sequence')".into()),
        Scope::Collection { id } => {
            conditions.push(
                "EXISTS (SELECT 1 FROM command_collections cc \
                 WHERE cc.command_id = c.id AND cc.collection_id = ?)"
                    .into(),
            );
            params.push(Box::new(*id));
        }
        Scope::Tag { name } => {
            conditions.push(tag_exists_clause());
            params.push(Box::new(name.trim().to_lowercase()));
        }
    }

    // Multiple tag filters narrow the results rather than widening them.
    for tag in &query.tags {
        conditions.push(tag_exists_clause());
        params.push(Box::new(tag.trim().to_lowercase()));
    }

    if !query.kinds.is_empty() {
        let placeholders = vec!["?"; query.kinds.len()].join(", ");
        conditions.push(format!("c.kind IN ({placeholders})"));
        for kind in &query.kinds {
            params.push(Box::new(kind.as_str().to_string()));
        }
    }

    if let Some(risk) = query.risk {
        conditions.push("c.risk_level = ?".into());
        params.push(Box::new(risk.as_str().to_string()));
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY ");
    sql.push_str(&order_clause(query));

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, 2000);
    sql.push_str(" LIMIT ?");
    params.push(Box::new(limit));

    BuiltQuery { sql, params }
}

fn tag_exists_clause() -> String {
    "EXISTS (SELECT 1 FROM command_tags ct JOIN tags t ON t.id = ct.tag_id \
     WHERE ct.command_id = c.id AND t.name = ?)"
        .into()
}

fn order_clause(query: &ListQuery) -> String {
    let searching = query
        .search
        .as_deref()
        .and_then(search::build_match_query)
        .is_some();

    // Relevance beats any stored ordering while the user is typing.
    if searching {
        return "m.rank ASC, c.updated_at DESC".into();
    }

    if matches!(query.scope, Scope::Recent) {
        return "c.last_copied_at DESC".into();
    }

    match query.sort {
        Sort::Updated => "c.updated_at DESC".into(),
        Sort::Created => "c.created_at DESC".into(),
        Sort::Title => "c.title COLLATE NOCASE ASC".into(),
        Sort::Copies => "c.copy_count DESC, c.updated_at DESC".into(),
        Sort::LastCopied => "c.last_copied_at IS NULL ASC, c.last_copied_at DESC".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_query_lists_everything_newest_first() {
        let built = build_list(&ListQuery::default());
        assert!(!built.sql.contains("WHERE"));
        assert!(built.sql.contains("ORDER BY c.updated_at DESC"));
        assert_eq!(built.params.len(), 1); // just the limit
    }

    #[test]
    fn search_joins_the_index_and_orders_by_rank() {
        let query = ListQuery {
            search: Some("docker prune".into()),
            ..Default::default()
        };
        let built = build_list(&query);
        assert!(built.sql.contains("commands_fts MATCH ?"));
        assert!(built.sql.contains("ORDER BY m.rank ASC"));
    }

    #[test]
    fn punctuation_only_search_is_ignored() {
        let query = ListQuery {
            search: Some("***".into()),
            ..Default::default()
        };
        let built = build_list(&query);
        assert!(!built.sql.contains("commands_fts"));
    }

    #[test]
    fn scopes_add_their_own_condition() {
        let favorites = build_list(&ListQuery {
            scope: Scope::Favorites,
            ..Default::default()
        });
        assert!(favorites.sql.contains("c.favorite = 1"));

        let scripts = build_list(&ListQuery {
            scope: Scope::Scripts,
            ..Default::default()
        });
        assert!(scripts.sql.contains("c.kind IN ('script', 'sequence')"));

        let recent = build_list(&ListQuery {
            scope: Scope::Recent,
            ..Default::default()
        });
        assert!(recent.sql.contains("last_copied_at IS NOT NULL"));
        assert!(recent.sql.contains("ORDER BY c.last_copied_at DESC"));
    }

    #[test]
    fn every_tag_filter_narrows_the_result() {
        let query = ListQuery {
            tags: vec!["docker".into(), "cleanup".into()],
            ..Default::default()
        };
        let built = build_list(&query);
        assert_eq!(built.sql.matches("command_tags").count(), 2);
    }

    #[test]
    fn limit_is_clamped_to_something_sane() {
        let built = build_list(&ListQuery {
            limit: Some(999_999),
            ..Default::default()
        });
        assert!(built.sql.ends_with("LIMIT ?"));
        // The clamped value is the final bound parameter.
        assert_eq!(built.params.len(), 1);
    }
}
