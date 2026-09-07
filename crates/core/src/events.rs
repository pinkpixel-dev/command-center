//! How the core tells a frontend that something changed.
//!
//! The desktop app pushes these to the webview with Tauri's event system; the
//! server fans them out to connected browsers over SSE. Neither mechanism
//! belongs in the core, so the core takes a listener and calls it.

use crate::db::settings::AppSettings;

/// Fired after any write that changes what the library view should show.
pub const LIBRARY_CHANGED: &str = "library-changed";

/// Fired after settings are saved, so every open window can react.
pub const SETTINGS_CHANGED: &str = "settings-changed";

/// The two notifications the frontend already listens for.
///
/// Implementations must not fail: a notification that could not be delivered
/// has to be dropped rather than turned into a failed write. That is why
/// neither method returns a result.
pub trait LibraryEvents: Send + Sync {
    fn library_changed(&self);
    fn settings_changed(&self, settings: &AppSettings);
}

/// A listener that discards everything, for tests and for callers that have no
/// frontend to notify.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoEvents;

impl LibraryEvents for NoEvents {
    fn library_changed(&self) {}
    fn settings_changed(&self, _settings: &AppSettings) {}
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn a_listener_can_be_used_behind_a_trait_object() {
        let counter = Counter::default();
        let events: &dyn LibraryEvents = &counter;

        events.library_changed();
        events.settings_changed(&AppSettings::default());

        assert_eq!(counter.library.load(Ordering::Relaxed), 1);
        assert_eq!(counter.settings.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn the_event_names_are_the_ones_the_frontend_listens_for() {
        assert_eq!(LIBRARY_CHANGED, "library-changed");
        assert_eq!(SETTINGS_CHANGED, "settings-changed");
    }
}
