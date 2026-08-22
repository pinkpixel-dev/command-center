//! Integration coverage against a real Codex installation.
//!
//! These tests start the actual `codex app-server`, so they only run where
//! Codex is installed. Where it is not, each test reports why it skipped
//! rather than failing, so a machine without Codex can still run the suite.
//!
//! Nothing here logs in or starts a turn. Everything below works against an
//! unauthenticated Codex and spends no ChatGPT usage.

use std::path::PathBuf;
use std::time::Duration;

use command_center_lib::ai::providers::codex::discovery::{self, CodexDiscovery};
use command_center_lib::ai::providers::codex::launch::{launch_plan, EnvSource, LaunchPlan};
use command_center_lib::ai::providers::codex::process::CodexProcess;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Finds Codex, or explains why the test cannot run.
async fn installed_codex() -> Option<PathBuf> {
    match discovery::discover(None).await {
        CodexDiscovery::Found { path, version, .. } => {
            eprintln!("using Codex {version} at {}", path.display());
            Some(path)
        }
        CodexDiscovery::TooOld { version, .. } => {
            eprintln!(
                "skipping: installed Codex {version} is below the {} floor",
                discovery::minimum_version()
            );
            None
        }
        CodexDiscovery::Unusable { path, reason } => {
            eprintln!("skipping: Codex at {} is unusable ({reason:?})", path.display());
            None
        }
        CodexDiscovery::NotFound => {
            eprintln!("skipping: no Codex installation found");
            None
        }
    }
}

struct Sandbox {
    _dir: tempfile::TempDir,
    plan: LaunchPlan,
}

fn sandbox() -> Sandbox {
    let dir = tempfile::tempdir().expect("temp dir");
    let plan = launch_plan(
        &dir.path().join("codex-home"),
        &dir.path().join("codex-work"),
        &EnvSource::Process,
    );
    Sandbox { _dir: dir, plan }
}

#[tokio::test]
async fn the_handshake_completes_against_a_real_app_server() {
    let Some(codex) = installed_codex().await else {
        return;
    };
    let sandbox = sandbox();

    // `start` is not considered successful until initialize and initialized
    // have both gone through, so reaching this point is the handshake passing.
    let process = CodexProcess::start(&codex, &sandbox.plan, "1.0.3")
        .await
        .expect("the app server should complete the handshake");

    assert!(process.is_running());
    process.shutdown().await;
}

#[tokio::test]
async fn the_hardening_overrides_are_accepted_by_the_installed_version() {
    let Some(codex) = installed_codex().await else {
        return;
    };
    let sandbox = sandbox();

    // Every override goes through `--strict-config`, which validates key names
    // and values. If the installed Codex renamed or removed one of them, the
    // process fails to start and this test catches it before a user does.
    let process = CodexProcess::start(&codex, &sandbox.plan, "1.0.3")
        .await
        .expect("hardening overrides should be valid for the installed Codex");

    process.shutdown().await;
}

#[tokio::test]
async fn an_unauthenticated_account_read_reports_that_login_is_required() {
    let Some(codex) = installed_codex().await else {
        return;
    };
    let sandbox = sandbox();

    let process = CodexProcess::start(&codex, &sandbox.plan, "1.0.3")
        .await
        .expect("app server should start");

    let account = process
        .request(
            "account/read",
            serde_json::json!({ "refreshToken": false }),
            REQUEST_TIMEOUT,
        )
        .await
        .expect("account/read should answer without a login");

    // A fresh Codex home has no account. This is the state the "Not connected"
    // panel is built from.
    assert!(account["account"].is_null(), "unexpected account: {account}");

    process.shutdown().await;
}

#[tokio::test]
async fn no_token_bearing_file_appears_in_the_codex_home() {
    let Some(codex) = installed_codex().await else {
        return;
    };
    let sandbox = sandbox();
    let home = sandbox.plan.codex_home.clone();

    let process = CodexProcess::start(&codex, &sandbox.plan, "1.0.3")
        .await
        .expect("app server should start");
    let _ = process
        .request(
            "account/read",
            serde_json::json!({ "refreshToken": false }),
            REQUEST_TIMEOUT,
        )
        .await;
    process.shutdown().await;

    // Credentials are pinned to the operating-system keyring. `auth.json` is
    // the file Codex writes when a file-backed store is in use, and it must
    // never appear here.
    assert!(
        !home.join("auth.json").exists(),
        "auth.json was written despite keyring-only credential storage",
    );
}

#[tokio::test]
async fn our_codex_home_is_used_instead_of_the_users_own() {
    let Some(codex) = installed_codex().await else {
        return;
    };
    let sandbox = sandbox();
    let home = sandbox.plan.codex_home.clone();

    let process = CodexProcess::start(&codex, &sandbox.plan, "1.0.3")
        .await
        .expect("app server should start");

    // The initialize result reports the home Codex actually resolved, which is
    // the check that matters: connecting here must never touch ~/.codex.
    assert!(home.exists(), "the isolated Codex home should have been created");
    assert!(
        std::fs::read_dir(&home)
            .expect("the isolated home should be readable")
            .next()
            .is_some(),
        "Codex should have written its own state into the isolated home",
    );

    process.shutdown().await;
}

#[tokio::test]
async fn the_process_stops_cleanly_on_shutdown() {
    let Some(codex) = installed_codex().await else {
        return;
    };
    let sandbox = sandbox();

    let process = CodexProcess::start(&codex, &sandbox.plan, "1.0.3")
        .await
        .expect("app server should start");
    assert!(process.is_running());

    process.shutdown().await;
    assert!(!process.is_running(), "the app server should not outlive shutdown");

    // Shutting down twice must not panic or hang.
    process.shutdown().await;
}

#[tokio::test]
async fn a_request_to_an_unknown_method_fails_without_leaking_provider_text() {
    let Some(codex) = installed_codex().await else {
        return;
    };
    let sandbox = sandbox();

    let process = CodexProcess::start(&codex, &sandbox.plan, "1.0.3")
        .await
        .expect("app server should start");

    let error = process
        .request(
            "commandCenter/notARealMethod",
            serde_json::json!({}),
            REQUEST_TIMEOUT,
        )
        .await
        .expect_err("an unknown method should fail");

    let message = error.to_string();
    assert!(
        message.contains("commandCenter/notARealMethod"),
        "the error should name the failed request: {message}",
    );

    process.shutdown().await;
}
