//! Managed Codex state: the cached discovery result, the running connection,
//! and the bounded restart counter.
//!
//! One instance lives for the life of the app. It starts nothing until a
//! caller asks for status or a connection, so a user who never turns AI on
//! never spawns Codex.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::Mutex;

pub mod session;

use super::discovery::{self, CodexDiscovery, UnusableReason};
use super::launch::{launch_plan, EnvSource};
use super::process::CodexProcess;
use super::version::MINIMUM_CODEX_VERSION;
use crate::error::{AppError, AppResult};

/// An unexpected exit gets one automatic restart. A second one stops automatic
/// recovery and hands the user a Retry action, so a crash loop cannot spin.
const MAX_AUTOMATIC_RESTARTS: u32 = 1;

/// Reading account state is a local call plus, at most, one upstream check.
const ACCOUNT_TIMEOUT: Duration = Duration::from_secs(30);

/// What the Settings panel is told about Codex itself. This is the part that
/// exists before any account is connected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CodexAvailability {
    /// Codex is installed, current enough, and usable.
    Ready { version: String },
    /// No Codex installation was found.
    NotFound,
    /// Found, but below the supported floor.
    TooOld {
        version: String,
        minimum: String,
    },
    /// Found, but the file cannot be used as it stands.
    Unusable { reason: UnusableReason },
    /// This build cannot run Codex at all.
    UnsupportedPlatform,
}

pub struct CodexService {
    /// The app's own Codex directory, kept away from the user's `~/.codex`.
    codex_home: PathBuf,
    /// The empty private directory threads run in.
    working_dir: PathBuf,
    app_version: String,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    discovery: Option<CodexDiscovery>,
    process: Option<Arc<CodexProcess>>,
    restarts: u32,
    /// Codex runs one login at a time, so this app tracks one too.
    active_login: Option<ActiveLogin>,
}

struct ActiveLogin {
    login_id: String,
    /// Resolved by the task watching for `account/login/completed`.
    completion: tokio::sync::oneshot::Receiver<super::auth::LoginOutcome>,
}

impl CodexService {
    pub fn new(app_data_dir: &Path, app_version: &str) -> Self {
        Self {
            codex_home: app_data_dir.join("codex"),
            working_dir: app_data_dir.join("codex-workspace"),
            app_version: app_version.to_owned(),
            inner: Mutex::new(Inner::default()),
        }
    }

    /// Resolves Codex, reusing the cached result. Settings calls
    /// [`Self::refresh`] when the user asks for a retry or changes the path.
    pub async fn availability(&self, saved_path: Option<&str>) -> CodexAvailability {
        if !cfg!(desktop) {
            return CodexAvailability::UnsupportedPlatform;
        }

        let discovery = self.discovery(saved_path).await;
        match discovery {
            CodexDiscovery::Found { version, .. } => CodexAvailability::Ready {
                version: version.to_string(),
            },
            CodexDiscovery::TooOld { version, .. } => CodexAvailability::TooOld {
                version: version.to_string(),
                minimum: MINIMUM_CODEX_VERSION.to_string(),
            },
            CodexDiscovery::Unusable { reason, .. } => CodexAvailability::Unusable { reason },
            CodexDiscovery::NotFound => CodexAvailability::NotFound,
        }
    }

    async fn discovery(&self, saved_path: Option<&str>) -> CodexDiscovery {
        {
            let inner = self.inner.lock().await;
            if let Some(cached) = &inner.discovery {
                return cached.clone();
            }
        }

        let saved = saved_path.map(PathBuf::from);
        let result = discovery::discover(saved.as_deref()).await;

        let mut inner = self.inner.lock().await;
        inner.discovery = Some(result.clone());
        result
    }

    /// Drops the cached discovery result and stops any running process, so the
    /// next call starts from scratch. Used by Retry and by a path change.
    pub async fn refresh(&self) {
        let process = {
            let mut inner = self.inner.lock().await;
            inner.discovery = None;
            inner.restarts = 0;
            inner.process.take()
        };
        if let Some(process) = process {
            process.shutdown().await;
        }
    }

    /// Returns a live connection, starting one if needed.
    ///
    /// A process that died since the last call is replaced, up to the restart
    /// ceiling. Past that the user has to ask for a retry, which resets the
    /// counter through [`Self::refresh`].
    pub async fn connection(&self, saved_path: Option<&str>) -> AppResult<Arc<CodexProcess>> {
        let discovery = self.discovery(saved_path).await;
        let executable = match &discovery {
            CodexDiscovery::Found { path, .. } => path.clone(),
            CodexDiscovery::TooOld { version, .. } => {
                return Err(AppError::ai_network(format!(
                    "Codex {version} is older than the {MINIMUM_CODEX_VERSION} this feature needs. Update Codex and try again."
                )))
            }
            CodexDiscovery::Unusable { .. } => {
                return Err(AppError::ai_network(
                    "The Codex program could not be started. Check the path in Settings.",
                ))
            }
            CodexDiscovery::NotFound => {
                return Err(AppError::ai_network(
                    "Codex was not found on this computer. Install it, then try again.",
                ))
            }
        };

        let mut inner = self.inner.lock().await;

        if let Some(process) = &inner.process {
            if process.is_running() {
                return Ok(Arc::clone(process));
            }
            // It exited on its own. Count it before trying again.
            inner.process = None;
            inner.restarts += 1;
            if inner.restarts > MAX_AUTOMATIC_RESTARTS {
                return Err(AppError::ai_network(
                    "Codex keeps stopping unexpectedly. Use Retry in Settings once the problem is fixed.",
                ));
            }
        }

        let plan = launch_plan(&self.codex_home, &self.working_dir, &EnvSource::Process);
        let process = CodexProcess::start(&executable, &plan, &self.app_version).await?;
        inner.process = Some(Arc::clone(&process));
        Ok(process)
    }

