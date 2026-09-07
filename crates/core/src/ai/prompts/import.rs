//! Import extraction: one technical document in, a list of saveable items out.

use std::sync::OnceLock;

use serde_json::{json, Value};

use super::StructuredTask;

pub fn import_extraction() -> &'static StructuredTask {
    static TASK: OnceLock<StructuredTask> = OnceLock::new();
    TASK.get_or_init(|| StructuredTask {
        name: "command_center_import_extraction",
        instructions: INSTRUCTIONS,
        schema: schema(),
    })
}

const INSTRUCTIONS: &str = concat!(
    "Command Center import extraction, revision 2026-07-27.1.\n\n",
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

fn schema() -> Value {
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
    use super::super::{MAX_IMPORT_ITEMS, PROMPT_REVISION};
    use super::*;

    #[test]
    fn import_instructions_state_the_limits_the_parser_enforces() {
        assert!(INSTRUCTIONS.contains(&MAX_IMPORT_ITEMS.to_string()));
        assert!(INSTRUCTIONS.contains("{{PLACEHOLDER}}"));
        assert!(INSTRUCTIONS.contains(PROMPT_REVISION));
    }
}
