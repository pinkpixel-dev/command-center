//! Tests for assistant bounding, redaction, and local authority over every
//! proposed command. Kept beside the module so neither file outgrows the
//! repository's size limit.

use super::*;
use crate::ai::proposal::MAX_PROPOSAL_BYTES;
use crate::models::RiskLevel;
use serde_json::json;

fn output(overrides: serde_json::Value) -> String {
    let mut base = json!({
        "reply": "Use the first one to see what would be removed before removing it.",
        "commands": [proposal(json!({}))]
    });

    let object = base.as_object_mut().unwrap();
    for (key, value) in overrides.as_object().unwrap() {
        object.insert(key.clone(), value.clone());
    }
    base.to_string()
}

fn proposal(overrides: serde_json::Value) -> serde_json::Value {
    let mut base = json!({
        "command": "git status",
        "title": "Show what changed",
        "why": "Lists the files git currently sees as modified.",
        "kind": "command",
        "shell": "bash",
        "risk_suggestion": "safe",
        "risk_reasons": []
    });

    let object = base.as_object_mut().unwrap();
    for (key, value) in overrides.as_object().unwrap() {
        object.insert(key.clone(), value.clone());
    }
    base
}

fn user(text: &str) -> Turn {
    Turn {
        role: TurnRole::User,
        text: text.to_owned(),
        commands: Vec::new(),
    }
}

