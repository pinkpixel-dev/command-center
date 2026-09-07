//! Account, model, and turn operations against a live Codex connection.
//!
//! These sit apart from the connection bookkeeping in the parent module so
//! neither file grows past the repository limit. They are still methods on
//! [`CodexService`], so callers see one type.

use std::time::Duration;

use tokio::sync::broadcast::error::RecvError;

use super::{ActiveLogin, CodexService};
use crate::ai::prompts::StructuredTask;
use crate::ai::providers::codex::auth::{
    self, LoginMode, LoginOutcome, LoginStart, LOGIN_START_TIMEOUT, LOGIN_WAIT_TIMEOUT,
    LOGOUT_TIMEOUT,
};
use crate::ai::providers::codex::models::{self, CodexModel, MAX_MODEL_PAGES, MODEL_LIST_TIMEOUT};
use crate::ai::providers::codex::process::Notification;
use crate::ai::providers::codex::turn::{
    self, ItemDisposition, INTERRUPT_TIMEOUT, THREAD_START_TIMEOUT, TURN_START_TIMEOUT,
};
use crate::error::{AppError, AppResult};

impl CodexService {
    /// Starts a ChatGPT login and returns what the caller must do next.
    ///
    /// The watcher subscribes to notifications *before* the request goes out,
    /// so a login that finishes immediately cannot be missed.
    pub async fn start_login(
        &self,
        saved_path: Option<&str>,
        mode: LoginMode,
    ) -> AppResult<LoginStart> {
        let process = self.connection(saved_path).await?;

        // Subscribe first. Everything after this point is observed.
        let notifications = process.subscribe();

        let reply = process
            .request("account/login/start", mode.request_params(), LOGIN_START_TIMEOUT)
            .await?;
        let start = auth::parse_login_start(mode, &reply)?;

        let (sender, receiver) = tokio::sync::oneshot::channel();
        tokio::spawn(watch_for_login(
            notifications,
            start.login_id().to_owned(),
            sender,
        ));

        let mut inner = self.inner.lock().await;
        inner.active_login = Some(ActiveLogin {
            login_id: start.login_id().to_owned(),
            completion: receiver,
        });
        Ok(start)
    }

    /// Waits for the active login to finish.
    pub async fn await_login(&self) -> AppResult<()> {
        let login = self
            .inner
            .lock()
            .await
            .active_login
            .take()
            .ok_or_else(|| AppError::invalid("No ChatGPT sign-in is in progress."))?;

        let outcome = match tokio::time::timeout(LOGIN_WAIT_TIMEOUT, login.completion).await {
            Ok(Ok(outcome)) => outcome,
            // The watcher stopped, which means the connection went away.
            Ok(Err(_)) => {
                return Err(AppError::ai_network(
                    "Codex stopped while waiting for the sign-in.",
                ))
            }
            Err(_) => {
                let _ = self.cancel_login_by_id(&login.login_id).await;
                return Err(AppError::ai_auth(
                    "The ChatGPT sign-in timed out. Start it again from Settings.",
                ));
            }
        };

        match outcome {
            LoginOutcome::Succeeded => Ok(()),
            LoginOutcome::Failed { reported } => Err(auth::login_failure_error(reported.as_deref())),
        }
    }

    /// Cancels the active login, if there is one. Leaving an abandoned login
    /// running would keep Codex's callback listener open.
    pub async fn cancel_login(&self) -> AppResult<()> {
        let login = self.inner.lock().await.active_login.take();
        match login {
            Some(login) => self.cancel_login_by_id(&login.login_id).await,
            None => Ok(()),
        }
    }

    async fn cancel_login_by_id(&self, login_id: &str) -> AppResult<()> {
        let process = match self.inner.lock().await.process.clone() {
            Some(process) => process,
            None => return Ok(()),
        };
        process
            .request(
                "account/login/cancel",
                serde_json::json!({ "loginId": login_id }),
                LOGIN_START_TIMEOUT,
            )
            .await
            .map(|_| ())
    }

    /// Disconnects the account from Command Center's Codex home.
    ///
    /// Codex owns its credential files. This asks it to log out and never
    /// deletes anything by hand.
    pub async fn logout(&self, saved_path: Option<&str>) -> AppResult<()> {
        let process = self.connection(saved_path).await?;
        let _ = self.cancel_login().await;
        process
            .request("account/logout", serde_json::json!({}), LOGOUT_TIMEOUT)
            .await
            .map(|_| ())
    }

