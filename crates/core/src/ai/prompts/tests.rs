//! Checks that hold across every prompt, whatever workflow it belongs to.

use super::*;

/// Strict Structured Outputs rejects an object that leaves a property out of
/// `required` or allows extra keys, so every object node has to satisfy both.
fn assert_strict(node: &Value) {
    if node["type"] == "object" {
        let properties = node["properties"].as_object().expect("object properties");
        let required: Vec<&str> = node["required"]
            .as_array()
            .expect("required list")
            .iter()
            .map(|value| value.as_str().expect("required name"))
            .collect();

        assert_eq!(node["additionalProperties"], false);
        assert_eq!(properties.len(), required.len());
        for name in properties.keys() {
            assert!(required.contains(&name.as_str()), "{name} is not required");
        }
        for child in properties.values() {
            assert_strict(child);
        }
    }
    if node["type"] == "array" {
        assert_strict(&node["items"]);
    }
}

fn every_task() -> Vec<&'static StructuredTask> {
    vec![
        connection_test(),
        import_extraction(),
        explanation(),
        assistant(),
        error_analysis(),
        shell_conversion(),
    ]
}

#[test]
fn every_schema_is_strict_mode_compatible() {
    for task in every_task() {
        assert_strict(&task.schema);
    }
}

/// The schema name becomes the structured-output format name on the wire, so a
/// duplicate would make two different responses indistinguishable in a log.
#[test]
fn every_task_has_its_own_name() {
    let mut seen = std::collections::HashSet::new();
    for task in every_task() {
        assert!(seen.insert(task.name), "{} is used twice", task.name);
        assert!(task.name.starts_with("command_center_"));
        assert!(!task.instructions.trim().is_empty());
    }
}

/// Every workflow that can propose a command parses the result into one
/// `RawProposal`, so all of them have to describe it identically.
#[test]
fn proposed_commands_share_one_shape_everywhere_they_appear() {
    let shared = proposal_schema();

    assert_eq!(assistant().schema["properties"]["commands"]["items"], shared);
    assert_eq!(
        error_analysis().schema["properties"]["commands"]["items"],
        shared
    );
    assert_eq!(shell_conversion().schema["properties"]["converted"], shared);
}

/// The revision marker is how a cached explanation or a bug report is traced
/// back to the wording that produced it, so no prompt may quietly skip it.
#[test]
fn every_instruction_carries_the_current_revision() {
    for task in every_task() {
        // The connection test has no user content and no wording to trace.
        if task.name == "command_center_connection_test" {
            continue;
        }
        assert!(
            task.instructions.contains(PROMPT_REVISION),
            "{} does not state its revision",
            task.name
        );
    }
}
