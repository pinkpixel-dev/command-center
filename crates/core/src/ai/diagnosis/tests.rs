use serde_json::json;

use super::*;
use crate::models::RiskLevel;

fn analysis(value: serde_json::Value) -> AppResult<ErrorAnalysis> {
    parse(&value.to_string())
}

fn minimal() -> serde_json::Value {
    json!({
        "summary": "The build could not reach the package registry.",
        "cause": "The registry host did not resolve, which usually means DNS or a proxy.",
        "confidence": "medium",
        "relevant_lines": [],
        "checks": [],
        "commands": [],
        "uncertainty": ""
    })
}

#[test]
fn pastes_are_size_checked_before_any_request() {
    assert!(check_output_size("error: nope").is_ok());
    assert_eq!(check_output_size("   ").unwrap_err().kind(), "invalid");

    let oversized = "x".repeat(MAX_OUTPUT_BYTES + 1);
    let error = check_output_size(&oversized).unwrap_err();
    assert_eq!(error.kind(), "invalid");
    assert!(error.to_string().contains("32 KB"));
}

#[test]
fn the_token_budget_follows_the_paste_size_within_bounds() {
    assert_eq!(output_token_budget(0), MIN_OUTPUT_TOKENS);
    assert_eq!(output_token_budget(16 * 1024), 20_000 + 16_384);
    assert_eq!(output_token_budget(MAX_OUTPUT_BYTES), MAX_OUTPUT_TOKENS);
}

/// The input carries only the redacted text it was handed. The disclosure step
/// redacts once and this sends that exact string, so the two cannot drift.
#[test]
fn the_request_carries_numbered_lines_and_only_the_redacted_text() {
    let input = build_input("Error: connect failed\npassword={{PASSWORD}}");

    assert!(input.contains("1|Error: connect failed\n"));
    assert!(input.contains("2|password={{PASSWORD}}\n"));
    assert!(!input.contains("hunter2"));
}

#[test]
fn numbering_is_one_based_and_survives_a_trailing_newline() {
    assert_eq!(numbered_lines("first\nsecond\n"), "1|first\n2|second\n");
    assert_eq!(numbered_lines("only"), "1|only\n");
    assert_eq!(numbered_lines(""), "");
}

#[test]
fn an_analysis_is_bounded_and_readable() {
    let parsed = analysis(json!({
        "summary": "  Port 3000 is already in use.  ",
        "cause": "Another process is bound to the port.",
        "confidence": "High",
        "relevant_lines": [
            { "line": 3, "quote": "  EADDRINUSE :::3000  ", "why": "The bind failed here" },
            { "line": null, "quote": "   ", "why": "dropped, no quote" }
        ],
        "checks": [
            { "check": "Find what owns port 3000", "why": "Tells you what to stop" },
            { "check": "  ", "why": "dropped" }
        ],
        "commands": [],
        "uncertainty": "The command that produced this was not included."
    }))
    .unwrap();

    assert_eq!(parsed.summary, "Port 3000 is already in use.");
    assert_eq!(parsed.confidence, Confidence::High);
    assert_eq!(parsed.relevant_lines.len(), 1, "an empty quote is dropped");
    assert_eq!(parsed.relevant_lines[0].line, Some(3));
    assert_eq!(parsed.relevant_lines[0].quote, "EADDRINUSE :::3000");
    assert_eq!(parsed.checks.len(), 1, "an empty check is dropped");
    assert!(parsed.uncertainty.contains("not included"));
}

/// Being unsure is a real answer, and an unreadable confidence value has to
/// land on the humble side rather than the confident one.
#[test]
fn an_unrecognized_confidence_reads_as_low() {
    for value in ["", "certain", "very high", "unknown"] {
        let parsed = analysis(json!({
            "summary": "Something failed.",
            "cause": "",
            "confidence": value,
            "relevant_lines": [],
            "checks": [],
            "commands": [],
            "uncertainty": ""
        }))
        .unwrap();
        assert_eq!(parsed.confidence, Confidence::Low, "{value:?}");
    }
}

