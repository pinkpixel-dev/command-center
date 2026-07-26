//! Import tests against realistic documents and a real database.

use command_center_lib::db::{commands, Database};
use command_center_lib::import::apply::{DuplicateAction, ImportItem};
use command_center_lib::import::{self, Candidate};
use command_center_lib::models::{CommandInput, CommandKind, RiskLevel};

fn library() -> Database {
    Database::open_in_memory().expect("in-memory library should open")
}

fn preview(db: &Database, source: &str) -> import::ImportPreview {
    db.with(|conn| import::preview(conn, source, Some("cheatsheet.md")))
        .unwrap()
}

fn find<'a>(preview: &'a import::ImportPreview, needle: &str) -> &'a Candidate {
    preview
        .candidates
        .iter()
        .find(|candidate| candidate.content.contains(needle))
        .unwrap_or_else(|| {
            panic!(
                "no candidate containing {needle:?}; got {:?}",
                preview
                    .candidates
                    .iter()
                    .map(|candidate| candidate.content.as_str())
                    .collect::<Vec<_>>()
            )
        })
}

fn input(title: &str, content: &str) -> CommandInput {
    serde_json::from_value(serde_json::json!({ "title": title, "content": content })).unwrap()
}

const CHEAT_SHEET: &str = r#"# Docker Cheat Sheet

Notes I keep losing.

## Remove stopped containers

Frees the disk space that dead containers are holding.

```bash
docker container prune
```

## Inspect

List the containers that are currently running:

```bash
docker ps
```

Which prints something like:

```
CONTAINER ID   IMAGE     COMMAND       STATUS
9f2b1c4d8e7a   nginx     "nginx -g"    Up 2 hours
```

## Build and push

```sh
docker build -t app:latest .
docker push app:latest
```
"#;

#[test]
fn pulls_commands_out_of_a_markdown_cheat_sheet() {
    let db = library();
    let result = preview(&db, CHEAT_SHEET);

    assert!(result.candidates.len() >= 4);
    assert_eq!(result.suggested_collection.as_deref(), Some("Docker Cheat Sheet"));

    let prune = find(&result, "docker container prune");
    assert_eq!(prune.title, "Remove stopped containers");
    assert_eq!(prune.description, "Frees the disk space that dead containers are holding");
    assert_eq!(prune.kind, CommandKind::Command);
    assert!(prune.selected);
}

#[test]
fn terminal_output_is_detected_and_left_unticked() {
    let db = library();
    let result = preview(&db, CHEAT_SHEET);

    let output = find(&result, "CONTAINER ID");
    assert!(output.looks_like_output);
    assert!(!output.selected, "output should not be ticked by default");
    assert!(output.output_reason.is_some());

    let real = find(&result, "docker ps");
    assert!(!real.looks_like_output);
    assert!(real.selected);
}

#[test]
fn a_multi_command_block_becomes_a_sequence() {
    let db = library();
    let result = preview(&db, CHEAT_SHEET);

    let build = find(&result, "docker build");
    assert_eq!(build.kind, CommandKind::Sequence);
    assert!(build.content.contains("docker push"));
}

#[test]
fn headings_become_tags() {
    let db = library();
    let result = preview(&db, CHEAT_SHEET);

    let prune = find(&result, "docker container prune");
    assert!(prune.tags.contains(&"containers".to_string()) || prune.tags.contains(&"remove".to_string()));
    assert!(prune.heading_path.iter().any(|heading| heading == "Docker Cheat Sheet"));
}

#[test]
fn a_shell_session_keeps_the_commands_and_drops_the_output() {
    let db = library();
    let source = r#"## Undo a commit

```console
$ git reset --soft HEAD~1
$ git status
On branch main
Changes to be committed:
  modified:   README.md
```
"#;

    let result = preview(&db, source);
    let session = find(&result, "git reset");

    assert!(session.content.contains("git status"));
    assert!(!session.content.contains("On branch main"));
    assert!(!session.content.contains("modified:"));
    assert!(!session.looks_like_output);
    assert!(session.dropped_output_lines >= 2);
}

#[test]
fn prompt_lines_outside_a_fence_are_still_commands() {
    let db = library();
    let source = "Some notes about ports.\n\n$ lsof -i :3000\n\nThat shows what is listening.\n";

    let result = preview(&db, source);
    let candidate = find(&result, "lsof");
    assert_eq!(candidate.content, "lsof -i :3000");
}

