//! Starting, waiting on, and ending a ChatGPT login.
//!
//! Codex owns the OAuth exchange, the tokens, and the refresh. Command Center
//! starts the flow, opens the browser, and waits for Codex to say it finished.
//! No token, code, or callback ever passes through this crate.

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::error::{AppError, AppResult};

/// Starting a login is a local call. Codex answers with a URL or a code
/// almost immediately.
pub const LOGIN_START_TIMEOUT: Duration = Duration::from_secs(30);

/// How long a login may stay open before Command Center stops waiting. The
/// user has to leave the app, authenticate, and come back, so this is
/// deliberately generous.
pub const LOGIN_WAIT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub const LOGOUT_TIMEOUT: Duration = Duration::from_secs(30);

/// Which login Codex should run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginMode {
    /// The default: Codex opens a loopback listener and we open the browser.
    Browser,
    /// Recovery for when the browser flow cannot finish. Codex returns a code
    /// the user types on another device.
    DeviceCode,
}

impl LoginMode {
    pub fn request_params(self) -> Value {
        match self {
            Self::Browser => serde_json::json!({ "type": "chatgpt" }),
            Self::DeviceCode => serde_json::json!({ "type": "chatgptDeviceCode" }),
        }
    }
}

/// What a started login needs from the caller next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginStart {
    /// Open this URL in the user's real browser. It never reaches the
    /// frontend: the IPC layer hands it straight to the system opener.
    Browser { login_id: String, auth_url: String },
    /// Show these to the user so they can authenticate elsewhere.
    DeviceCode {
        login_id: String,
        verification_url: String,
        user_code: String,
    },
}

impl LoginStart {
    pub fn login_id(&self) -> &str {
        match self {
            Self::Browser { login_id, .. } | Self::DeviceCode { login_id, .. } => login_id,
        }
    }
}

/// The part of a started login the frontend may see.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LoginPrompt {
    /// The browser was opened. There is nothing for the user to copy.
    Browser,
    /// Codex could not use a browser, so the user finishes this by hand.
    DeviceCode {
        verification_url: String,
        user_code: String,
    },
}

impl From<&LoginStart> for LoginPrompt {
    fn from(start: &LoginStart) -> Self {
        match start {
            // The authorization URL is deliberately dropped here.
            LoginStart::Browser { .. } => Self::Browser,
            LoginStart::DeviceCode {
                verification_url,
                user_code,
                ..
            } => Self::DeviceCode {
                verification_url: verification_url.clone(),
                user_code: user_code.clone(),
            },
        }
    }
}

/// Reads the reply from `account/login/start`.
pub fn parse_login_start(mode: LoginMode, value: &Value) -> AppResult<LoginStart> {
    let login_id = value
        .get("loginId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::ai_response("Codex started a sign-in without an identifier."))?
        .to_owned();

    match mode {
        LoginMode::Browser => {
            let auth_url = value
                .get("authUrl")
                .and_then(Value::as_str)
                .filter(|url| is_safe_auth_url(url))
                .ok_or_else(|| {
                    AppError::ai_response("Codex did not return a usable sign-in address.")
                })?
                .to_owned();
            Ok(LoginStart::Browser { login_id, auth_url })
        }
        LoginMode::DeviceCode => {
            let verification_url = value
                .get("verificationUrl")
                .and_then(Value::as_str)
                .filter(|url| is_safe_auth_url(url))
                .ok_or_else(|| {
                    AppError::ai_response("Codex did not return a usable sign-in address.")
                })?
                .to_owned();
            let user_code = value
                .get("userCode")
                .and_then(Value::as_str)
                .filter(|code| !code.is_empty() && code.len() <= 64)
                .ok_or_else(|| AppError::ai_response("Codex did not return a sign-in code."))?
                .to_owned();
            Ok(LoginStart::DeviceCode {
                login_id,
                verification_url,
                user_code,
            })
        }
    }
}

/// Only an ordinary https address is ever opened or shown.
///
/// The URL comes from Codex rather than the user, but it is still handed to
/// the operating system's opener, so a non-https scheme is refused rather
/// than passed along.
fn is_safe_auth_url(url: &str) -> bool {
    const MAX_URL_LENGTH: usize = 4096;

    url.len() <= MAX_URL_LENGTH
        && url.starts_with("https://")
        && !url.chars().any(char::is_control)
        && !url.chars().any(char::is_whitespace)
}

/// The outcome carried by an `account/login/completed` notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOutcome {
    Succeeded,
    Failed { reported: Option<String> },
}

