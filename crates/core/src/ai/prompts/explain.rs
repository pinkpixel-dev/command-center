//! Command explanation: one saved entry in, one structured explanation out
//! that has to fill both the Quick and the Detailed view.

use std::sync::OnceLock;

use serde_json::{json, Value};

use super::StructuredTask;

pub fn explanation() -> &'static StructuredTask {
    static TASK: OnceLock<StructuredTask> = OnceLock::new();
    TASK.get_or_init(|| StructuredTask {
        name: "command_center_explanation",
        instructions: INSTRUCTIONS,
        schema: schema(),
    })
}

const INSTRUCTIONS: &str = concat!(
    "Command Center command explanation, revision 2026-07-27.1.\n\n",
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

fn schema() -> Value {
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
    use super::super::{MAX_EXPLANATION_ITEMS, PROMPT_REVISION};
    use super::*;

    #[test]
    fn explanation_instructions_state_the_limits_the_parser_enforces() {
        assert!(INSTRUCTIONS.contains(&MAX_EXPLANATION_ITEMS.to_string()));
        assert!(INSTRUCTIONS.contains("{{PLACEHOLDER}}"));
        assert!(INSTRUCTIONS.contains(PROMPT_REVISION));
        // The local rules are the authority, and the prompt says so.
        assert!(INSTRUCTIONS.contains("stricter verdict"));
    }

    /// One response has to serve both the Quick and the Detailed view, so the
    /// schema can never lose the summary or the detail sections.
    #[test]
    fn one_explanation_covers_the_quick_and_detailed_views() {
        let required = schema()["required"].clone();
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
