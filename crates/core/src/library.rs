//! The library writes, with their change notification attached.
//!
//! Reads go straight to `db`, because nothing needs to happen after them. A
//! write is different: every frontend has to hear about it, and pairing the
//! write with its notification here is what stops the desktop app and the
//! server from drifting apart on which operations announce.

use crate::db::{
    collections as collections_db, commands as commands_db, settings as settings_db, Database,
};
use crate::db::settings::AppSettings;
use crate::error::AppResult;
use crate::events::LibraryEvents;
use crate::import::apply::{self, ImportItem, ImportSummary};
use crate::models::{Collection, CollectionInput, Command, CommandInput};

pub fn create_command(
    db: &Database,
    events: &dyn LibraryEvents,
    input: CommandInput,
) -> AppResult<Command> {
    let created = db.with_mut(|conn| commands_db::create(conn, input))?;
    events.library_changed();
    Ok(created)
}

pub fn update_command(
    db: &Database,
    events: &dyn LibraryEvents,
    id: i64,
    input: CommandInput,
) -> AppResult<Command> {
    let updated = db.with_mut(|conn| commands_db::update(conn, id, input))?;
    events.library_changed();
    Ok(updated)
}

pub fn delete_command(db: &Database, events: &dyn LibraryEvents, id: i64) -> AppResult<()> {
    db.with_mut(|conn| commands_db::delete(conn, id))?;
    events.library_changed();
    Ok(())
}

pub fn delete_commands(
    db: &Database,
    events: &dyn LibraryEvents,
    command_ids: &[i64],
) -> AppResult<()> {
    db.with_mut(|conn| commands_db::delete_many(conn, command_ids))?;
    events.library_changed();
    Ok(())
}

pub fn toggle_favorite(db: &Database, events: &dyn LibraryEvents, id: i64) -> AppResult<bool> {
    let favorite = db.with(|conn| commands_db::toggle_favorite(conn, id))?;
    events.library_changed();
    Ok(favorite)
}

/// Called after the frontend puts a command on the clipboard.
pub fn record_copy(db: &Database, events: &dyn LibraryEvents, id: i64) -> AppResult<Command> {
    let updated = db.with(|conn| commands_db::record_copy(conn, id))?;
    events.library_changed();
    Ok(updated)
}

pub fn create_collection(
    db: &Database,
    events: &dyn LibraryEvents,
    input: CollectionInput,
) -> AppResult<Vec<Collection>> {
    let list = db.with(|conn| {
        collections_db::create(conn, input)?;
        collections_db::list(conn)
    })?;
    events.library_changed();
    Ok(list)
}

pub fn update_collection(
    db: &Database,
    events: &dyn LibraryEvents,
    id: i64,
    input: CollectionInput,
) -> AppResult<Vec<Collection>> {
    let list = db.with(|conn| {
        collections_db::update(conn, id, input)?;
        collections_db::list(conn)
    })?;
    events.library_changed();
    Ok(list)
}

pub fn delete_collection(
    db: &Database,
    events: &dyn LibraryEvents,
    id: i64,
) -> AppResult<Vec<Collection>> {
    let list = db.with(|conn| {
        collections_db::delete(conn, id)?;
        collections_db::list(conn)
    })?;
    events.library_changed();
    Ok(list)
}

pub fn add_commands_to_collection(
    db: &Database,
    events: &dyn LibraryEvents,
    command_ids: &[i64],
    collection_id: i64,
) -> AppResult<()> {
    db.with_mut(|conn| {
        collections_db::add_commands_to_collection(conn, command_ids, collection_id)
    })?;
    events.library_changed();
    Ok(())
}

/// An import that changed nothing does not announce, so a preview the user
/// confirmed and then emptied does not refresh the view for no reason.
pub fn import_commands(
    db: &Database,
    events: &dyn LibraryEvents,
    items: Vec<ImportItem>,
) -> AppResult<ImportSummary> {
    let summary = db.with_mut(|conn| apply::run(conn, items))?;
    if summary.touched() > 0 {
        events.library_changed();
    }
    Ok(summary)
}

/// Saves settings and announces the saved values, which are the sanitized ones
/// rather than whatever arrived.
///
/// Anything a frontend has to do for itself before the write, such as the
/// desktop app registering itself with the operating system's login startup,
/// happens at the call site.
pub fn save_settings(
    db: &Database,
    events: &dyn LibraryEvents,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    let saved = db.with(|conn| settings_db::save(conn, settings))?;
    events.settings_changed(&saved);
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::NoEvents;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct Counter {
        library: AtomicUsize,
        settings: AtomicUsize,
    }

    impl LibraryEvents for Counter {
        fn library_changed(&self) {
            self.library.fetch_add(1, Ordering::Relaxed);
        }
        fn settings_changed(&self, _settings: &AppSettings) {
            self.settings.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn entry(title: &str) -> CommandInput {
        serde_json::from_value(serde_json::json!({
            "title": title,
            "content": format!("echo {title}")
        }))
        .unwrap()
    }

    #[test]
    fn every_library_write_announces_exactly_once() {
        let db = Database::open_in_memory().unwrap();
        let events = Counter::default();

        let created = create_command(&db, &events, entry("one")).unwrap();
        update_command(&db, &events, created.id, entry("two")).unwrap();
        toggle_favorite(&db, &events, created.id).unwrap();
        record_copy(&db, &events, created.id).unwrap();
        delete_command(&db, &events, created.id).unwrap();

        assert_eq!(events.library.load(Ordering::Relaxed), 5);
        assert_eq!(events.settings.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_failed_write_announces_nothing() {
        let db = Database::open_in_memory().unwrap();
        let events = Counter::default();

        assert!(delete_command(&db, &events, 404).is_err());
        assert!(toggle_favorite(&db, &events, 404).is_err());

        assert_eq!(events.library.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn an_import_that_changed_nothing_does_not_announce() {
        let db = Database::open_in_memory().unwrap();
        let events = Counter::default();

        let summary = import_commands(&db, &events, Vec::new()).unwrap();

        assert_eq!(summary.touched(), 0);
        assert_eq!(events.library.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn saving_settings_announces_the_sanitized_values() {
        let db = Database::open_in_memory().unwrap();
        let events = Counter::default();

        let saved = save_settings(
            &db,
            &events,
            AppSettings {
                theme: "not-a-theme".into(),
                ..AppSettings::default()
            },
        )
        .unwrap();

        assert_ne!(saved.theme, "not-a-theme");
        assert_eq!(events.settings.load(Ordering::Relaxed), 1);
        assert_eq!(events.library.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_caller_with_no_frontend_can_still_write() {
        let db = Database::open_in_memory().unwrap();
        let created = create_command(&db, &NoEvents, entry("one")).unwrap();

        assert_eq!(created.title, "one");
    }
}