/// Reads a completion notification, returning `None` when it belongs to a
/// different login attempt.
///
/// A notification with no `loginId` is treated as belonging to the attempt in
/// progress: Codex reports one login at a time, and dropping it would leave
/// the panel waiting forever.
pub fn parse_login_completed(expected_login_id: &str, params: &Value) -> Option<LoginOutcome> {
    let reported_id = params.get("loginId").and_then(Value::as_str);
    if let Some(reported_id) = reported_id {
        if reported_id != expected_login_id {
            return None;
        }
    }

    if params.get("success").and_then(Value::as_bool) == Some(true) {
        return Some(LoginOutcome::Succeeded);
    }

    Some(LoginOutcome::Failed {
        reported: params
            .get("error")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| text.chars().take(200).collect()),
    })
}

/// Turns a failed login into a message for the user.
///
/// Codex's own text is short and written for people, so it is shown when
/// present, but never with an address or payload attached.
pub fn login_failure_error(reported: Option<&str>) -> AppError {
    match reported.filter(|text| is_safe_to_show(text)) {
        Some(text) => AppError::ai_auth(format!("ChatGPT sign-in did not finish: {text}")),
        None => AppError::ai_auth(
            "ChatGPT sign-in did not finish. Start it again from Settings.".to_owned(),
        ),
    }
}

