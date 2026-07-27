//! Versioned instructions and strict response schemas. Every request the app
//! makes is defined here, so prompt changes are reviewable in one place.

use std::sync::OnceLock;

use serde_json::{json, Value};

/// Bumped whenever the wording or a schema below changes, so cached results and
/// bug reports can be traced back to a known prompt.
pub const PROMPT_REVISION: &str = "2026-07-26.3";

/// Ceiling the extraction prompt states and the parser enforces.
pub const MAX_IMPORT_ITEMS: usize = 150;

/// Ceiling the explanation prompt states and the parser enforces, per list.
pub const MAX_EXPLANATION_ITEMS: usize = 12;

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
    "Command Center import extraction, revision 2026-07-26.3.\n\n",
    "You read one technical document and list the commands, scripts, and snippets ",
    "a person would want to save in a personal command library.\n\n",
    "Each item is saved as its own card and copied to a terminal later, so content has to be the ",
    "command as the user would type it, not the document's presentation of it.\n\n",
    "Rules:\n",
    "- Every line of the document is prefixed with its line number and a pipe character. Use those ",
    "numbers for source_line, and never copy a prefix into content.\n",
    "- Only report commands that appear in the document. Never invent a command, a flag, or a value.\n",
    "- Default to one command per item. Group several commands into one item only when the document ",
    "presents them as an ordered procedure where a later command depends on an earlier one. A list ",
    "of independent commands under a shared heading is not a procedure, even when the document ",
    "prints them as one block.\n",
    "- When a command line ends with a comment that explains what it does, use that comment as the ",
    "title and leave the comment out of content. `sudo fuser -k 3000/tcp   # Kill process on port` ",
    "becomes content `sudo fuser -k 3000/tcp` with title `Kill process on port`.\n",
    "- Clean up the document's formatting. Remove alignment padding, tabs, and trailing spaces used ",
    "to line up columns. Remove Markdown escaping, so `\\-ltnp` becomes `-ltnp`, `\\#` becomes `#`, ",
    "and `\\*` becomes `*`. Remove list markers, backticks, and fences. Never add escape sequences ",
    "or backslashes of your own.\n",
    "- Keep any {{PLACEHOLDER}} text exactly as it is. A placeholder means a secret was removed on ",
    "the user's machine before the document was sent.\n",
    "- Drop shell prompts such as $, #, or PS>, and drop pasted result text that follows a command.\n",
    "- Set looks_like_output to true when an item is terminal output rather than something to run.\n",
    "- title: short, specific, and written as ordinary words. Not a slug, not kebab-case.\n",
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

pub fn explanation() -> &'static StructuredTask {
    static TASK: OnceLock<StructuredTask> = OnceLock::new();
    TASK.get_or_init(|| StructuredTask {
        name: "command_center_explanation",
        instructions: EXPLANATION_INSTRUCTIONS,
        schema: explanation_schema(),
    })
}