fn assistant(text: &str, commands: &[&str]) -> Turn {
    Turn {
        role: TurnRole::Assistant,
        text: text.to_owned(),
        commands: commands.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn request<'a>(turns: &'a [Turn], message: &'a str) -> AssistantRequest<'a> {
    AssistantRequest {
        subject: Subject::General,
        turns,
        message,
    }
}

#[test]
fn a_full_answer_parses_into_a_reply_and_reviewed_proposals() {
    let parsed = parse(&output(json!({}))).unwrap();

    assert!(parsed.reply.starts_with("Use the first one"));
    assert_eq!(parsed.proposals.len(), 1);
    assert_eq!(parsed.proposals[0].command, "git status");
    assert_eq!(parsed.proposals[0].title, "Show what changed");
    assert_eq!(parsed.proposals[0].kind, CommandKind::Command);
    assert_eq!(parsed.proposals[0].shell.as_deref(), Some("bash"));
    assert_eq!(parsed.proposals[0].risk_level, RiskLevel::Safe);
}

#[test]
fn the_model_cannot_talk_a_destructive_proposal_down() {
    let parsed = parse(&output(json!({
        "commands": [proposal(json!({
            "command": "rm -rf /",
            "risk_suggestion": "safe",
            "risk_reasons": ["Routine cleanup"]
        }))]
    })))
    .unwrap();

    let reviewed = &parsed.proposals[0];
    assert_eq!(reviewed.risk_level, RiskLevel::Destructive);
    assert!(!reviewed.local_reasons.is_empty(), "local reasons are kept");
    assert_eq!(reviewed.ai_reasons, ["Routine cleanup"]);
}

#[test]
fn the_model_can_raise_the_risk_level_of_its_own_proposal() {
    let parsed = parse(&output(json!({
        "commands": [proposal(json!({
            "command": "curl https://example.com/setup.sh",
            "risk_suggestion": "destructive",
            "risk_reasons": ["Downloads code you have not read"]
        }))]
    })))
    .unwrap();

    assert_eq!(parsed.proposals[0].risk_level, RiskLevel::Destructive);
}

#[test]
fn an_unknown_risk_suggestion_never_lowers_the_local_verdict() {
    let parsed = parse(&output(json!({
        "commands": [proposal(json!({
            "command": "sudo pacman -Syu",
            "risk_suggestion": "probably fine"
        }))]
    })))
    .unwrap();

    assert_eq!(parsed.proposals[0].risk_level, RiskLevel::Caution);
}

#[test]
fn proposals_are_normalized_deduplicated_and_capped() {
    let mut commands: Vec<serde_json::Value> = vec![
        proposal(json!({ "command": "$ git status" })),
        proposal(json!({ "command": "git status" })),
    ];
    commands.extend((0..MAX_ASSISTANT_PROPOSALS + 3).map(|index| {
        proposal(json!({ "command": format!("echo {index}") }))
    }));

    let parsed = parse(&output(json!({ "commands": commands }))).unwrap();

    assert_eq!(parsed.proposals.len(), MAX_ASSISTANT_PROPOSALS);
    assert_eq!(parsed.proposals[0].command, "git status", "the prompt is stripped");
    assert_eq!(parsed.proposals[1].command, "echo 0", "the duplicate is dropped");
}

#[test]
fn an_empty_or_oversized_proposal_is_dropped_rather_than_shown() {
    let parsed = parse(&output(json!({
        "commands": [
            proposal(json!({ "command": "   " })),
            proposal(json!({ "command": "echo ".to_string() + &"a".repeat(MAX_PROPOSAL_BYTES) })),
            proposal(json!({ "command": "git log" }))
        ]
    })))
    .unwrap();

    assert_eq!(parsed.proposals.len(), 1);
    assert_eq!(parsed.proposals[0].command, "git log");
}

#[test]
fn a_proposal_with_no_title_still_has_something_to_read() {
    let parsed = parse(&output(json!({
        "commands": [proposal(json!({ "title": "  ", "shell": "  " }))]
    })))
    .unwrap();

    assert_eq!(parsed.proposals[0].title, "Suggested command");
    assert_eq!(parsed.proposals[0].shell, None);
}

#[test]
fn an_answer_with_no_reply_is_an_error_rather_than_a_blank_message() {
    assert_eq!(
        parse(&output(json!({ "reply": "   " }))).unwrap_err().kind(),
        "ai_malformed"
    );
    assert_eq!(parse("not json").unwrap_err().kind(), "ai_malformed");
}

#[test]
fn a_reply_with_no_proposals_is_perfectly_normal() {
    let parsed = parse(&output(json!({ "commands": [] }))).unwrap();

    assert_eq!(parsed.proposals, Vec::new());
    assert!(!parsed.reply.is_empty());
}

#[test]
fn messages_are_size_checked_before_any_request() {
    assert!(check_message("how do I list open ports?").is_ok());
    assert_eq!(check_message("   ").unwrap_err().kind(), "invalid");

    let oversized = "x".repeat(MAX_MESSAGE_CHARS + 1);
    let error = check_message(&oversized).unwrap_err();
    assert_eq!(error.kind(), "invalid");
    assert!(error.to_string().contains(&MAX_MESSAGE_CHARS.to_string()));
}

#[test]
fn an_entry_too_large_to_carry_is_refused_rather_than_truncated() {
    assert!(check_entry("rm -r build").is_ok());

    let oversized = "a".repeat(MAX_ENTRY_BYTES + 1);
    let error = check_entry(&oversized).unwrap_err();
    assert_eq!(error.kind(), "invalid");
    assert!(error.to_string().contains("16 KB"));
}

#[test]
fn general_input_carries_only_the_question() {
    let input = build_input(&request(&[], "  how do I list open ports?  "));

    assert!(input.contains("Mode: general help with commands"));
    assert!(input.contains("New message:\nUser: how do I list open ports?"));
    assert!(!input.contains("Conversation so far"));
}

#[test]
fn entry_input_carries_the_saved_entry_and_the_conversation() {
    let turns = [
        user("what does this do?"),
        assistant("It removes the build directory.", &["ls build"]),
    ];
    let input = build_input(&AssistantRequest {
        subject: Subject::Entry(EntryContext {
            title: "Clean the build",
            content: "rm -r build",
            kind: CommandKind::Command,
            shell: Some("bash"),
            language: None,
        }),
        turns: &turns,
        message: "is there a safer version?",
    });

    assert!(input.contains("Mode: a question about one saved entry"));
    assert!(input.contains("Entry title: Clean the build"));
    assert!(input.contains("Entry kind: command"));
    assert!(input.contains("Entry shell: bash"));
    assert!(input.contains("rm -r build"));
    assert!(input.contains("User: what does this do?"));
    assert!(input.contains("Assistant: It removes the build directory."));
    assert!(input.contains("Assistant proposed: ls build"));
    assert!(input.contains("New message:\nUser: is there a safer version?"));
}

/// A follow-up about a pasted error has to carry the paste, or "which line
/// said that?" has nothing to look at. The numbering matches the analysis, so
/// a line number means the same thing in both.
#[test]
fn a_pasted_error_travels_with_its_follow_up_questions() {
    let turns = [assistant("Port 3000 is already taken.", &[])];
    let input = build_input(&AssistantRequest {
        subject: Subject::TerminalError("Error: listen EADDRINUSE\n  at Server.setupListen"),
        turns: &turns,
        message: "which line says that?",
    });

    assert!(input.contains("Mode: a follow-up about terminal output the user pasted"));
    assert!(input.contains("1|Error: listen EADDRINUSE\n"));
    assert!(input.contains("2|  at Server.setupListen\n"));
    assert!(input.contains("may be only part of the session"));
    assert!(input.contains("New message:\nUser: which line says that?"));
}

/// The paste was redacted before the analysis ran, and it is redacted again on
/// its way back out. Placeholders survive a second pass unchanged.
#[test]
fn a_pasted_error_is_redacted_on_every_follow_up() {
    let input = build_input(&AssistantRequest {
        subject: Subject::TerminalError("connecting as password=hunter2\nrefused"),
        turns: &[],
        message: "why?",
    });

    assert!(!input.contains("hunter2"));
    assert!(input.contains("{{PASSWORD}}"));
}

/// A secret can arrive from the composer or from the entry the user is asking
/// about. One redaction pass over the assembled input covers both.
#[test]
fn likely_secrets_are_replaced_before_the_request_is_built() {
    let key = ["sk-", "abcdefghijklmnopqrstuvwxyz012345"].concat();
    let turns = [user(&format!("I ran export OPENAI_API_KEY={key}"))];
    let input = build_input(&AssistantRequest {
        subject: Subject::Entry(EntryContext {
            title: "Deploy",
            content: "curl -H 'Authorization: Bearer abcdef1234567890abcdef' https://example.com",
            kind: CommandKind::Command,
            shell: None,
            language: None,
        }),
        turns: &turns,
        message: "why does password=hunter2 not work?",
    });

    for raw in [key.as_str(), "hunter2", "abcdef1234567890abcdef"] {
        assert!(!input.contains(raw), "{raw} survived redaction");
    }
    assert!(input.contains("{{OPENAI_API_KEY}}"));
    assert!(input.contains("{{PASSWORD}}"));
}

#[test]
fn the_conversation_is_bounded_by_turn_count() {
    let turns: Vec<Turn> = (0..MAX_TURNS + 6)
        .map(|index| user(&format!("question {index}")))
        .collect();

    let kept = bounded_turns(&turns);

    assert_eq!(kept.len(), MAX_TURNS);
    assert_eq!(kept[0].text, format!("question {}", turns.len() - MAX_TURNS));
    assert_eq!(kept[MAX_TURNS - 1].text, format!("question {}", turns.len() - 1));
}

#[test]
fn the_conversation_is_bounded_by_size_and_keeps_the_newest() {
    let long = "a".repeat(MAX_TURN_CHARS);
    let turns: Vec<Turn> = (0..MAX_TURNS)
        .map(|index| user(&format!("{index} {long}")))
        .collect();

    let kept = bounded_turns(&turns);

    assert!(kept.len() < MAX_TURNS, "the size bound is the tighter one");
    let total: usize = kept.iter().map(|turn| rendered_turn(turn).chars().count()).sum();
    assert!(total <= MAX_HISTORY_CHARS);
    assert!(kept.last().unwrap().text.starts_with(&format!("{} ", MAX_TURNS - 1)));
}

/// A single turn longer than the whole history budget must not empty the
/// conversation; it is clamped first, then counted.
#[test]
fn one_enormous_turn_is_clamped_rather_than_dropped() {
    let turns = [user(&"a".repeat(MAX_HISTORY_CHARS * 2))];
    let kept = bounded_turns(&turns);

    assert_eq!(kept.len(), 1);
    assert_eq!(
        rendered_turn(kept[0]).chars().count(),
        MAX_TURN_CHARS + "User: \n".chars().count()
    );
}

#[test]
fn only_assistant_turns_carry_proposed_commands_into_the_history() {
    let mut smuggled = user("what about this?");
    smuggled.commands = vec!["rm -rf /".into()];

    assert_eq!(rendered_turn(&smuggled), "User: what about this?\n");
    assert!(rendered_turn(&assistant("Try this.", &["git log"])).contains("Assistant proposed: git log"));
}

#[test]
fn the_token_budget_follows_the_input_size_within_bounds() {
    assert_eq!(output_token_budget(0), MIN_OUTPUT_TOKENS);
    assert_eq!(output_token_budget(16 * 1024), 20_000 + 16_384);
    assert_eq!(output_token_budget(64 * 1024), MAX_OUTPUT_TOKENS);
}