    /// Reads the connected account without asking Codex to refresh its token.
    /// Returns the raw account value for the caller to narrow; no credential
    /// is present in it.
    pub async fn read_account(&self, saved_path: Option<&str>) -> AppResult<serde_json::Value> {
        let process = self.connection(saved_path).await?;
        process
            .request(
                "account/read",
                serde_json::json!({ "refreshToken": false }),
                ACCOUNT_TIMEOUT,
            )
            .await
    }

    /// Sanitized diagnostics from the running process, if there is one.
    pub async fn diagnostics(&self) -> Vec<String> {
        let inner = self.inner.lock().await;
        match &inner.process {
            Some(process) => process.diagnostics(),
            None => Vec::new(),
        }
    }

    /// Stops the process. Called when the app exits.
    pub async fn shutdown(&self) {
        let process = self.inner.lock().await.process.take();
        if let Some(process) = process {
            process.shutdown().await;
        }
    }

    pub fn codex_home(&self) -> &Path {
        &self.codex_home
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(dir: &Path) -> CodexService {
        CodexService::new(dir, "1.0.3")
    }

    #[test]
    fn the_codex_home_is_inside_app_data_and_not_the_users_own() {
        let service = service(Path::new("/app-data"));

        assert_eq!(service.codex_home(), Path::new("/app-data/codex"));
        assert!(!service.codex_home().ends_with(".codex"));
    }

    #[test]
    fn availability_serializes_with_a_state_tag_the_frontend_can_switch_on() {
        let ready = serde_json::to_value(CodexAvailability::Ready {
            version: "0.147.0".into(),
        })
        .unwrap();
        assert_eq!(ready["state"], "ready");
        assert_eq!(ready["version"], "0.147.0");

        let not_found = serde_json::to_value(CodexAvailability::NotFound).unwrap();
        assert_eq!(not_found["state"], "notFound");

        let too_old = serde_json::to_value(CodexAvailability::TooOld {
            version: "0.100.0".into(),
            minimum: "0.147.0".into(),
        })
        .unwrap();
        assert_eq!(too_old["state"], "tooOld");
        // The panel needs both numbers to write a useful sentence.
        assert_eq!(too_old["version"], "0.100.0");
        assert_eq!(too_old["minimum"], "0.147.0");
    }

    #[test]
    fn an_unusable_reason_reaches_the_frontend_as_a_name_not_a_path() {
        let value = serde_json::to_value(CodexAvailability::Unusable {
            reason: UnusableReason::ShimNotSupported,
        })
        .unwrap();

        assert_eq!(value["state"], "unusable");
        assert_eq!(value["reason"], "shimNotSupported");
        // The path is deliberately absent: the user typed it, and repeating it
        // back adds nothing the Settings field does not already show.
        assert!(value.get("path").is_none());
    }

    #[tokio::test]
    async fn a_missing_saved_path_reports_unusable_rather_than_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(dir.path());

        let availability = service
            .availability(Some("/definitely/not/here/codex"))
            .await;

        // Falling back to a different Codex would hide the user's mistake.
        assert_eq!(
            availability,
            CodexAvailability::Unusable {
                reason: UnusableReason::Missing,
            }
        );
    }

    #[tokio::test]
    async fn discovery_is_cached_until_it_is_refreshed() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(dir.path());
        let missing = Some("/definitely/not/here/codex");

        let first = service.availability(missing).await;
        let second = service.availability(missing).await;
        assert_eq!(first, second);

        service.refresh().await;
        let after_refresh = service.availability(missing).await;
        assert_eq!(first, after_refresh);
    }

    #[tokio::test]
    async fn no_connection_means_no_diagnostics_rather_than_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(dir.path());

        assert!(service.diagnostics().await.is_empty());
    }

    #[tokio::test]
    async fn asking_for_a_connection_without_codex_explains_what_to_do() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(dir.path());

        let message = match service.connection(Some("/definitely/not/here/codex")).await {
            Ok(_) => panic!("a missing Codex cannot connect"),
            Err(error) => error.to_string(),
        };

        assert!(message.contains("Settings"), "unhelpful message: {message}");
    }

    #[tokio::test]
    async fn shutting_down_without_a_process_is_harmless() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(dir.path());

        service.shutdown().await;
        service.shutdown().await;
    }
}
