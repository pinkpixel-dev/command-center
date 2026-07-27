use serde_json::json;

use super::*;
use crate::models::RiskLevel;

fn request<'a>(content: &'a str, source: Option<&'a str>, target: TargetShell) -> ConversionRequest<'a> {
    ConversionRequest {
        content,
        title: "Set the API host",
        kind: CommandKind::Command,
        source_shell: source,
        target,
    }
}

fn converted(command: &str, risk: &str) -> serde_json::Value {
    json!({
        "converted": {
            "command": command,
            "title": "Set the API host",
            "why": "Sets the variable for this session",
            "kind": "command",
            "shell": "bash",
            "risk_suggestion": risk,
            "risk_reasons": []
        },
        "equivalence": "close",
        "differences": ["fish scopes the variable to the session, not the process tree"],
        "unsupported": [],
        "notes": ""
    })
}

#[test]
fn every_supported_shell_round_trips_through_its_identifier() {
    for shell in [
        TargetShell::Bash,
        TargetShell::Fish,
        TargetShell::Zsh,
        TargetShell::PowerShell,
    ] {
        assert_eq!(TargetShell::parse(shell.as_str()).unwrap(), shell);
    }

    assert_eq!(TargetShell::parse("PowerShell").unwrap(), TargetShell::PowerShell);
    assert_eq!(TargetShell::parse("pwsh").unwrap(), TargetShell::PowerShell);
    assert_eq!(TargetShell::parse("  ZSH ").unwrap(), TargetShell::Zsh);
    assert_eq!(TargetShell::parse("sh").unwrap(), TargetShell::Bash);
    assert_eq!(TargetShell::parse("nushell").unwrap_err().kind(), "invalid");
}

/// A saved entry's shell is free text, so an unrecognized one is not an error.
#[test]
fn an_unrecognized_saved_shell_is_simply_not_detected() {
    assert_eq!(TargetShell::detect(Some("fish")), Some(TargetShell::Fish));
    assert_eq!(TargetShell::detect(Some("nushell")), None);
    assert_eq!(TargetShell::detect(Some("")), None);
    assert_eq!(TargetShell::detect(None), None);
}

#[test]
fn conversions_are_checked_before_any_request() {
    assert!(check(&request("echo hi", Some("bash"), TargetShell::Fish)).is_ok());
    // An entry with no recorded shell can still be converted.
    assert!(check(&request("echo hi", None, TargetShell::Fish)).is_ok());

    assert_eq!(
        check(&request("   ", Some("bash"), TargetShell::Fish)).unwrap_err().kind(),
        "invalid"
    );

    let oversized = "x".repeat(MAX_COMMAND_BYTES + 1);
    assert_eq!(
        check(&request(&oversized, Some("bash"), TargetShell::Fish)).unwrap_err().kind(),
        "invalid"
    );
}

/// Converting bash to bash is a request with no answer, and saying so beats
/// paying for a model to restate the command.
#[test]
fn converting_an_entry_to_the_shell_it_already_uses_is_refused() {
    let error = check(&request("echo hi", Some("fish"), TargetShell::Fish)).unwrap_err();

    assert_eq!(error.kind(), "invalid");
    assert!(error.to_string().contains("already saved as fish"));
}

#[test]
fn the_request_states_both_shells_and_redacts_the_content() {
    let input = build_input(&request(
        "export TOKEN=super-secret-value",
        Some("bash"),
        TargetShell::Fish,
    ));

    assert!(input.contains("Source shell: bash"));
    assert!(input.contains("Target shell: fish"));
    assert!(input.contains("{{TOKEN}}"));
    assert!(!input.contains("super-secret-value"));
}

/// Letting the model assume bash silently would produce a conversion from a
/// shell the entry never named.
#[test]
fn a_missing_source_shell_is_stated_rather_than_assumed() {
    let input = build_input(&request("echo hi", None, TargetShell::PowerShell));

    assert!(input.contains("Source shell: not recorded"));
    assert!(!input.contains("Source shell: bash"));
}

#[test]
fn the_token_budget_follows_the_entry_size_within_bounds() {
    assert_eq!(output_token_budget(0), MIN_OUTPUT_TOKENS);
    assert_eq!(output_token_budget(MAX_COMMAND_BYTES), 20_000 + 16_384);
    // The size check keeps real inputs below the ceiling, so the clamp is a
    // guard rather than something a normal entry reaches.
    assert_eq!(output_token_budget(usize::MAX), MAX_OUTPUT_TOKENS);
}

