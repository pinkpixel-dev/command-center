//! Tests for explanation parsing and local authority. Kept beside the module
//! rather than inside it so neither file outgrows the repository's size limit.

use super::*;
use serde_json::json;

fn output(overrides: serde_json::Value) -> String {
    let mut base = json!({
        "summary": "Deletes the build directory and everything under it.",
        "flags": [{ "flag": "-r", "meaning": "Recurse into directories" }],
        "pipeline": [],
        "side_effects": ["Removes files from disk"],
        "safety": { "level": "caution", "reasons": ["Deletes files"] },
        "preview_command": null,
        "assumptions": [],
        "caveats": []
    });

    let object = base.as_object_mut().unwrap();
    for (key, value) in overrides.as_object().unwrap() {
        object.insert(key.clone(), value.clone());
    }
    base.to_string()
}

fn request<'a>(content: &'a str) -> ExplanationRequest<'a> {
    ExplanationRequest {
        content,
        title: "Clean the build",
        description: "",
        kind: CommandKind::Command,
        shell: Some("bash"),
        language: None,
    }
}

#[test]
fn a_full_explanation_parses_into_both_views() {
    let parsed = parse(
        &output(json!({
            "pipeline": [{ "stage": "docker ps -q", "purpose": "Lists running container ids" }],
            "assumptions": ["Run from the repository root"],
            "caveats": ["GNU and BSD builds differ here"]
        })),
        "rm -r build",
    )
    .unwrap();

    assert!(parsed.summary.starts_with("Deletes the build directory"));
    assert_eq!(parsed.flags[0].flag, "-r");
    assert_eq!(parsed.pipeline[0].stage, "docker ps -q");
    assert_eq!(parsed.side_effects, ["Removes files from disk"]);
    assert_eq!(parsed.assumptions, ["Run from the repository root"]);
    assert_eq!(parsed.caveats, ["GNU and BSD builds differ here"]);
}

#[test]
fn the_model_cannot_talk_a_destructive_command_down() {
    let parsed = parse(
        &output(json!({ "safety": { "level": "safe", "reasons": ["Routine cleanup"] } })),
        "rm -rf /",
    )
    .unwrap();

    assert_eq!(parsed.safety.level, RiskLevel::Destructive);
    assert!(!parsed.safety.local_reasons.is_empty(), "local reasons are kept");
    assert_eq!(parsed.safety.ai_reasons, ["Routine cleanup"]);
}

#[test]
fn the_model_can_raise_the_risk_level() {
    let parsed = parse(
        &output(json!({ "safety": { "level": "destructive", "reasons": ["Runs remote code"] } })),
        "curl https://example.com/setup | less",
    )
    .unwrap();

    assert_eq!(parsed.safety.level, RiskLevel::Destructive);
}

#[test]
fn an_unknown_safety_level_never_lowers_the_local_verdict() {
    let parsed = parse(
        &output(json!({ "safety": { "level": "probably fine", "reasons": [] } })),
        "sudo pacman -Syu",
    )
    .unwrap();

    assert_eq!(parsed.safety.level, RiskLevel::Caution);
}

#[test]
fn a_preview_command_is_kept_with_its_own_local_risk_result() {
    let parsed = parse(
        &output(json!({ "preview_command": "find . -name '*.log' -mtime +7 -print" })),
        "find . -name '*.log' -mtime +7 -delete",
    )
    .unwrap();

    let preview = parsed.preview_command.expect("the print form is a real preview");
    assert!(preview.command.contains("-print"));
    assert_eq!(preview.risk_level, RiskLevel::Safe);
}

#[test]
fn a_preview_the_local_rules_call_destructive_is_dropped() {
    let parsed = parse(
        &output(json!({ "preview_command": "rm -rf ./build --dry-run" })),
        "rm -rf ./build",
    )
    .unwrap();

    assert_eq!(parsed.preview_command, None);
}

#[test]
fn a_preview_that_only_restates_the_original_is_dropped() {
    let parsed = parse(
        &output(json!({ "preview_command": "$ rm -r build  " })),
        "rm -r build",
    )
    .unwrap();

    assert_eq!(parsed.preview_command, None);
}

#[test]
fn a_preview_that_carries_a_warning_keeps_its_reasons() {
    let parsed = parse(
        &output(json!({ "preview_command": "sudo systemctl stop nginx" })),
        "sudo systemctl disable --now nginx",
    )
    .unwrap();

    let preview = parsed.preview_command.unwrap();
    assert_eq!(preview.risk_level, RiskLevel::Caution);
    assert!(!preview.risk_reasons.is_empty());
}

