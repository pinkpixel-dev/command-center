//! The compact capture window and the global shortcut that summons it.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppResult;

pub const WINDOW_LABEL: &str = "quick-add";

/// Shows the Quick Add window, creating it the first time it is needed.
pub fn show(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.show()?;
        window.unminimize().ok();
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("quick-add.html".into()))
        .title("Quick Add")
        .inner_size(660.0, 380.0)
        .min_inner_size(420.0, 300.0)
        .resizable(true)
        .always_on_top(true)
        .center()
        .build()?;

    Ok(())
}

/// Hides the window instead of destroying it, so the next summon is instant.
pub fn hide(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.hide()?;
    }
    Ok(())
}

/// Shortcut behaviour: summon when hidden, dismiss when already focused.
pub fn toggle(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        let visible = window.is_visible().unwrap_or(false);
        let focused = window.is_focused().unwrap_or(false);
        if visible && focused {
            window.hide()?;
            return Ok(());
        }
    }
    show(app)
}

#[cfg(desktop)]
pub mod shortcut {
    use tauri::AppHandle;
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

    use crate::error::{AppError, AppResult};

    /// Registers the Quick Add accelerator, replacing whatever was registered
    /// before. An accelerator another app already owns is reported back to the
    /// caller instead of silently failing.
    pub fn register(app: &AppHandle, accelerator: &str) -> AppResult<()> {
        let manager = app.global_shortcut();
        manager.unregister_all().ok();

        let shortcut: Shortcut = accelerator
            .parse()
            .map_err(|_| AppError::invalid(format!("\"{accelerator}\" is not a valid shortcut")))?;

        manager
            .on_shortcut(shortcut, |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    if let Err(error) = super::toggle(app) {
                        eprintln!("quick add shortcut failed: {error}");
                    }
                }
            })
            .map_err(|err| {
                AppError::invalid(format!(
                    "Could not register \"{accelerator}\". Another app may already use it. ({err})"
                ))
            })
    }
}