const EXPLANATION_INSTRUCTIONS: &str = concat!(
    "Command Center command explanation, revision 2026-07-26.3.\n\n",
    "You explain one saved command, script, or snippet to the person who saved it. They can read ",
    "a terminal; they want to know what this particular thing does before they run it.\n\n",
    "One response fills two views. The summary is shown on its own as the quick answer, and the ",
    "rest is shown when the reader asks for detail, so the summary has to stand alone and the ",
    "detail must not repeat it word for word.\n\n",
    "Rules:\n",
    "- summary: two or three sentences in plain words. What it does, and what it touches.\n",
    "- flags: one entry per flag, option, or argument that changes behaviour. flag is the option ",
    "as written, such as `-r` or `--force`. meaning is a short phrase. Skip a subcommand that ",
    "carries no options of its own, and skip anything the command does not actually use.\n",
    "- pipeline: one entry per stage when the content pipes, chains, or runs steps in order. ",
    "stage is the stage as written, purpose is what it contributes. Leave the list empty for a ",
    "single command with no pipeline.\n",
    "- side_effects: what changes on the machine after this runs. Files written or deleted, ",
    "services touched, packages installed, network requests made. Empty when it only reads.\n",
    "- safety.level: safe, caution, or destructive, judged by what it does on a normal machine. ",
    "This is advice. Command Center runs its own rules and keeps the stricter verdict.\n",
    "- safety.reasons: short phrases explaining that level. Empty when the item is ordinary.\n",
    "- preview_command: a genuinely safer way to see what this would do before doing it, such as ",
    "a dry-run flag, a list-only form, or the same search without the delete. It must be a real ",
    "form of this command that the tool actually supports. Return null when there is no honest ",
    "preview. Never return the original command, and never invent a flag.\n",
    "- assumptions: what you had to assume, such as the shell, the platform, or what a path or ",
    "placeholder stands for. Empty when nothing was assumed.\n",
    "- caveats: behaviour that depends on the tool version, the platform, or the shell, and ",
    "anything that would surprise the reader. Empty when there is nothing to warn about.\n",
    "- {{PLACEHOLDER}} text is a value the user fills in before running the command. Explain what ",
    "belongs there. Never guess a concrete value for it.\n",
    "- Explain only what is in front of you. Never invent flags, files, or behaviour, and say so ",
    "in assumptions when the content is ambiguous.\n",
    "- At most 12 entries in any list, keeping the ones that matter most.",
);

fn explanation_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": { "type": "string" },
            "flags": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "flag": { "type": "string" },
                        "meaning": { "type": "string" }
                    },
                    "required": ["flag", "meaning"],
                    "additionalProperties": false
                }
            },
            "pipeline": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "stage": { "type": "string" },
                        "purpose": { "type": "string" }
                    },
                    "required": ["stage", "purpose"],
                    "additionalProperties": false
                }
            },
            "side_effects": { "type": "array", "items": { "type": "string" } },
            "safety": {
                "type": "object",
                "properties": {
                    "level": {
                        "type": "string",
                        "enum": ["safe", "caution", "destructive"]
                    },
                    "reasons": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["level", "reasons"],
                "additionalProperties": false
            },
            "preview_command": { "type": ["string", "null"] },
            "assumptions": { "type": "array", "items": { "type": "string" } },
            "caveats": { "type": "array", "items": { "type": "string" } }
        },
        "required": [
            "summary",
            "flags",
            "pipeline",
            "side_effects",
            "safety",
            "preview_command",
            "assumptions",
            "caveats"
        ],
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
        assert_strict(&explanation().schema);
    }

    #[test]
    fn import_instructions_state_the_limits_the_parser_enforces() {
        assert!(IMPORT_INSTRUCTIONS.contains(&MAX_IMPORT_ITEMS.to_string()));
        assert!(IMPORT_INSTRUCTIONS.contains("{{PLACEHOLDER}}"));
        assert!(IMPORT_INSTRUCTIONS.contains(PROMPT_REVISION));
    }

    #[test]
    fn explanation_instructions_state_the_limits_the_parser_enforces() {
        assert!(EXPLANATION_INSTRUCTIONS.contains(&MAX_EXPLANATION_ITEMS.to_string()));
        assert!(EXPLANATION_INSTRUCTIONS.contains("{{PLACEHOLDER}}"));
        assert!(EXPLANATION_INSTRUCTIONS.contains(PROMPT_REVISION));
        // The local rules are the authority, and the prompt says so.
        assert!(EXPLANATION_INSTRUCTIONS.contains("stricter verdict"));
    }

    /// One response has to serve both the Quick and the Detailed view, so the
    /// schema can never lose the summary or the detail sections.
    #[test]
    fn one_explanation_covers_the_quick_and_detailed_views() {
        let required = explanation().schema["required"].clone();
        let names: Vec<&str> = required
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();

        for field in [
            "summary",
            "flags",
            "pipeline",
            "side_effects",
            "safety",
            "preview_command",
            "assumptions",
            "caveats",
        ] {
            assert!(names.contains(&field), "{field} is missing");
        }
    }
}