#[test]
fn a_conversion_keeps_both_commands_and_its_caveats() {
    let source = "export API_HOST=example.com";
    let parsed = parse(
        &converted("set -x API_HOST example.com", "safe").to_string(),
        &request(source, Some("bash"), TargetShell::Fish),
    )
    .unwrap();

    assert_eq!(parsed.original, source);
    assert_eq!(parsed.converted.command, "set -x API_HOST example.com");
    assert_eq!(parsed.source_shell.as_deref(), Some("bash"));
    assert_eq!(parsed.target_shell, TargetShell::Fish);
    assert_eq!(parsed.equivalence, Equivalence::Close);
    assert_eq!(parsed.differences.len(), 1);
    assert!(parsed.unsupported.is_empty());
}

/// The app picked the target, so a mislabelled answer must not be able to make
/// the result look like a conversion to somewhere else.
#[test]
fn the_converted_shell_is_the_requested_target_whatever_the_model_said() {
    let parsed = parse(
        &converted("Set-Item Env:API_HOST example.com", "safe").to_string(),
        &request("export API_HOST=example.com", Some("bash"), TargetShell::PowerShell),
    )
    .unwrap();

    assert_eq!(parsed.converted.shell.as_deref(), Some("powershell"));
}

#[test]
fn the_converted_command_is_risk_checked_locally() {
    let parsed = parse(
        &converted("rm -rf /", "safe").to_string(),
        &request("del /f /s /q C:\\\\", Some("powershell"), TargetShell::Bash),
    )
    .unwrap();

    assert_eq!(parsed.converted.risk_level, RiskLevel::Destructive);
    assert!(!parsed.converted.local_reasons.is_empty());
}

/// Overstating how well a conversion held up is the failure that matters, so
/// anything unreadable lands on uncertain.
#[test]
fn an_unrecognized_equivalence_reads_as_uncertain() {
    for value in ["", "exact", "identical", "perfect"] {
        let mut body = converted("set -x API_HOST example.com", "safe");
        body["equivalence"] = json!(value);

        let parsed = parse(
            &body.to_string(),
            &request("export API_HOST=example.com", Some("bash"), TargetShell::Fish),
        )
        .unwrap();
        assert_eq!(parsed.equivalence, Equivalence::Uncertain, "{value:?}");
    }
}

/// Some commands need no rewrite at all. Reporting that as partial or
/// uncertain would invent a problem the user then has to go looking for.
#[test]
fn a_command_that_needed_no_rewrite_is_not_reported_as_partial() {
    let mut body = converted("git status", "safe");
    body["equivalence"] = json!("partial");

    let parsed = parse(
        &body.to_string(),
        &request("git status", Some("bash"), TargetShell::Fish),
    )
    .unwrap();

    assert_eq!(parsed.equivalence, Equivalence::Close);
    assert_eq!(parsed.converted.command, "git status");
}

#[test]
fn notes_are_bounded_and_deduplicated() {
    let mut body = converted("set -x API_HOST example.com", "safe");
    body["differences"] = json!((0..MAX_CONVERSION_NOTES + 5)
        .map(|index| format!("difference {index}"))
        .collect::<Vec<_>>());
    body["unsupported"] = json!(["Same note", "same note", "   "]);

    let parsed = parse(
        &body.to_string(),
        &request("export API_HOST=example.com", Some("bash"), TargetShell::Fish),
    )
    .unwrap();

    assert_eq!(parsed.differences.len(), MAX_CONVERSION_NOTES);
    assert_eq!(parsed.unsupported, vec!["Same note"]);
}

#[test]
fn a_conversion_with_no_command_is_an_error_rather_than_an_empty_result() {
    let mut body = converted("   ", "safe");
    body["converted"]["command"] = json!("   ");

    assert_eq!(
        parse(
            &body.to_string(),
            &request("echo hi", Some("bash"), TargetShell::Fish)
        )
        .unwrap_err()
        .kind(),
        "ai_malformed"
    );
    assert_eq!(
        parse("not json", &request("echo hi", Some("bash"), TargetShell::Fish))
            .unwrap_err()
            .kind(),
        "ai_malformed"
    );
}

/// The UI has to be able to say "this is not guaranteed" from the payload
/// alone, so no serialized value may read as a promise.
#[test]
fn no_serialized_equivalence_claims_the_conversion_is_exact() {
    for level in [Equivalence::Close, Equivalence::Partial, Equivalence::Uncertain] {
        let serialized = serde_json::to_string(&level).unwrap();
        for forbidden in ["exact", "identical", "equivalent"] {
            assert!(!serialized.contains(forbidden), "{serialized} reads as a guarantee");
        }
    }
}
