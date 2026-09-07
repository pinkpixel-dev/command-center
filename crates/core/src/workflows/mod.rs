//! The AI workflows, expressed once for both applications.
//!
//! These used to live in the desktop crate's IPC layer, where each one was a
//! Tauri command wrapping about forty lines of real work: read settings,
//! check the AI switch, resolve a provider, run the request, parse the reply.
//! None of that is desktop-specific, and the server needs every bit of it, so
//! it lives here and each shell contributes only its own plumbing.

pub mod assistant;
pub mod codex;
pub mod convert;
pub mod diagnose;
pub mod explain;
pub mod import;
pub mod status;

use crate::db::settings::AppSettings;
use crate::error::{AppError, AppResult};

/// The switch that gates every network-backed action.
pub fn require_ai_enabled(settings: &AppSettings) -> AppResult<()> {
    if settings.ai_enabled {
        Ok(())
    } else {
        Err(AppError::AiDisabled)
    }
}

/// The model a disclosure screen must name.
///
/// Naming the wrong provider's model on the one screen that explains where a
/// document is going would be worse than refusing to show it.
pub fn selected_model(settings: &AppSettings) -> AppResult<&str> {
    settings
        .selected_model()
        .ok_or_else(|| AppError::ai_model("Choose a Codex model in Settings first."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(ai_enabled: bool) -> AppSettings {
        AppSettings {
            ai_enabled,
            ..AppSettings::default()
        }
    }

    #[test]
    fn every_workflow_is_refused_while_ai_is_off() {
        assert_eq!(
            require_ai_enabled(&settings(false)).unwrap_err().kind(),
            "ai_disabled"
        );
        assert!(require_ai_enabled(&settings(true)).is_ok());
    }
}