    /// Reads the live model catalogue for the connected account.
    pub async fn list_models(&self, saved_path: Option<&str>) -> AppResult<Vec<CodexModel>> {
        let process = self.connection(saved_path).await?;

        let mut collected = Vec::new();
        let mut cursor: Option<String> = None;

        for _ in 0..MAX_MODEL_PAGES {
            let params = match &cursor {
                Some(cursor) => serde_json::json!({ "cursor": cursor }),
                None => serde_json::json!({}),
            };
            let reply = process
                .request("model/list", params, MODEL_LIST_TIMEOUT)
                .await?;

            let (page, next) = models::parse_model_page(&reply);
            collected.extend(page);
            match next {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        Ok(models::arrange(collected))
    }

    /// Runs one structured task as a single ephemeral turn and returns the
    /// JSON text it produced.
    pub async fn run_structured_task(
        &self,
        saved_path: Option<&str>,
        model: &str,
        task: &StructuredTask,
        input: &str,
        timeout: Duration,
    ) -> AppResult<String> {
        let process = self.connection(saved_path).await?;

        // Subscribe before the thread exists so no early item is missed.
        let mut notifications = process.subscribe();

        let thread = process
            .request(
                "thread/start",
                turn::thread_params(model, &self.working_dir, task.instructions),
                THREAD_START_TIMEOUT,
            )
            .await?;
        let thread_id = thread["thread"]["id"]
            .as_str()
            .ok_or_else(|| AppError::ai_response("Codex started a thread without an identifier."))?
            .to_owned();

        let started = process
            .request(
                "turn/start",
                turn::turn_params(&thread_id, input, task),
                TURN_START_TIMEOUT,
            )
            .await?;
        let turn_id = started["turn"]["id"].as_str().unwrap_or_default().to_owned();

        let result = tokio::time::timeout(
            timeout,
            collect_turn(&mut notifications, &thread_id, &turn_id),
        )
        .await;

        match result {
            Ok(outcome) => outcome,
            Err(_) => {
                // Stop the work rather than leaving it running upstream.
                let _ = process
                    .request(
                        "turn/interrupt",
                        serde_json::json!({ "threadId": thread_id, "turnId": turn_id }),
                        INTERRUPT_TIMEOUT,
                    )
                    .await;
                Err(AppError::ai_network("Codex did not answer in time."))
            }
        }
    }
}

/// Watches for the completion notification belonging to one login attempt.
async fn watch_for_login(
    mut notifications: tokio::sync::broadcast::Receiver<Notification>,
    login_id: String,
    sender: tokio::sync::oneshot::Sender<LoginOutcome>,
) {
    loop {
        let notification = match notifications.recv().await {
            Ok(notification) => notification,
            // Lagged means notifications were dropped under load. Keep
            // listening: the completion may still be ahead of us.
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => return,
        };

        if notification.method != "account/login/completed" {
            continue;
        }
        if let Some(outcome) = auth::parse_login_completed(&login_id, &notification.params) {
            let _ = sender.send(outcome);
            return;
        }
    }
}

/// Reads one turn to completion, enforcing that it only ever answers.
async fn collect_turn(
    notifications: &mut tokio::sync::broadcast::Receiver<Notification>,
    thread_id: &str,
    turn_id: &str,
) -> AppResult<String> {
    let mut collected = String::new();

    loop {
        let notification = match notifications.recv().await {
            Ok(notification) => notification,
            Err(RecvError::Lagged(_)) => {
                // Dropped deltas mean the answer would be incomplete, and a
                // truncated result must never be parsed as a whole one.
                return Err(AppError::AiIncomplete(
                    "The reply from Codex arrived faster than it could be read.".into(),
                ));
            }
            Err(RecvError::Closed) => {
                return Err(AppError::ai_network("Codex stopped during the request."))
            }
        };

        let params = &notification.params;
        if params.get("threadId").and_then(serde_json::Value::as_str) != Some(thread_id) {
            continue;
        }

        match notification.method.as_str() {
            "item/agentMessage/delta" => {
                if let Some(delta) = params.get("delta").and_then(serde_json::Value::as_str) {
                    turn::append_bounded(&mut collected, delta)?;
                }
            }
            "item/started" | "item/completed" => {
                let item = &params["item"];
                let item_type = item
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();

                match turn::item_disposition(item_type) {
                    ItemDisposition::Forbidden => {
                        return Err(turn::forbidden_item_error(item_type))
                    }
                    ItemDisposition::Ignored => {}
                    ItemDisposition::Answer => {
                        // The completed item is authoritative. Prefer it over
                        // whatever the deltas accumulated.
                        if notification.method == "item/completed" {
                            if let Some(text) = turn::agent_text(item) {
                                collected = text;
                            }
                        }
                    }
                }
            }
            "turn/completed" => {
                let turn_value = &params["turn"];
                let completed_id = turn_value.get("id").and_then(serde_json::Value::as_str);
                // An empty expected id means turn/start did not report one;
                // there is only one turn on this thread either way.
                if !turn_id.is_empty() && completed_id.is_some_and(|id| id != turn_id) {
                    continue;
                }
                return turn::turn_outcome(turn_value, &collected);
            }
            _ => {}
        }
    }
}

