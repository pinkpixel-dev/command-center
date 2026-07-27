//! Real SQLite coverage for bulk command mutations.

use command_center_lib::db::query::ListQuery;
use command_center_lib::db::{collections, commands, tags, Database};
use command_center_lib::models::{CollectionInput, Command, CommandInput};

fn library() -> Database {
    Database::open_in_memory().expect("in-memory library should open")
}

fn entry(title: &str, content: &str) -> CommandInput {
    serde_json::from_value(serde_json::json!({
        "title": title,
        "content": content,
    }))
    .expect("input should deserialize")
}

fn add(db: &Database, input: CommandInput) -> Command {
    db.with_mut(|conn| commands::create(conn, input)).unwrap()
}

fn create_collection(db: &Database, name: &str) -> i64 {
    db.with(|conn| {
        collections::create(
            conn,
            CollectionInput {
                name: name.into(),
                description: String::new(),
            },
        )
    })
    .unwrap()
}

fn search(db: &Database, term: &str) -> Vec<Command> {
    db.with(|conn| {
        commands::list(
            conn,
            &ListQuery {
                search: Some(term.into()),
                ..Default::default()
            },
        )
    })
    .unwrap()
}

#[test]
fn collection_assignment_preserves_memberships_and_reindexes_search() {
    let db = library();
    let existing_collection = create_collection(&db, "Existing Group");
    let target_collection = create_collection(&db, "Roadmap Batch");
    let first = add(&db, {
        let mut input = entry("First", "echo first");
        input.collection_ids = vec![existing_collection];
        input
    });
    let second = add(&db, entry("Second", "echo second"));

    let updated = db
        .with_mut(|conn| {
            collections::add_commands_to_collection(
                conn,
                &[first.id, second.id, first.id],
                target_collection,
            )
        })
        .unwrap();
    assert_eq!(updated, 2, "duplicate selections should only count once");

    let first = db.with(|conn| commands::get(conn, first.id)).unwrap();
    assert_eq!(
        first
            .collections
            .iter()
            .map(|collection| collection.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Existing Group", "Roadmap Batch"]
    );
    let second = db.with(|conn| commands::get(conn, second.id)).unwrap();
    assert_eq!(second.collections[0].name, "Roadmap Batch");

    let mut titles = search(&db, "roadmap")
        .into_iter()
        .map(|entry| entry.title)
        .collect::<Vec<_>>();
    titles.sort();
    assert_eq!(titles, vec!["First", "Second"]);
}

#[test]
fn collection_assignment_is_atomic_when_a_selected_command_is_missing() {
    let db = library();
    let target_collection = create_collection(&db, "Atomic Target");
    let existing = add(&db, entry("Keep unchanged", "echo steady"));

    let error = db
        .with_mut(|conn| {
            collections::add_commands_to_collection(
                conn,
                &[existing.id, 999_999],
                target_collection,
            )
        })
        .unwrap_err();
    assert_eq!(error.kind(), "not_found");

    let unchanged = db.with(|conn| commands::get(conn, existing.id)).unwrap();
    assert!(unchanged.collections.is_empty());
    assert!(search(&db, "atomic").is_empty());
}

#[test]
fn delete_cleans_stats_search_history_memberships_and_orphaned_tags() {
    let db = library();
    let collection_id = create_collection(&db, "Cleanup Batch");
    let first = add(&db, {
        let mut input = entry("Delete first", "echo vanished-one");
        input.tags = vec!["orphan-one".into()];
        input.collection_ids = vec![collection_id];
        input
    });
    let second = add(&db, {
        let mut input = entry("Delete second", "echo vanished-two");
        input.tags = vec!["orphan-two".into()];
        input.collection_ids = vec![collection_id];
        input
    });
    add(&db, {
        let mut input = entry("Survivor", "echo remains");
        input.tags = vec!["keep".into()];
        input
    });
    db.with(|conn| commands::toggle_favorite(conn, first.id))
        .unwrap();
    db.with(|conn| commands::record_copy(conn, second.id))
        .unwrap();

    let deleted = db
        .with_mut(|conn| commands::delete_many(conn, &[first.id, second.id, first.id]))
        .unwrap();
    assert_eq!(deleted, 2);

    let stats = db.with(commands::stats).unwrap();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.favorites, 0);
    assert_eq!(stats.recent, 0);
    assert_eq!(
        db.with(tags::list)
            .unwrap()
            .into_iter()
            .map(|tag| tag.name)
            .collect::<Vec<_>>(),
        vec!["keep"]
    );
    assert!(search(&db, "vanished").is_empty());
    assert_eq!(
        db.with(|conn| collections::get(conn, collection_id))
            .unwrap()
            .command_count,
        0
    );
}

#[test]
fn delete_is_atomic_when_a_selected_command_is_missing() {
    let db = library();
    let first = add(&db, {
        let mut input = entry("Still here", "echo searchable");
        input.tags = vec!["preserved".into()];
        input
    });
    let second = add(&db, entry("Also here", "echo second"));

    let error = db
        .with_mut(|conn| commands::delete_many(conn, &[first.id, 999_999]))
        .unwrap_err();
    assert_eq!(error.kind(), "not_found");
    assert!(db.with(|conn| commands::get(conn, first.id)).is_ok());
    assert!(db.with(|conn| commands::get(conn, second.id)).is_ok());
    assert_eq!(search(&db, "searchable")[0].title, "Still here");
    assert_eq!(db.with(tags::list).unwrap()[0].name, "preserved");
}
