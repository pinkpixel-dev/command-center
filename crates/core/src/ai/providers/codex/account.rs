//! Narrowing Codex's `account/read` reply into what Settings may see.
//!
//! Codex owns the ChatGPT credentials. This module deliberately keeps only the
//! few display fields the connection panel needs, so nothing token-shaped can
//! reach the frontend even if the protocol grows new fields later.

use serde::Serialize;
use serde_json::Value;

/// The connection state shown in AI Settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CodexAccount {
    /// No account in Command Center's Codex home. The user has not connected
    /// here yet, even if they are signed in to their own Codex CLI.
    NotConnected,
    /// A connected ChatGPT account.
    Connected {
        /// Absent when Codex does not report one.
        email: Option<String>,
        /// A readable plan name for the badge. Display metadata only:
        /// entitlements are enforced upstream, so this must never gate a
        /// feature. The raw identifier is not kept, because nothing uses it.
        plan: Option<String>,
    },
    /// Codex is authenticated by something that is not a ChatGPT account.
    /// Command Center never creates this state, but reporting it honestly
    /// beats showing "not connected" for an account that exists.
    ConnectedWithOtherCredentials { kind: String },
}

/// The longest account field Command Center will echo back. Codex controls
/// this text, so it is bounded before it reaches the UI.
const MAX_FIELD_LENGTH: usize = 320;

/// Reads the reply from `account/read`.
///
/// An unexpected shape reads as "not connected" rather than failing. A status
/// panel that cannot render is worse than one that offers Connect again.
pub fn parse_account(value: &Value) -> CodexAccount {
    let Some(account) = value.get("account") else {
        return CodexAccount::NotConnected;
    };
    if account.is_null() {
        return CodexAccount::NotConnected;
    }

    match account.get("type").and_then(Value::as_str) {
        Some("chatgpt") => CodexAccount::Connected {
            email: bounded_field(account.get("email")),
            plan: bounded_field(account.get("planType"))
                .as_deref()
                .map(plan_label),
        },
        Some(other) => CodexAccount::ConnectedWithOtherCredentials {
            kind: bounded_field(Some(&Value::String(other.to_owned())))
                .unwrap_or_else(|| "unknown".to_owned()),
        },
        None => CodexAccount::NotConnected,
    }
}

fn bounded_field(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?.trim();
    if text.is_empty() || text.len() > MAX_FIELD_LENGTH || text.chars().any(char::is_control) {
        return None;
    }
    Some(text.to_owned())
}

/// A readable plan name for the badge. Codex reports snake-case identifiers
/// that would look like debug output if shown as-is.
pub fn plan_label(plan: &str) -> String {
    match plan {
        "free" => "Free".to_owned(),
        "go" => "Go".to_owned(),
        "plus" => "Plus".to_owned(),
        "pro" => "Pro".to_owned(),
        "prolite" => "Pro Lite".to_owned(),
        "team" => "Team".to_owned(),
        "business" | "self_serve_business_prolite" | "self_serve_business_usage_based" => {
            "Business".to_owned()
        }
        "enterprise" | "ent26" | "enterprise_cbp_automation" | "enterprise_cbp_usage_based" => {
            "Enterprise".to_owned()
        }
        "edu" => "Education".to_owned(),
        // Including "unknown", which Codex sends when it cannot tell.
        _ => "Connected".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_fresh_codex_home_reads_as_not_connected() {
        // The exact reply observed from Codex 0.147.0 before any login.
        let reply = json!({ "account": null, "requiresOpenaiAuth": true });
        assert_eq!(parse_account(&reply), CodexAccount::NotConnected);
    }

    #[test]
    fn a_chatgpt_account_keeps_only_its_display_fields() {
        let reply = json!({
            "account": {
                "type": "chatgpt",
                "email": "person@example.com",
                "planType": "plus",
            },
            "requiresOpenaiAuth": false,
        });

        assert_eq!(
            parse_account(&reply),
            CodexAccount::Connected {
                email: Some("person@example.com".into()),
                plan: Some("Plus".into()),
            }
        );
    }

    #[test]
    fn extra_protocol_fields_are_dropped_rather_than_forwarded() {
        let reply = json!({
            "account": {
                "type": "chatgpt",
                "email": "person@example.com",
                "planType": "pro",
                "accessToken": "secret-token-value",
                "refreshToken": "another-secret",
            }
        });

        let parsed = parse_account(&reply);
        let encoded = serde_json::to_string(&parsed).unwrap();

        assert!(!encoded.contains("secret-token-value"));
        assert!(!encoded.contains("another-secret"));
        assert!(!encoded.contains("accessToken"));
        assert!(!encoded.contains("refreshToken"));
    }

    #[test]
    fn a_missing_email_is_absent_rather_than_blank() {
        let reply = json!({
            "account": { "type": "chatgpt", "email": null, "planType": "team" }
        });

        assert_eq!(
            parse_account(&reply),
            CodexAccount::Connected {
                email: None,
                plan: Some("Team".into()),
            }
        );
    }

    #[test]
    fn an_api_key_account_is_reported_honestly() {
        let reply = json!({ "account": { "type": "apiKey" } });

        assert_eq!(
            parse_account(&reply),
            CodexAccount::ConnectedWithOtherCredentials {
                kind: "apiKey".into()
            }
        );
    }

    #[test]
    fn an_unreadable_reply_falls_back_to_not_connected() {
        assert_eq!(parse_account(&json!({})), CodexAccount::NotConnected);
        assert_eq!(parse_account(&json!({ "account": {} })), CodexAccount::NotConnected);
        assert_eq!(parse_account(&json!("nonsense")), CodexAccount::NotConnected);
    }

    #[test]
    fn absurd_field_values_are_dropped() {
        let reply = json!({
            "account": {
                "type": "chatgpt",
                "email": "x".repeat(MAX_FIELD_LENGTH + 1),
                "planType": "plus",
            }
        });

        match parse_account(&reply) {
            CodexAccount::Connected { email, .. } => assert_eq!(email, None),
            other => panic!("expected a connected account, got {other:?}"),
        }
    }

    #[test]
    fn plan_labels_are_readable_and_never_debug_output() {
        assert_eq!(plan_label("plus"), "Plus");
        assert_eq!(plan_label("pro"), "Pro");
        assert_eq!(plan_label("enterprise_cbp_usage_based"), "Enterprise");
        assert_eq!(plan_label("self_serve_business_prolite"), "Business");
        // Anything unrecognized still reads as a sentence fragment.
        assert_eq!(plan_label("unknown"), "Connected");
        assert_eq!(plan_label("some_future_plan"), "Connected");
    }

    #[test]
    fn the_serialized_state_tag_matches_what_the_panel_switches_on() {
        let value = serde_json::to_value(CodexAccount::NotConnected).unwrap();
        assert_eq!(value["state"], "notConnected");

        let connected = serde_json::to_value(CodexAccount::Connected {
            email: Some("a@b.com".into()),
            plan: Some("Plus".into()),
        })
        .unwrap();
        assert_eq!(connected["state"], "connected");
        assert_eq!(connected["email"], "a@b.com");
        assert_eq!(connected["plan"], "Plus");
    }
}
