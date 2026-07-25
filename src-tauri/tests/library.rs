//! End-to-end exercises of the library against a real SQLite database. No
//! fakes: every test opens an actual database, migrates it, and writes rows.

use command_center_lib::db::query::{ListQuery, Scope, Sort};
use command_center_lib::db::{collections, commands, tags, Database};
use command_center_lib::models::{CollectionInput, CommandInput, CommandKind, RiskLevel};

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

fn add(db: &Database, input: CommandInput) -> command_center_lib::models::Command {
    db.with_mut(|conn| commands::create(conn, input)).unwrap()
}

fn list(db: &Database, filter: ListQuery) -> Vec<command_center_lib::models::Command> {
    db.with(|conn| commands::list(conn, &filter)).unwrap()
}

fn titles(entries: &[command_center_lib::models::Command]) -> Vec<&str> {
    entries.iter().map(|entry| entry.title.as_str()).collect()
}

#[test]
fn creates_and_reads_back_an_entry() {
    let db = library();
    let mut input = entry("List open ports", "lsof -i :3000");
    input.description = "Shows what is squatting on a port".into();
    input.tags = vec!["Network".into(), "debug".into()];
    input.shell = Some("bash".into());

    let created = add(&db, input);

    assert_eq!(created.title, "List open ports");
    assert_eq!(created.tags, vec!["debug", "network"]);
    assert_eq!(created.shell.as_deref(), Some("bash"));
    assert_eq!(created.copy_count, 0);
    assert!(created.last_copied_at.is_none());

    let fetched = db.with(|conn| commands::get(conn, created.id)).unwrap();
    assert_eq!(fetched.content, "lsof -i :3000");
}

#[test]
fn risk_is_detected_on_save_but_can_be_overridden() {
    let db = library();

    let detected = add(&db, entry("Nuke build", "rm -rf ./dist"));
    assert_eq!(detected.risk_level, RiskLevel::Destructive);
    assert!(!detected.risk_reasons.is_empty());

    let mut manual = entry("Careful delete", "rm -rf ./dist");
    manual.risk_level = Some(RiskLevel::Caution);
    let stored = add(&db, manual);
    assert_eq!(stored.risk_level, RiskLevel::Caution);
}

#[test]
fn updates_replace_tags_and_collections() {
    let db = library();
    let created = add(&db, {
        let mut input = entry("Prune", "docker container prune");
        input.tags = vec!["docker".into(), "cleanup".into()];
        input
    });

    let collection_id = db
        .with(|conn| {
            collections::create(
                conn,
                CollectionInput {
                    name: "Docker Cleanup".into(),
                    description: String::new(),
                },
            )
        })
        .unwrap();

    let mut update = entry("Prune containers", "docker container prune -f");
    update.tags = vec!["docker".into()];
    update.collection_ids = vec![collection_id];

    let updated = db
        .with_mut(|conn| commands::update(conn, created.id, update))
        .unwrap();

    assert_eq!(updated.title, "Prune containers");
    assert_eq!(updated.tags, vec!["docker"]);
    assert_eq!(updated.collections.len(), 1);
    assert_eq!(updated.collections[0].name, "Docker Cleanup");

    // "cleanup" is no longer used by anything and should be gone.
    let remaining = db.with(tags::list).unwrap();
    assert!(remaining.iter().all(|tag| tag.name != "cleanup"));
}

#[test]
fn full_text_search_covers_title_content_description_and_tags() {
    let db = library();
    add(&db, {
        let mut input = entry("Update Arch packages", "sudo pacman -Syu");
        input.description = "Refresh every installed package".into();
        input.tags = vec!["arch".into(), "update".into()];
        input
    });
    add(&db, entry("Show running containers", "docker ps"));

    for term in ["pacman", "arch", "refresh", "Update Arch"] {
        let results = list(
            &db,
            ListQuery {
                search: Some(term.into()),
                ..Default::default()
            },
        );
        assert_eq!(titles(&results), vec!["Update Arch packages"], "term: {term}");
    }

    let none = list(
        &db,
        ListQuery {
            search: Some("kubernetes".into()),
            ..Default::default()
        },
    );
    assert!(none.is_empty());
}

#[test]
fn search_matches_prefixes_while_typing() {
    let db = library();
    add(&db, entry("Docker cleanup", "docker system prune -a"));

    for term in ["doc", "docke", "docker sys"] {
        let results = list(
            &db,
            ListQuery {
                search: Some(term.into()),
                ..Default::default()
            },
        );
        assert_eq!(results.len(), 1, "term: {term}");
    }
}

#[test]
fn search_survives_punctuation_heavy_input() {
    let db = library();
    add(&db, entry("Force delete", "rm -rf ./node_modules"));

    let results = list(
        &db,
        ListQuery {
            search: Some("rm -rf".into()),
            ..Default::default()
        },
    );
    assert_eq!(results.len(), 1);

    // Pure punctuation is treated as "no search", not as a syntax error.
    let everything = list(
        &db,
        ListQuery {
            search: Some("***".into()),
            ..Default::default()
        },
    );
    assert_eq!(everything.len(), 1);
}