#[test]
fn indented_code_blocks_are_found_and_dedented() {
    let db = library();
    let source = "Old style docs:\n\n    npm install\n    npm run dev\n\nBack to prose.\n";

    let result = preview(&db, source);
    let candidate = find(&result, "npm install");
    assert_eq!(candidate.content, "npm install\nnpm run dev");
}

#[test]
fn standalone_inline_code_is_captured_but_prose_is_not() {
    let db = library();
    let source = "# Notes\n\n`systemctl restart nginx`\n\nJust some prose about `x` here.\n";

    let result = preview(&db, source);
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.candidates[0].content, "systemctl restart nginx");
}

#[test]
fn markdown_lists_do_not_become_code_blocks() {
    let db = library();
    let source = "# Steps\n\n- first thing\n- second thing\n    - a nested bullet\n\nDone.\n";

    let result = preview(&db, source);
    assert!(
        result.candidates.is_empty(),
        "got {:?}",
        result.candidates.iter().map(|c| c.content.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn config_snippets_keep_their_language() {
    let db = library();
    let source = "# Rust\n\nSmaller binaries:\n\n```toml\n[profile.release]\nlto = true\nstrip = true\n```\n";

    let result = preview(&db, source);
    let snippet = find(&result, "profile.release");
    assert_eq!(snippet.kind, CommandKind::Snippet);
    assert_eq!(snippet.language.as_deref(), Some("toml"));
}

#[test]
fn risk_is_assessed_during_the_preview() {
    let db = library();
    let source = "## Clean up\n\n```bash\nrm -rf ./dist\n```\n";

    let result = preview(&db, source);
    let dangerous = find(&result, "rm -rf");
    assert_eq!(dangerous.risk_level, RiskLevel::Destructive);
    assert!(!dangerous.risk_reasons.is_empty());
}

#[test]
fn placeholders_are_reported() {
    let db = library();
    let source = "```bash\nssh {{user}}@{{host}}\n```\n";

    let result = preview(&db, source);
    assert_eq!(result.candidates[0].variables, vec!["user", "host"]);
}

#[test]
fn duplicates_of_saved_commands_are_flagged_and_unticked() {
    let db = library();
    db.with_mut(|conn| commands::create(conn, input("Already here", "docker ps")))
        .unwrap();

    let result = preview(&db, "```bash\ndocker ps\n```\n");
    let candidate = &result.candidates[0];

    let duplicate = candidate.duplicate.as_ref().expect("should match the saved entry");
    assert_eq!(duplicate.title, "Already here");
    assert!(!candidate.selected);
    assert_eq!(result.stats.duplicates, 1);
}

#[test]
fn duplicate_detection_sees_through_prompt_prefixes() {
    let db = library();
    db.with_mut(|conn| commands::create(conn, input("Saved", "git status")))
        .unwrap();

    let result = preview(&db, "$ git status\n");
    assert!(result.candidates[0].duplicate.is_some());
}

#[test]
fn a_command_repeated_in_one_document_is_only_ticked_once() {
    let db = library();
    let source = "```bash\ndocker ps\n```\n\n```bash\ndocker ps\n```\n";

    let result = preview(&db, source);
    assert_eq!(result.candidates.len(), 2);
    assert!(result.candidates[0].selected);
    assert!(!result.candidates[1].selected);
    assert!(result.candidates[1].repeated_in_document);
}

#[test]
fn analyze_recalculates_after_a_split() {
    let db = library();
    db.with_mut(|conn| commands::create(conn, input("Saved", "npm test")))
        .unwrap();

    let analysis = db.with(|conn| import::analyze(conn, "$ npm test")).unwrap();
    assert_eq!(analysis.kind, CommandKind::Command);
    assert!(analysis.duplicate.is_some());
    assert_eq!(analysis.suggested_title, "npm test");

    let dangerous = db.with(|conn| import::analyze(conn, "rm -rf /tmp/x")).unwrap();
    assert_eq!(dangerous.risk_level, RiskLevel::Destructive);
}

#[test]
fn importing_creates_the_selected_entries() {
    let db = library();
    let items = vec![
        ImportItem {
            input: input("First", "docker ps"),
            duplicate_action: DuplicateAction::Create,
            existing_id: None,
        },
        ImportItem {
            input: input("Second", "docker images"),
            duplicate_action: DuplicateAction::Create,
            existing_id: None,
        },
    ];

    let summary = db.with_mut(|conn| import::apply::run(conn, items)).unwrap();
    assert_eq!(summary.created, 2);
    assert!(summary.failures.is_empty());
    assert_eq!(db.with(commands::stats).unwrap().total, 2);
}

#[test]
fn skip_leaves_the_library_alone() {
    let db = library();
    let existing = db
        .with_mut(|conn| commands::create(conn, input("Keep me", "docker ps")))
        .unwrap();

    let summary = db
        .with_mut(|conn| {
            import::apply::run(
                conn,
                vec![ImportItem {
                    input: input("Imported", "docker ps"),
                    duplicate_action: DuplicateAction::Skip,
                    existing_id: Some(existing.id),
                }],
            )
        })
        .unwrap();

    assert_eq!(summary.skipped, 1);
    assert_eq!(db.with(commands::stats).unwrap().total, 1);
    assert_eq!(db.with(|conn| commands::get(conn, existing.id)).unwrap().title, "Keep me");
}

#[test]
fn replace_overwrites_the_existing_entry() {
    let db = library();
    let existing = db
        .with_mut(|conn| commands::create(conn, input("Old title", "docker ps")))
        .unwrap();

    let mut replacement = input("New title", "docker ps -a");
    replacement.description = "Now with all containers".into();

    let summary = db
        .with_mut(|conn| {
            import::apply::run(
                conn,
                vec![ImportItem {
                    input: replacement,
                    duplicate_action: DuplicateAction::Replace,
                    existing_id: Some(existing.id),
                }],
            )
        })
        .unwrap();

    assert_eq!(summary.replaced, 1);
    let stored = db.with(|conn| commands::get(conn, existing.id)).unwrap();
    assert_eq!(stored.title, "New title");
    assert_eq!(stored.content, "docker ps -a");
    assert_eq!(db.with(commands::stats).unwrap().total, 1);
}

#[test]
fn merge_keeps_the_saved_command_and_folds_in_the_extras() {
    let db = library();
    let mut original = input("Show containers", "docker ps");
    original.tags = vec!["docker".into()];
    let existing = db.with_mut(|conn| commands::create(conn, original)).unwrap();

    let mut incoming = input("Docker ps", "docker ps");
    incoming.tags = vec!["inspect".into()];
    incoming.description = "Lists running containers".into();
    incoming.notes = "From the cheat sheet".into();

    let summary = db
        .with_mut(|conn| {
            import::apply::run(
                conn,
                vec![ImportItem {
                    input: incoming,
                    duplicate_action: DuplicateAction::Merge,
                    existing_id: Some(existing.id),
                }],
            )
        })
        .unwrap();

    assert_eq!(summary.merged, 1);
    let stored = db.with(|conn| commands::get(conn, existing.id)).unwrap();

    assert_eq!(stored.title, "Show containers", "the saved title wins");
    assert_eq!(stored.tags, vec!["docker", "inspect"]);
    assert_eq!(stored.description, "Lists running containers");
    assert_eq!(stored.notes, "From the cheat sheet");
    assert_eq!(db.with(commands::stats).unwrap().total, 1);
}

#[test]
fn one_bad_entry_does_not_sink_the_batch() {
    let db = library();
    let items = vec![
        ImportItem {
            input: input("Good", "docker ps"),
            duplicate_action: DuplicateAction::Create,
            existing_id: None,
        },
        ImportItem {
            input: input("Empty", "   "),
            duplicate_action: DuplicateAction::Create,
            existing_id: None,
        },
        ImportItem {
            input: input("Also good", "docker images"),
            duplicate_action: DuplicateAction::Create,
            existing_id: None,
        },
    ];

    let summary = db.with_mut(|conn| import::apply::run(conn, items)).unwrap();

    assert_eq!(summary.created, 2);
    assert_eq!(summary.failures.len(), 1);
    assert_eq!(summary.failures[0].title, "Empty");
    assert_eq!(db.with(commands::stats).unwrap().total, 2);
}

#[test]
fn imported_entries_are_searchable_immediately() {
    let db = library();
    db.with_mut(|conn| {
        import::apply::run(
            conn,
            vec![ImportItem {
                input: {
                    let mut item = input("Prune images", "docker image prune -a");
                    item.tags = vec!["cleanup".into()];
                    item
                },
                duplicate_action: DuplicateAction::Create,
                existing_id: None,
            }],
        )
    })
    .unwrap();

    let found = db
        .with(|conn| {
            commands::list(
                conn,
                &serde_json::from_value(serde_json::json!({ "search": "cleanup" })).unwrap(),
            )
        })
        .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "Prune images");
}

/// A document shaped like something someone actually keeps: mixed fence styles,
/// an indented block, a console session, an inline command, and a table of
/// output pasted in for reference.
const ARCH_RESCUE: &str = r#"# Arch Rescue

Things I need when the system will not boot.

## Rebuild the initramfs

Run this after a kernel update goes sideways.

```bash
mkinitcpio -P
```

## Reinstall the kernel

    pacman -S linux linux-firmware

## Check what failed at boot

```console
$ systemctl --failed
  UNIT                   LOAD   ACTIVE SUB    DESCRIPTION
  nvidia-suspend.service loaded failed failed NVIDIA system suspend
1 loaded units listed.
```

## Fix the bootloader

`grub-mkconfig -o /boot/grub/grub.cfg`

## Free up space when pacman complains

Careful, this one is not reversible:

```bash
rm -rf /var/cache/pacman/pkg/*
```

## Disk layout for reference

```
NAME        MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS
nvme0n1     259:0    0   3.6T  0 disk
├─nvme0n1p1 259:1    0   512M  0 part /boot
└─nvme0n1p2 259:2    0   3.6T  0 part /
```

## Connect to wifi

```bash
iwctl station wlan0 connect {{network}}
```
"#;

#[test]
fn handles_a_document_of_the_shape_people_actually_write() {
    let db = library();
    let result = preview(&db, ARCH_RESCUE);

    assert_eq!(result.suggested_collection.as_deref(), Some("Arch Rescue"));
    assert_eq!(result.stats.blocks_found, 7);

    // Fenced, indented, session, and inline blocks all arrived.
    for needle in [
        "mkinitcpio -P",
        "pacman -S linux",
        "systemctl --failed",
        "grub-mkconfig",
        "rm -rf /var/cache",
        "iwctl station",
    ] {
        find(&result, needle);
    }

    // The session kept its command and dropped the three lines of output.
    let session = find(&result, "systemctl --failed");
    assert!(!session.content.contains("nvidia-suspend"));
    assert_eq!(session.dropped_output_lines, 3);

    // The pasted lsblk table is the only thing recognised as output.
    let output: Vec<&str> = result
        .candidates
        .iter()
        .filter(|candidate| candidate.looks_like_output)
        .map(|candidate| candidate.content.as_str())
        .collect();
    assert_eq!(output.len(), 1);
    assert!(output[0].contains("MOUNTPOINTS"));

    // Titles come from the headings that cover a single block.
    let initramfs = find(&result, "mkinitcpio");
    assert_eq!(initramfs.title, "Rebuild the initramfs");
    assert_eq!(initramfs.description, "Run this after a kernel update goes sideways");

    // Tags name the tool, not the words in a sentence heading.
    assert!(initramfs.tags.contains(&"mkinitcpio".to_string()));
    let failed = find(&result, "systemctl --failed");
    assert!(failed.tags.contains(&"systemctl".to_string()));
    assert!(!failed.tags.contains(&"failed".to_string()));

    // Risk still comes from the local rules.
    assert_eq!(
        find(&result, "rm -rf /var/cache").risk_level,
        RiskLevel::Destructive
    );
    assert_eq!(find(&result, "pacman -S linux").risk_level, RiskLevel::Caution);

    // Placeholders survive the trip.
    assert_eq!(find(&result, "iwctl").variables, vec!["network"]);

    // Everything except the output block is ready to import.
    let ticked = result.candidates.iter().filter(|candidate| candidate.selected).count();
    assert_eq!(ticked, 6);
}

#[test]
fn an_empty_document_produces_nothing() {
    let db = library();
    let result = preview(&db, "# Just a heading\n\nAnd a paragraph with no code.\n");
    assert!(result.candidates.is_empty());
    assert_eq!(result.stats.blocks_found, 0);
}
