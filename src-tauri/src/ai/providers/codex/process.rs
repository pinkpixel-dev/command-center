//! The live Codex app-server process and its JSON-RPC connection.
//!
//! One process, one connection, one reader task. Requests are matched to
//! responses by a monotonic id. Notifications are broadcast. Server requests
//! are always answered, because an ignored one leaves Codex waiting.
//!
//! Nothing here returns a process handle, a raw message, or a credential to
//! the caller. The IPC layer above it only ever sees typed values.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin};
use tokio::sync::{broadcast, oneshot};

use super::launch::{initialize_params, LaunchPlan};
use super::lines::{BoundedLines, LineError};
use super::rpc::{self, Incoming, ParseError, RpcError, MAX_MESSAGE_BYTES};
use crate::error::{AppError, AppResult};

/// The handshake is local work. If it has not finished by now, the executable
/// is not a working app server.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// How many sanitized diagnostic lines are kept for the Settings panel.
const DIAGNOSTIC_LINES: usize = 20;

/// How many notification messages may queue before a slow subscriber starts
/// losing the oldest ones. Losing deltas is survivable; unbounded growth is
/// not.
const NOTIFICATION_CAPACITY: usize = 256;

/// Why Command Center refuses whatever the server just asked for. Approvals
/// get their own typed decline once turns are routed through this provider;
/// until then every server request is refused the same way.
const SERVER_REQUEST_REFUSAL: &str =
    "Command Center does not grant tool, command, or file requests.";

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, RpcError>>>>>;
type Diagnostics = Arc<Mutex<std::collections::VecDeque<String>>>;

pub struct CodexProcess {
    child: Mutex<Option<Child>>,
    stdin: tokio::sync::Mutex<ChildStdin>,
    next_id: AtomicI64,
    pending: PendingMap,
    notifications: broadcast::Sender<Notification>,
    diagnostics: Diagnostics,
}

impl CodexProcess {
    /// Spawns the app server and completes the `initialize` / `initialized`
    /// handshake. Nothing else may be sent before this returns.
    pub async fn start(
        executable: &Path,
        plan: &LaunchPlan,
        app_version: &str,
    ) -> AppResult<Arc<Self>> {
        std::fs::create_dir_all(&plan.codex_home).map_err(|error| {
            AppError::runtime(format!("could not create the Codex directory: {}", error.kind()))
        })?;
        std::fs::create_dir_all(&plan.working_dir).map_err(|error| {
            AppError::runtime(format!(
                "could not create the Codex working directory: {}",
                error.kind()
            ))
        })?;

        let mut command = tokio::process::Command::new(executable);
        command
            .args(&plan.args)
            .env_clear()
            .current_dir(&plan.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (name, value) in &plan.env {
            command.env(name, value);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn().map_err(|error| {
            AppError::ai_network(format!("Codex could not be started ({}).", error.kind()))
        })?;

        let stdin = child.stdin.take().ok_or_else(unavailable)?;
        let stdout = child.stdout.take().ok_or_else(unavailable)?;
        let stderr = child.stderr.take().ok_or_else(unavailable)?;

        let pending: PendingMap = Arc::default();
        let diagnostics: Diagnostics = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        let (notifications, _) = broadcast::channel(NOTIFICATION_CAPACITY);

        let process = Arc::new(Self {
            child: Mutex::new(Some(child)),
            stdin: tokio::sync::Mutex::new(stdin),
            next_id: AtomicI64::new(0),
            pending: Arc::clone(&pending),
            notifications: notifications.clone(),
            diagnostics: Arc::clone(&diagnostics),
        });

        tokio::spawn(read_stdout(
            stdout,
            Arc::downgrade(&process),
            pending,
            notifications,
            Arc::clone(&diagnostics),
        ));
        tokio::spawn(read_stderr(stderr, diagnostics));

        process.handshake(app_version).await?;
        Ok(process)
    }

    async fn handshake(&self, app_version: &str) -> AppResult<()> {
        self.request("initialize", initialize_params(app_version), HANDSHAKE_TIMEOUT)
            .await?;
        self.notify("initialized", serde_json::json!({})).await?;
        Ok(())
    }

    /// Sends a request and waits for its response.
    pub async fn request(
        &self,
        method: &str,
        params: impl Serialize,
        timeout: Duration,
    ) -> AppResult<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let line = rpc::request_line(id, method, params)
            .map_err(|_| AppError::runtime("could not encode a Codex request"))?;

        let (sender, receiver) = oneshot::channel();
        self.pending
            .lock()
            .expect("pending map is not poisoned")
            .insert(id, sender);

        if let Err(error) = self.write_line(&line).await {
            self.pending.lock().expect("pending map is not poisoned").remove(&id);
            return Err(error);
        }

        match tokio::time::timeout(timeout, receiver).await {
            Ok(Ok(Ok(result))) => Ok(result),
            Ok(Ok(Err(error))) => Err(map_rpc_error(method, &error)),
            // The reader task dropped the sender, which means the connection
            // is gone.
            Ok(Err(_)) => Err(AppError::ai_network(
                "Codex stopped before it answered. Try again.",
            )),
            Err(_) => {
                self.pending.lock().expect("pending map is not poisoned").remove(&id);
                Err(AppError::ai_network(format!(
                    "Codex did not answer the {method} request in time."
                )))
            }
        }
    }

    pub async fn notify(&self, method: &str, params: impl Serialize) -> AppResult<()> {
        let line = rpc::notification_line(method, params)
            .map_err(|_| AppError::runtime("could not encode a Codex notification"))?;
        self.write_line(&line).await
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.notifications.subscribe()
    }

    /// Sanitized diagnostic lines for the Settings panel. Never contains a
    /// request body, an authorization URL, or a credential, because only
    /// bounded stderr text and unknown-method names are recorded.
    pub fn diagnostics(&self) -> Vec<String> {
        self.diagnostics
            .lock()
            .expect("diagnostics are not poisoned")
            .iter()
            .cloned()
            .collect()
    }

    async fn write_line(&self, line: &str) -> AppResult<()> {
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|_| connection_lost())?;
        stdin.flush().await.map_err(|_| connection_lost())
    }