#[test]
fn search_index_follows_edits_and_deletes() {
    let db = library();
    let created = add(&db, entry("Old title", "echo before"));

    db.with_mut(|conn| commands::update(conn, created.id, entry("New title", "echo after")))
        .unwrap();

    let stale = list(
        &db,
        ListQuery {
            search: Some("before".into()),
            ..Default::default()
        },
    );
    assert!(stale.is_empty(), "the old content should not be searchable");

    let fresh = list(
        &db,
        ListQuery {
            search: Some("after".into()),
            ..Default::default()
        },
    );
    assert_eq!(fresh.len(), 1);

    db.with_mut(|conn| commands::delete(conn, created.id)).unwrap();
    let gone = list(
        &db,
        ListQuery {
            search: Some("after".into()),
            ..Default::default()
        },
    );
    assert!(gone.is_empty());
}

#[test]
fn renaming_a_collection_keeps_its_entries_searchable() {
    let db = library();
    let collection_id = db
        .with(|conn| {
            collections::create(
                conn,
                CollectionInput {
                    name: "Arch Rescue".into(),
                    description: String::new(),
                },
            )
        })
        .unwrap();

    add(&db, {
        let mut input = entry("Rebuild initramfs", "mkinitcpio -P");
        input.collection_ids = vec![collection_id];
        input
    });

    db.with(|conn| {
        collections::update(
            conn,
            collection_id,
            CollectionInput {
                name: "Arch Recovery".into(),
                description: String::new(),
            },
        )
    })
    .unwrap();

    let by_new_name = list(
        &db,
        ListQuery {
            search: Some("recovery".into()),
            ..Default::default()
        },
    );
    assert_eq!(by_new_name.len(), 1);

    let by_old_name = list(
        &db,
        ListQuery {
            search: Some("rescue".into()),
            ..Default::default()
        },
    );
    assert!(by_old_name.is_empty());
}

#[test]
fn favorites_scope_only_returns_pinned_entries() {
    let db = library();
    let pinned = add(&db, entry("Pinned", "git status"));
    add(&db, entry("Unpinned", "git log"));

    let now_favorite = db
        .with(|conn| commands::toggle_favorite(conn, pinned.id))
        .unwrap();
    assert!(now_favorite);

    let favorites = list(
        &db,
        ListQuery {
            scope: Scope::Favorites,
            ..Default::default()
        },
    );
    assert_eq!(titles(&favorites), vec!["Pinned"]);

    let unpinned_again = db
        .with(|conn| commands::toggle_favorite(conn, pinned.id))
        .unwrap();
    assert!(!unpinned_again);
    assert!(list(
        &db,
        ListQuery {
            scope: Scope::Favorites,
            ..Default::default()
        }
    )
    .is_empty());
}

#[test]
fn copying_updates_counts_and_fills_the_recent_scope() {
    let db = library();
    let created = add(&db, entry("Copy me", "pwd"));
    add(&db, entry("Never copied", "whoami"));

    let once = db.with(|conn| commands::record_copy(conn, created.id)).unwrap();
    assert_eq!(once.copy_count, 1);
    assert!(once.last_copied_at.is_some());

    let twice = db.with(|conn| commands::record_copy(conn, created.id)).unwrap();
    assert_eq!(twice.copy_count, 2);

    let recent = list(
        &db,
        ListQuery {
            scope: Scope::Recent,
            ..Default::default()
        },
    );
    assert_eq!(titles(&recent), vec!["Copy me"]);

    let history: i64 = db
        .with(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM usage_history WHERE command_id = ?1",
                [created.id],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(history, 2);
}

#[test]
fn scripts_scope_covers_scripts_and_sequences() {
    let db = library();
    add(&db, {
        let mut input = entry("Backup script", "#!/usr/bin/env bash\necho hi");
        input.kind = CommandKind::Script;
        input
    });
    add(&db, {
        let mut input = entry("Release steps", "npm test\nnpm run build");
        input.kind = CommandKind::Sequence;
        input
    });
    add(&db, entry("Plain", "ls"));

    let scripts = list(
        &db,
        ListQuery {
            scope: Scope::Scripts,
            ..Default::default()
        },
    );
    assert_eq!(scripts.len(), 2);
}

#[test]
fn tag_and_collection_scopes_filter_correctly() {
    let db = library();
    let collection_id = db
        .with(|conn| {
            collections::create(
                conn,
                CollectionInput {
                    name: "Git Mistakes".into(),
                    description: "Undo buttons".into(),
                },
            )
        })
        .unwrap();

    add(&db, {
        let mut input = entry("Undo last commit", "git reset --soft HEAD~1");
        input.tags = vec!["git".into(), "undo".into()];
        input.collection_ids = vec![collection_id];
        input
    });
    add(&db, {
        let mut input = entry("Disk usage", "du -sh *");
        input.tags = vec!["filesystem".into()];
        input
    });

    let by_tag = list(
        &db,
        ListQuery {
            scope: Scope::Tag { name: "GIT".into() },
            ..Default::default()
        },
    );
    assert_eq!(titles(&by_tag), vec!["Undo last commit"]);

    let by_collection = list(
        &db,
        ListQuery {
            scope: Scope::Collection { id: collection_id },
            ..Default::default()
        },
    );
    assert_eq!(titles(&by_collection), vec!["Undo last commit"]);

    let two_tags = list(
        &db,
        ListQuery {
            tags: vec!["git".into(), "filesystem".into()],
            ..Default::default()
        },
    );
    assert!(two_tags.is_empty(), "tag filters should narrow, not widen");
}

#[test]
fn sorting_options_change_the_order() {
    let db = library();
    let first = add(&db, entry("Alpha", "echo a"));
    add(&db, entry("Zulu", "echo z"));
    db.with(|conn| commands::record_copy(conn, first.id)).unwrap();

    let by_title = list(
        &db,
        ListQuery {
            sort: Sort::Title,
            ..Default::default()
        },
    );
    assert_eq!(titles(&by_title), vec!["Alpha", "Zulu"]);

    let by_copies = list(
        &db,
        ListQuery {
            sort: Sort::Copies,
            ..Default::default()
        },
    );
    assert_eq!(titles(&by_copies)[0], "Alpha");
}

#[test]
fn duplicate_detection_ignores_prompt_prefixes_and_whitespace() {
    let db = library();
    add(&db, entry("Reset", "git reset --soft HEAD~1"));

    let found = db
        .with(|conn| commands::find_by_content(conn, "$ git reset --soft HEAD~1  \n\n"))
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().title, "Reset");

    let different = db
        .with(|conn| commands::find_by_content(conn, "git reset --hard HEAD~1"))
        .unwrap();
    assert!(different.is_none());
}