/// The whole point of routing proposals through `ai::proposal`: a suggested
/// next step is still a command, and the local rules judge it.
#[test]
fn a_suggested_command_is_risk_checked_locally() {
    let parsed = analysis(json!({
        "summary": "Stale build output.",
        "cause": "The previous build left files behind.",
        "confidence": "medium",
        "relevant_lines": [],
        "checks": [],
        "commands": [{
            "command": "rm -rf /",
            "title": "Clear the build",
            "why": "Removes the stale output",
            "kind": "command",
            "shell": "bash",
            "risk_suggestion": "safe",
            "risk_reasons": []
        }],
        "uncertainty": ""
    }))
    .unwrap();

    assert_eq!(parsed.proposals.len(), 1);
    assert_eq!(parsed.proposals[0].risk_level, RiskLevel::Destructive);
    assert!(!parsed.proposals[0].local_reasons.is_empty());
    assert!(parsed.proposals[0].ai_reasons.is_empty());
}

/// A diagnostic paste can easily produce more suggestions than anyone wants to
/// read, so the ceiling holds whatever the model returns.
#[test]
fn the_proposal_ceiling_holds() {
    let commands: Vec<serde_json::Value> = (0..MAX_ANALYSIS_PROPOSALS + 4)
        .map(|index| {
            json!({
                "command": format!("echo check-{index}"),
                "title": "Check",
                "why": "",
                "kind": "command",
                "shell": null,
                "risk_suggestion": "safe",
                "risk_reasons": []
            })
        })
        .collect();

    let mut value = minimal();
    value["commands"] = json!(commands);

    assert_eq!(parse(&value.to_string()).unwrap().proposals.len(), MAX_ANALYSIS_PROPOSALS);
}

#[test]
fn list_ceilings_hold_for_lines_and_checks() {
    let many: Vec<serde_json::Value> = (0..MAX_ANALYSIS_ITEMS + 6)
        .map(|index| json!({ "line": index + 1, "quote": format!("line {index}"), "why": "noted" }))
        .collect();
    let checks: Vec<serde_json::Value> = (0..MAX_ANALYSIS_ITEMS + 6)
        .map(|index| json!({ "check": format!("check {index}"), "why": "tells you something" }))
        .collect();

    let mut value = minimal();
    value["relevant_lines"] = json!(many);
    value["checks"] = json!(checks);

    let parsed = parse(&value.to_string()).unwrap();
    assert_eq!(parsed.relevant_lines.len(), MAX_ANALYSIS_ITEMS);
    assert_eq!(parsed.checks.len(), MAX_ANALYSIS_ITEMS);
}

#[test]
fn malformed_output_is_an_error_rather_than_a_blank_analysis() {
    assert_eq!(parse("not json").unwrap_err().kind(), "ai_malformed");

    let mut empty = minimal();
    empty["summary"] = json!("   ");
    assert_eq!(parse(&empty.to_string()).unwrap_err().kind(), "ai_malformed");
}

/// Nothing about an analysis reaches SQLite, so the only thing to prove about
/// the result is that a redacted paste stays redacted through serialization.
#[test]
fn a_serialized_analysis_carries_no_secret_from_the_paste() {
    let parsed = analysis(json!({
        "summary": "Authentication failed against the registry.",
        "cause": "The stored token was rejected.",
        "confidence": "high",
        "relevant_lines": [
            { "line": 2, "quote": "Authorization: Bearer {{BEARER_TOKEN}}", "why": "The rejected header" }
        ],
        "checks": [],
        "commands": [],
        "uncertainty": ""
    }))
    .unwrap();

    let serialized = serde_json::to_string(&parsed).unwrap();
    assert!(serialized.contains("{{BEARER_TOKEN}}"));
    assert!(!serialized.to_lowercase().contains("eyjhbgci"));
}