fn is_safe_to_show(text: &str) -> bool {
    !text.contains("://") && !text.contains("Bearer") && !text.contains("sk-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_two_modes_ask_codex_for_different_flows() {
        assert_eq!(LoginMode::Browser.request_params()["type"], "chatgpt");
        assert_eq!(
            LoginMode::DeviceCode.request_params()["type"],
            "chatgptDeviceCode"
        );
    }

    #[test]
    fn the_forbidden_token_login_mode_is_never_requested() {
        // Codex marks this internal and unstable. Feeding it a token would
        // also mean handling one, which this app must never do.
        for mode in [LoginMode::Browser, LoginMode::DeviceCode] {
            let params = mode.request_params().to_string();
            assert!(!params.contains("chatgptAuthTokens"));
            assert!(!params.contains("accessToken"));
        }
    }

    #[test]
    fn a_browser_login_keeps_its_id_and_address() {
        let reply = json!({
            "type": "chatgpt",
            "loginId": "login-1",
            "authUrl": "https://auth.openai.com/authorize?client_id=abc",
        });

        assert_eq!(
            parse_login_start(LoginMode::Browser, &reply).unwrap(),
            LoginStart::Browser {
                login_id: "login-1".into(),
                auth_url: "https://auth.openai.com/authorize?client_id=abc".into(),
            }
        );
    }

    #[test]
    fn a_device_code_login_keeps_the_code_and_verification_address() {
        let reply = json!({
            "type": "chatgptDeviceCode",
            "loginId": "login-2",
            "verificationUrl": "https://auth.openai.com/device",
            "userCode": "ABCD-EFGH",
        });

        assert_eq!(
            parse_login_start(LoginMode::DeviceCode, &reply).unwrap(),
            LoginStart::DeviceCode {
                login_id: "login-2".into(),
                verification_url: "https://auth.openai.com/device".into(),
                user_code: "ABCD-EFGH".into(),
            }
        );
    }

    #[test]
    fn a_login_without_an_identifier_is_refused() {
        let reply = json!({ "authUrl": "https://auth.openai.com/authorize" });
        assert!(parse_login_start(LoginMode::Browser, &reply).is_err());
    }

    #[test]
    fn a_non_https_sign_in_address_is_refused_rather_than_opened() {
        for bad in [
            "http://auth.openai.com/authorize",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://auth.openai.com/a b",
        ] {
            let reply = json!({ "loginId": "login-1", "authUrl": bad });
            assert!(
                parse_login_start(LoginMode::Browser, &reply).is_err(),
                "accepted {bad}",
            );
        }
    }

    #[test]
    fn the_authorization_url_never_reaches_the_frontend() {
        let start = LoginStart::Browser {
            login_id: "login-1".into(),
            auth_url: "https://auth.openai.com/authorize?code_challenge=secret".into(),
        };

        let prompt = LoginPrompt::from(&start);
        let encoded = serde_json::to_string(&prompt).unwrap();

        assert_eq!(prompt, LoginPrompt::Browser);
        assert!(!encoded.contains("auth.openai.com"));
        assert!(!encoded.contains("code_challenge"));
    }

    #[test]
    fn a_device_code_prompt_does_reach_the_frontend() {
        // The user cannot finish this flow without seeing both.
        let start = LoginStart::DeviceCode {
            login_id: "login-2".into(),
            verification_url: "https://auth.openai.com/device".into(),
            user_code: "ABCD-EFGH".into(),
        };

        let value = serde_json::to_value(LoginPrompt::from(&start)).unwrap();
        assert_eq!(value["mode"], "deviceCode");
        assert_eq!(value["userCode"], "ABCD-EFGH");
        assert_eq!(value["verificationUrl"], "https://auth.openai.com/device");
    }

    #[test]
    fn a_completion_for_another_login_is_ignored() {
        let params = json!({ "loginId": "someone-else", "success": true });
        assert_eq!(parse_login_completed("mine", &params), None);
    }

    #[test]
    fn a_matching_success_completes_the_login() {
        let params = json!({ "loginId": "mine", "success": true });
        assert_eq!(
            parse_login_completed("mine", &params),
            Some(LoginOutcome::Succeeded)
        );
    }

    #[test]
    fn a_completion_without_an_id_is_taken_as_ours() {
        // Codex runs one login at a time. Dropping this would leave the panel
        // waiting until the timeout for a login that already finished.
        let params = json!({ "success": true });
        assert_eq!(
            parse_login_completed("mine", &params),
            Some(LoginOutcome::Succeeded)
        );
    }

    #[test]
    fn a_failure_carries_what_codex_reported() {
        let params = json!({ "loginId": "mine", "success": false, "error": "access denied" });

        assert_eq!(
            parse_login_completed("mine", &params),
            Some(LoginOutcome::Failed {
                reported: Some("access denied".into())
            })
        );
    }

    #[test]
    fn a_failure_message_is_shown_when_it_is_plain_text() {
        let error = login_failure_error(Some("access denied"));
        assert!(error.to_string().contains("access denied"));
    }

    #[test]
    fn a_failure_message_carrying_an_address_or_token_is_replaced() {
        for unsafe_text in [
            "failed at https://auth.openai.com/callback?code=abc",
            "Bearer sk-live-1234 was rejected",
            "sk-proj-secret is invalid",
        ] {
            let message = login_failure_error(Some(unsafe_text)).to_string();
            assert!(!message.contains("://"), "leaked an address: {message}");
            assert!(!message.contains("sk-"), "leaked a token: {message}");
            assert!(message.contains("Start it again"));
        }
    }

    #[test]
    fn an_absurdly_long_failure_message_is_bounded() {
        let params = json!({ "success": false, "error": "x".repeat(5_000) });

        match parse_login_completed("mine", &params) {
            Some(LoginOutcome::Failed { reported }) => {
                assert_eq!(reported.unwrap().chars().count(), 200);
            }
            other => panic!("expected a failure, got {other:?}"),
        }
    }
}
