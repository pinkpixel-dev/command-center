//! What every handler is given.

use std::sync::Arc;

use command_center_core::ai::providers::codex::service::CodexService;
use command_center_core::ai::{AiService, CredentialStore};
use command_center_core::db::Database;
use command_center_core::error::AppResult;

use crate::auth::Auth;
use crate::config::Config;
use crate::events::BroadcastEvents;

/// Cheap to clone: every field is shared, because there is one library, one
/// Codex process, and one event channel for the whole server.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub ai: Arc<AiService>,
    pub codex: Arc<CodexService>,
    pub events: Arc<BroadcastEvents>,
    pub auth: Arc<Auth>,
    pub config: Config,
}

impl AppState {
    pub fn new(
        config: Config,
        credentials: Arc<dyn CredentialStore>,
        auth: Arc<Auth>,
        app_version: &str,
    ) -> AppResult<Self> {
        let db = Database::open(config.library_path())?;
        // Codex keeps its login under the same mount, so a container restart
        // does not sign the account out again.
        let codex = CodexService::new(&config.data_dir, app_version);

        Ok(Self {
            db: Arc::new(db),
            ai: Arc::new(AiService::new(credentials)?),
            codex: Arc::new(codex),
            events: Arc::new(BroadcastEvents::default()),
            auth,
            config,
        })
    }
}