#[test]
fn deleting_an_entry_cleans_up_everything_attached_to_it() {
    let db = library();
    let collection_id = db
        .with(|conn| {
            collections::create(
                conn,
                CollectionInput {
                    name: "Temp".into(),
                    description: String::new(),
                },
            )
        })
        .unwrap();

    let created = add(&db, {
        let mut input = entry("Throwaway", "echo bye");
        input.tags = vec!["temporary".into()];
        input.collection_ids = vec![collection_id];
        input
    });

    db.with(|conn| commands::record_copy(conn, created.id)).unwrap();
    db.with_mut(|conn| commands::delete(conn, created.id)).unwrap();

    let leftovers: i64 = db
        .with(|conn| {
            Ok(conn.query_row(
                "SELECT
                    (SELECT COUNT(*) FROM command_tags WHERE command_id = ?1) +
                    (SELECT COUNT(*) FROM command_collections WHERE command_id = ?1) +
                    (SELECT COUNT(*) FROM usage_history WHERE command_id = ?1) +
                    (SELECT COUNT(*) FROM tags WHERE name = 'temporary')",
                [created.id],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(leftovers, 0);

    // The collection itself survives its last entry leaving.
    assert_eq!(db.with(collections::list).unwrap().len(), 1);
}

#[test]
fn deleting_a_collection_keeps_its_entries() {
    let db = library();
    let collection_id = db
        .with(|conn| {
            collections::create(
                conn,
                CollectionInput {
                    name: "Doomed".into(),
                    description: String::new(),
                },
            )
        })
        .unwrap();

    let created = add(&db, {
        let mut input = entry("Survivor", "uptime");
        input.collection_ids = vec![collection_id];
        input
    });

    db.with(|conn| collections::delete(conn, collection_id)).unwrap();

    let still_there = db.with(|conn| commands::get(conn, created.id)).unwrap();
    assert!(still_there.collections.is_empty());
}

#[test]
fn duplicate_collection_names_are_refused() {
    let db = library();
    db.with(|conn| {
        collections::create(
            conn,
            CollectionInput {
                name: "Rust Tooling".into(),
                description: String::new(),
            },
        )
    })
    .unwrap();

    let again = db.with(|conn| {
        collections::create(
            conn,
            CollectionInput {
                name: "rust tooling".into(),
                description: String::new(),
            },
        )
    });
    assert!(again.is_err());
}

#[test]
fn stats_track_the_sidebar_counts() {
    let db = library();
    let favorite = add(&db, entry("Fav", "ls"));
    add(&db, {
        let mut input = entry("Script", "echo hi");
        input.kind = CommandKind::Script;
        input
    });

    db.with(|conn| commands::toggle_favorite(conn, favorite.id)).unwrap();
    db.with(|conn| commands::record_copy(conn, favorite.id)).unwrap();

    let stats = db.with(commands::stats).unwrap();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.favorites, 1);
    assert_eq!(stats.scripts, 1);
    assert_eq!(stats.recent, 1);
}

#[test]
fn variables_are_exposed_for_templated_commands() {
    let db = library();
    let created = add(&db, entry("SSH in", "ssh {{user}}@{{host}}"));
    assert_eq!(created.variables, vec!["user", "host"]);
}

#[test]
fn missing_entries_report_a_clear_error() {
    let db = library();
    let missing = db.with(|conn| commands::get(conn, 4242));
    assert!(missing.is_err());
    assert!(missing.unwrap_err().to_string().contains("not found"));
}