#[test]
fn lists_are_trimmed_deduplicated_and_bounded() {
    let many: Vec<String> = (0..MAX_EXPLANATION_ITEMS + 8)
        .map(|index| format!("  effect {index}  "))
        .collect();
    let parsed = parse(
        &output(json!({
            "side_effects": many,
            "flags": [
                { "flag": "  -r  ", "meaning": "  Recurse  " },
                { "flag": "", "meaning": "no flag" },
                { "flag": "-f", "meaning": "" }
            ],
            "assumptions": ["Same note", "same note"]
        })),
        "rm -r build",
    )
    .unwrap();

    assert_eq!(parsed.side_effects.len(), MAX_EXPLANATION_ITEMS);
    assert_eq!(parsed.side_effects[0], "effect 0");
    assert_eq!(parsed.flags.len(), 1, "half-written flags are dropped");
    assert_eq!(parsed.flags[0].flag, "-r");
    assert_eq!(parsed.assumptions, ["Same note"]);
}

#[test]
fn an_explanation_without_a_summary_is_an_error_rather_than_an_empty_panel() {
    assert_eq!(
        parse(&output(json!({ "summary": "   " })), "ls").unwrap_err().kind(),
        "ai_malformed"
    );
    assert_eq!(parse("not json", "ls").unwrap_err().kind(), "ai_malformed");
}

#[test]
fn search_text_carries_the_prose_and_leaves_commands_out() {
    let parsed = parse(
        &output(json!({
            "preview_command": "find . -name '*.log' -print",
            "caveats": ["Behaviour differs on macOS"]
        })),
        "find . -name '*.log' -delete",
    )
    .unwrap();

    let text = parsed.search_text();
    assert!(text.contains("Deletes the build directory"));
    assert!(text.contains("Recurse into directories"));
    assert!(text.contains("Behaviour differs on macOS"));
    assert!(!text.contains("-print"), "proposed commands stay out of search");
}

#[test]
fn the_request_carries_the_saved_context_with_likely_secrets_replaced() {
    let content = "curl -H 'Authorization: Bearer abcdefghijklmnopqrst' https://example.com";
    let mut outgoing = request(content);
    outgoing.description = "password=hunter2";

    let input = build_input(&outgoing);

    assert!(input.contains("Kind: command"));
    assert!(input.contains("Title: Clean the build"));
    assert!(input.contains("Shell: bash"));
    assert!(input.contains("{{BEARER_TOKEN}}"));
    assert!(input.contains("password={{PASSWORD}}"));
    assert!(!input.contains("abcdefghijklmnopqrst"));
    assert!(!input.contains("hunter2"));
}

#[test]
fn empty_context_fields_are_left_out_of_the_request() {
    let outgoing = ExplanationRequest {
        content: "ls -la",
        title: "  ",
        description: "",
        kind: CommandKind::Snippet,
        shell: Some("   "),
        language: None,
    };

    let input = build_input(&outgoing);

    assert!(!input.contains("Title:"));
    assert!(!input.contains("Shell:"));
    assert!(!input.contains("Language:"));
    assert!(input.ends_with("ls -la\n"));
}

#[test]
fn entries_are_size_checked_before_any_request() {
    assert!(check_size("ls -la").is_ok());
    assert_eq!(check_size("   ").unwrap_err().kind(), "invalid");

    let oversized = "x".repeat(MAX_COMMAND_BYTES + 1);
    let error = check_size(&oversized).unwrap_err();
    assert_eq!(error.kind(), "invalid");
    assert!(error.to_string().contains("16 KB"));
}

#[test]
fn the_token_budget_has_a_reasoning_floor() {
    assert_eq!(output_token_budget(0), MIN_OUTPUT_TOKENS);
    assert_eq!(output_token_budget(200), MIN_OUTPUT_TOKENS);
    assert_eq!(output_token_budget(MAX_COMMAND_BYTES), 20_000 + 16_384);
}

#[test]
fn a_stored_explanation_round_trips_through_the_cache_shape() {
    let parsed = parse(&output(json!({})), "rm -r build").unwrap();
    let encoded = serde_json::to_string(&parsed).unwrap();

    assert!(encoded.contains("\"sideEffects\""));
    assert!(encoded.contains("\"localReasons\""));
    assert_eq!(
        serde_json::from_str::<Explanation>(&encoded).unwrap(),
        parsed
    );
}
