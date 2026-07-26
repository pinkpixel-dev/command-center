//! Versioned instructions and strict response schemas. Every request the app
//! makes is defined here, so prompt changes are reviewable in one place.

use std::sync::OnceLock;

use serde_json::{json, Value};

/// Bumped whenever the wording or a schema below changes, so cached results and
/// bug reports can be traced back to a known prompt.
pub const PROMPT_REVISION: &str = "2026-07-26.1";

/// Ceiling the extraction prompt states and the parser enforces.
pub const MAX_IMPORT_ITEMS: usize = 150;

pub struct StructuredTask {
    pub name: &'static str,
    pub instructions: &'static str,
    pub schema: Value,
}

pub fn connection_test() -> &'static StructuredTask {
    static TASK: OnceLock<StructuredTask> = OnceLock::new();
    TASK.get_or_init(|| StructuredTask {
        name: "command_center_connection_test",
        instructions: "Return the requested connection readiness object.",
        schema: json!({
            "type": "object",
            "properties": {
                "ready": { "type": "boolean", "enum": [true] }
            },
            "required": ["ready"],
            "additionalProperties": false
        }),
    })
}

pub const CONNECTION_TEST_INPUT: &str = "Confirm that this Responses API request succeeded.";

pub fn import_extraction() -> &'static StructuredTask {
    static TASK: OnceLock<StructuredTask> = OnceLock::new();
    TASK.get_or_init(|| StructuredTask {
        name: "command_center_import_extraction",
        instructions: IMPORT_INSTRUCTIONS,
        schema: import_schema(),
    })
}

const IMPORT_INSTRUCTIONS: &str = concat!(
    "Command Center import extraction, revision 2026-07-26.1.\n\n",
    "You read one technical document and list the commands, scripts, and snippets ",
    "a person would want to save in a personal command library.\n\n",
    "Rules:\n",
    "- Every line of the document is prefixed with its line number and a pipe character. Use those ",
    "numbers for source_line, and never copy a prefix into content.\n",
    "- Only report content that appears in the document. Never invent a command, a flag, or a value.\n",
    "- Copy each item exactly as written, including any {{PLACEHOLDER}} text. A placeholder means a ",
    "secret was removed on the user's machine before the document was sent; leave it in place.\n",
    "- Keep the steps of one procedure together when they are meant to run in order, and separate ",
    "unrelated commands that happen to share a code block.\n",
    "- Drop shell prompts such as $, #, or PS>, and drop pasted result text that follows a command.\n",
    "- Set looks_like_output to true when an item is terminal output rather than something to run.\n",
    "- title: short, specific, and imperative.\n",
    "- description: one sentence about what the item does or when to use it, or an empty string when ",
    "the document does not say.\n",
    "- tags: at most five lowercase words or hyphenated words.\n",
    "- kind: command for a single command, sequence for ordered steps, script for a shebang or shell ",
    "control flow, snippet for configuration or code, reference for something read rather than run.\n",
    "- shell and language: only when the document makes them clear, otherwise null.\n",
    "- risk_suggestion: safe, caution, or destructive, judged by what the item does on a normal ",
    "machine. This is advice; the application applies its own rules as well.\n",
    "- risk_reasons: short phrases, empty when the item is ordinary.\n",
    "- uncertainty: what context is missing, or an empty string when nothing is missing.\n",
    "- source_line: the 1-based line where the item starts, or null when that is unclear.\n",
    "- Return at most 150 items, keeping the most useful ones.\n",
    "- Return an empty list when the document holds nothing worth saving.",
);

fn import_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "content": { "type": "string" },
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "kind": {
                            "type": "string",
                            "enum": ["command", "sequence", "script", "snippet", "reference"]
                        },
                        "shell": { "type": ["string", "null"] },
                        "language": { "type": ["string", "null"] },
                        "tags": { "type": "array", "items": { "type": "string" } },
                        "source_line": { "type": ["integer", "null"] },
                        "looks_like_output": { "type": "boolean" },
                        "risk_suggestion": {
                            "type": "string",
                            "enum": ["safe", "caution", "destructive"]
                        },
                        "risk_reasons": { "type": "array", "items": { "type": "string" } },
                        "uncertainty": { "type": "string" }
                    },
                    "required": [
                        "content",
                        "title",
                        "description",
                        "kind",
                        "shell",
                        "language",
                        "tags",
                        "source_line",
                        "looks_like_output",
                        "risk_suggestion",
                        "risk_reasons",
                        "uncertainty"
                    ],
                    "additionalProperties": false
                }
            }
        },
        "required": ["items"],
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn every_schema_is_strict_mode_compatible() {
        assert_strict(&connection_test().schema);
        assert_strict(&import_extraction().schema);
    }

    #[test]
    fn import_instructions_state_the_limits_the_parser_enforces() {
        assert!(IMPORT_INSTRUCTIONS.contains(&MAX_IMPORT_ITEMS.to_string()));
        assert!(IMPORT_INSTRUCTIONS.contains("{{PLACEHOLDER}}"));
        assert!(IMPORT_INSTRUCTIONS.contains(PROMPT_REVISION));
    }
}
