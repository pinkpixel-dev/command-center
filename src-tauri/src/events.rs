//! The desktop side of `LibraryEvents`: Tauri's own event system.

use tauri::{AppHandle, Emitter};

use command_center_core::db::settings::AppSettings;
use command_center_core::events::{LibraryEvents, LIBRARY_CHANGED, SETTINGS_CHANGED};

/// Emits to every window. Cheap to construct, so a command builds one from the
/// handle it already receives rather than holding one in managed state.
pub struct DesktopEvents(pub AppHandle);

impl LibraryEvents for DesktopEvents {
    fn library_changed(&self) {
        // A failed notification should never fail the write that triggered it.
        let _ = self.0.emit(LIBRARY_CHANGED, ());
    }

    fn settings_changed(&self, settings: &AppSettings) {
        let _ = self.0.emit(SETTINGS_CHANGED, settings);
    }
}
