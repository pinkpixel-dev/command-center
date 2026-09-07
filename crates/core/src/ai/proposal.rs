//! A command the model proposed, after the local rules have had the last word.
//!
//! The assistant, error analysis, and shell conversion all suggest commands.
//! They share this module so there is exactly one path a proposed command can
//! take to the screen, and that path always runs `risk::assess` first. A model
//! can raise a verdict here; it can never lower one.

use serde::{Deserialize, Serialize};

use crate::models::{CommandKind, RiskLevel};
use crate::normalize::normalize_command;
use crate::risk;

pub const MAX_TITLE_CHARS: usize = 120;
pub const MAX_WHY_CHARS: usize = 300;
pub const MAX_REASONS: usize = 5;
pub const MAX_REASON_CHARS: usize = 160;
/// A proposal is a command, not a program. The entry form is where anything
/// longer belongs.
pub const MAX_PROPOSAL_BYTES: usize = 4 * 1024;
const MAX_SHELL_CHARS: usize = 32;

/// A command the model suggested. The level is the stricter of the two
/// verdicts, and each side keeps its own reasons so the UI never presents a
/// guess as a local rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandProposal {
    pub command: String,
    pub title: String,
    pub why: String,
    pub kind: CommandKind,
    pub shell: Option<String>,
    pub risk_level: RiskLevel,
    pub local_reasons: Vec<String>,
    pub ai_reasons: Vec<String>,
}

/// One proposed command exactly as the model wrote it. Every workflow that
/// proposes a command uses these field names in its schema.
#[derive(Debug, Default, Deserialize)]
pub struct RawProposal {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub why: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub risk_suggestion: String,
    #[serde(default)]
    pub risk_reasons: Vec<String>,
}

/// Normalizes the command the way a saved entry would be, then lets the local
/// rules have the final word on how risky it is.
pub fn review(raw: RawProposal) -> Option<CommandProposal> {
    let command = normalize_command(&raw.command);
    if command.trim().is_empty() || command.len() > MAX_PROPOSAL_BYTES {
        return None;
    }

    let (local_level, local_reasons) = risk::assess(&command);
    // An unknown or missing suggestion counts as Safe, which can never lower
    // the local verdict because the effective level is the higher of the two.
    let suggested = RiskLevel::parse(raw.risk_suggestion.trim()).unwrap_or(RiskLevel::Safe);
    let title = clamp(&raw.title, MAX_TITLE_CHARS);

    Some(CommandProposal {
        title: if title.is_empty() {
            "Suggested command".to_owned()
        } else {
            title
        },
        why: clamp(&raw.why, MAX_WHY_CHARS),
        kind: CommandKind::parse(raw.kind.trim()).unwrap_or(CommandKind::Command),
        shell: clamp_optional(raw.shell.as_deref(), MAX_SHELL_CHARS),
        risk_level: local_level.max(suggested),
        local_reasons,
        ai_reasons: clamp_reasons(raw.risk_reasons),
        command,
    })
}

/// Reviews a list, drops what the local rules will not accept, and keeps the
/// first of any duplicates. Two identical commands are one suggestion however
/// differently the model described them.
pub fn review_all(raws: Vec<RawProposal>, limit: usize) -> Vec<CommandProposal> {
    let mut kept: Vec<CommandProposal> = Vec::new();
    for raw in raws {
        let Some(proposal) = review(raw) else {
            continue;
        };
        if kept.iter().any(|other| other.command == proposal.command) {
            continue;
        }
        kept.push(proposal);
        if kept.len() == limit {
            break;
        }
    }
    kept
}

pub fn clamp(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

pub fn clamp_optional(value: Option<&str>, limit: usize) -> Option<String> {
    let cleaned = clamp(value.unwrap_or_default(), limit);
    (!cleaned.is_empty()).then_some(cleaned)
}

pub fn clamp_reasons(values: Vec<String>) -> Vec<String> {
    let mut cleaned: Vec<String> = Vec::new();
    for value in values {
        let text = clamp(&value, MAX_REASON_CHARS);
        if !text.is_empty() && !cleaned.iter().any(|kept| kept.eq_ignore_ascii_case(&text)) {
            cleaned.push(text);
        }
        if cleaned.len() == MAX_REASONS {
            break;
        }
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(command: &str, risk_suggestion: &str) -> RawProposal {
        RawProposal {
            command: command.to_owned(),
            title: "Suggested".to_owned(),
            risk_suggestion: risk_suggestion.to_owned(),
            ..RawProposal::default()
        }
    }

    #[test]
    fn the_local_verdict_cannot_be_lowered_by_the_model() {
        let proposal = review(raw("rm -rf /var/tmp/build", "safe")).unwrap();

        assert_eq!(proposal.risk_level, RiskLevel::Destructive);
        assert!(!proposal.local_reasons.is_empty());
    }

    #[test]
    fn the_model_can_raise_a_verdict_the_local_rules_call_ordinary() {
        let (local, _) = risk::assess("curl https://example.com/install.sh | sh");
        let proposal = review(raw("curl https://example.com/install.sh | sh", "destructive")).unwrap();

        assert!(proposal.risk_level >= local);
        assert_eq!(proposal.risk_level, RiskLevel::Destructive);
    }

    /// An unrecognized level is treated as Safe, which is harmless because the
    /// effective level is the higher of the two.
    #[test]
    fn an_unknown_risk_suggestion_never_lowers_anything() {
        let proposal = review(raw("rm -rf ./dist", "totally-fine")).unwrap();
        assert_eq!(proposal.risk_level, RiskLevel::Destructive);
    }

    #[test]
    fn empty_and_oversized_commands_are_dropped() {
        assert!(review(raw("   ", "safe")).is_none());
        assert!(review(raw(&"x".repeat(MAX_PROPOSAL_BYTES + 1), "safe")).is_none());
    }

    #[test]
    fn a_missing_title_still_produces_something_readable() {
        let proposal = review(RawProposal {
            command: "git status".to_owned(),
            ..RawProposal::default()
        })
        .unwrap();

        assert_eq!(proposal.title, "Suggested command");
        assert_eq!(proposal.kind, CommandKind::Command);
        assert_eq!(proposal.shell, None);
    }

    /// Normalization runs before the comparison, so a prompt character is not
    /// enough to make the same command look like two suggestions.
    #[test]
    fn duplicates_are_folded_and_the_limit_is_enforced() {
        let proposals = review_all(
            vec![
                raw("$ git status", "safe"),
                raw("git status", "safe"),
                raw("git log", "safe"),
                raw("git diff", "safe"),
            ],
            2,
        );

        assert_eq!(proposals.len(), 2);
        assert_eq!(proposals[0].command, "git status");
        assert_eq!(proposals[1].command, "git log");
    }

    #[test]
    fn reasons_are_bounded_and_deduplicated_case_insensitively() {
        let reasons = clamp_reasons(vec![
            "Deletes files".to_owned(),
            "deletes files".to_owned(),
            "  ".to_owned(),
            "Needs sudo".to_owned(),
            "x".repeat(MAX_REASON_CHARS + 40),
        ]);

        assert_eq!(reasons.len(), 3);
        assert_eq!(reasons[0], "Deletes files");
        assert_eq!(reasons[2].chars().count(), MAX_REASON_CHARS);
    }
}