    /// Ends the process. Safe to call more than once.
    pub async fn shutdown(&self) {
        let child = self.child.lock().expect("child handle is not poisoned").take();
        if let Some(mut child) = child {
            // Closing stdin is the polite exit. The kill is the guarantee.
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        self.fail_pending("Codex was stopped.");
    }

    /// True while the process is still running.
    pub fn is_running(&self) -> bool {
        let mut guard = self.child.lock().expect("child handle is not poisoned");
        match guard.as_mut() {
            Some(child) => !matches!(child.try_wait(), Ok(Some(_))),
            None => false,
        }
    }

    fn fail_pending(&self, reason: &str) {
        let waiting: Vec<_> = self
            .pending
            .lock()
            .expect("pending map is not poisoned")
            .drain()
            .map(|(_, sender)| sender)
            .collect();
        for sender in waiting {
            let _ = sender.send(Err(RpcError {
                code: 0,
                message: reason.to_owned(),
            }));
        }
    }
}

impl Drop for CodexProcess {
    fn drop(&mut self) {
        // `kill_on_drop` handles the child; this only releases anything still
        // waiting on a response so no caller hangs forever.
        if let Ok(mut pending) = self.pending.lock() {
            pending.clear();
        }
    }
}

async fn read_stdout(
    stdout: tokio::process::ChildStdout,
    process: std::sync::Weak<CodexProcess>,
    pending: PendingMap,
    notifications: broadcast::Sender<Notification>,
    diagnostics: Diagnostics,
) {
    let mut lines = BoundedLines::new(stdout, MAX_MESSAGE_BYTES);

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(LineError::TooLong) => {
                record(&diagnostics, "Codex sent an oversized message.");
                break;
            }
            Err(LineError::Io(kind)) => {
                record(&diagnostics, &format!("Codex output ended ({kind})."));
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        match rpc::parse_incoming(&line) {
            Ok(Incoming::Response { id, result }) => {
                resolve(&pending, id, Ok(result));
            }
            Ok(Incoming::ErrorResponse { id, error }) => {
                resolve(&pending, id, Err(error));
            }
            Ok(Incoming::Notification { method, params }) => {
                // A send failure only means nobody is listening yet.
                let _ = notifications.send(Notification { method, params });
            }
            Ok(Incoming::ServerRequest { id, method, .. }) => {
                // Answer every server request. Ignoring one leaves Codex
                // waiting and can wedge a turn.
                record(&diagnostics, &format!("Declined a {method} request."));
                if let Some(process) = process.upgrade() {
                    if let Ok(reply) = rpc::rejection_line(&id, SERVER_REQUEST_REFUSAL) {
                        let _ = process.write_line(&reply).await;
                    }
                }
            }
            Err(ParseError::TooLarge) => {
                record(&diagnostics, "Codex sent an oversized message.");
                break;
            }
            Err(ParseError::Malformed) => {
                record(&diagnostics, "Codex sent a message the app could not read.");
            }
            Err(ParseError::Unrecognized) => {
                record(&diagnostics, "Codex sent an unrecognized message.");
            }
        }
    }

    // The stream ended. Release everything still waiting rather than letting
    // callers sit until their own timeouts.
    let waiting: Vec<_> = pending
        .lock()
        .expect("pending map is not poisoned")
        .drain()
        .map(|(_, sender)| sender)
        .collect();
    for sender in waiting {
        let _ = sender.send(Err(RpcError {
            code: 0,
            message: "Codex stopped responding.".to_owned(),
        }));
    }
}

async fn read_stderr(stderr: tokio::process::ChildStderr, diagnostics: Diagnostics) {
    // Codex writes warnings and tracing here. It is bounded and recorded, but
    // never surfaced verbatim as an error message.
    let mut lines = BoundedLines::new(stderr, MAX_MESSAGE_BYTES);
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            record(&diagnostics, trimmed);
        }
    }
}

fn resolve(pending: &PendingMap, id: i64, outcome: Result<Value, RpcError>) {
    let sender = pending
        .lock()
        .expect("pending map is not poisoned")
        .remove(&id);
    if let Some(sender) = sender {
        let _ = sender.send(outcome);
    }
}

/// Diagnostics are bounded and truncated. They are shown to the user in a
/// details panel, so they must stay short and free of payloads.
fn record(diagnostics: &Diagnostics, message: &str) {
    const MAX_DIAGNOSTIC_CHARS: usize = 300;

    let mut trimmed: String = message.chars().take(MAX_DIAGNOSTIC_CHARS).collect();
    if message.chars().count() > MAX_DIAGNOSTIC_CHARS {
        trimmed.push('…');
    }

    let mut log = diagnostics.lock().expect("diagnostics are not poisoned");
    if log.len() == DIAGNOSTIC_LINES {
        log.pop_front();
    }
    log.push_back(trimmed);
}

/// Codex error text is provider output. It can carry an authorization URL, a
/// request body, or an account identifier, so the message the user sees names
/// the failed request and nothing else. The original stays in diagnostics.
fn map_rpc_error(method: &str, _error: &RpcError) -> AppError {
    AppError::ai_response(format!("Codex could not complete the {method} request."))
}

fn connection_lost() -> AppError {
    AppError::ai_network("The connection to Codex was lost. Try again.")
}

fn unavailable() -> AppError {
    AppError::ai_network("Codex started without a usable connection.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn diagnostics() -> Diagnostics {
        Arc::new(Mutex::new(std::collections::VecDeque::new()))
    }

    #[test]
    fn diagnostics_are_bounded_to_the_most_recent_lines() {
        let log = diagnostics();
        for index in 0..DIAGNOSTIC_LINES + 10 {
            record(&log, &format!("line {index}"));
        }

        let stored = log.lock().unwrap();
        assert_eq!(stored.len(), DIAGNOSTIC_LINES);
        assert_eq!(stored.front().unwrap(), "line 10");
        assert_eq!(stored.back().unwrap(), "line 29");
    }

    #[test]
    fn a_long_diagnostic_line_is_truncated() {
        let log = diagnostics();
        record(&log, &"x".repeat(5_000));

        let stored = log.lock().unwrap();
        assert!(stored.back().unwrap().chars().count() <= 301);
        assert!(stored.back().unwrap().ends_with('…'));
    }

    #[tokio::test]
    async fn a_response_resolves_only_its_own_request() {
        let pending: PendingMap = Arc::default();
        let (first_tx, first_rx) = oneshot::channel();
        let (second_tx, second_rx) = oneshot::channel();
        pending.lock().unwrap().insert(1, first_tx);
        pending.lock().unwrap().insert(2, second_tx);

        resolve(&pending, 2, Ok(json!({ "ok": true })));

        assert_eq!(second_rx.await.unwrap().unwrap()["ok"], true);
        assert_eq!(pending.lock().unwrap().len(), 1);
        // The untouched request is still waiting.
        drop(first_rx);
    }

    #[tokio::test]
    async fn resolving_an_unknown_id_is_harmless() {
        let pending: PendingMap = Arc::default();
        resolve(&pending, 99, Ok(json!({})));
        assert!(pending.lock().unwrap().is_empty());
    }

    #[test]
    fn provider_errors_do_not_repeat_raw_codex_text() {
        let error = map_rpc_error(
            "turn/start",
            &RpcError {
                code: 401,
                message: "Bearer sk-live-secret rejected at https://api.openai.com".into(),
            },
        );
        let message = error.to_string();

        assert!(message.contains("turn/start"));
        assert!(!message.contains("sk-live-secret"));
        assert!(!message.contains("api.openai.com"));
    }

    #[test]
    fn the_refusal_message_says_what_was_refused() {
        assert!(SERVER_REQUEST_REFUSAL.contains("does not grant"));
        assert!(SERVER_REQUEST_REFUSAL.contains("file"));
    }
}
