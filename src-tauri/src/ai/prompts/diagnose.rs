//! Terminal error analysis: pasted output in, a likely cause and some
//! practical next checks out.
//!
//! The hardest part of this prompt is not the diagnosis, it is stopping the
//! model from inventing one. Terminal output is usually a fragment, so the
//! prompt spends most of its length on saying when to admit that.

use std::sync::OnceLock;

use serde_json::{json, Value};

use super::StructuredTask;

pub fn error_analysis() -> &'static StructuredTask {
    static TASK: OnceLock<StructuredTask> = OnceLock::new();
    TASK.get_or_init(|| StructuredTask {
        name: "command_center_error_analysis",
        instructions: INSTRUCTIONS,
        schema: schema(),
    })
}

const INSTRUCTIONS: &str = concat!(
    "Command Center terminal error analysis, revision 2026-07-27.1.\n\n",
    "You read the terminal output one person pasted after something went wrong. They can read a ",
    "terminal. They want to know what actually failed and what to check next, not a tutorial.\n\n",
    "You see only the pasted text. You cannot run anything, read their filesystem, see their ",
    "configuration, or search their library. Most pasted output is a fragment of a longer session, ",
    "so the real cause is often above what you were given. Say that when it is true.\n\n",
    "Rules:\n",
    "- summary: one sentence naming what failed. The thing that broke, not the advice.\n",
    "- cause: two or three sentences on the most likely reason, in plain words. When the output ",
    "supports more than one explanation, give the likeliest and name the alternative.\n",
    "- confidence: high when the output names the failure outright, medium when you are reading ",
    "between the lines, low when you are largely guessing. Be honest. Low is a useful answer.\n",
    "- relevant_lines: the lines that actually carry the diagnosis, at most 8. Every line of the ",
    "output is prefixed with its line number and a pipe character. Use those numbers for line, and ",
    "never copy a prefix into quote. quote is the line as printed, trimmed. why is a short phrase ",
    "on what that line tells you. Skip stack frames and progress noise that add nothing.\n",
    "- checks: what to do next, in the order worth trying, at most 8. check is the action in a ",
    "few words. why is what it would tell them. Prefer a check that narrows the problem over one ",
    "that changes something. Empty only when the output already settles it.\n",
    "- commands: a command worth running next, when there is an obvious one. This is usually a ",
    "diagnostic: read a log, show a status, print a version, check a port. Propose a fix command ",
    "only when the output makes the fix unambiguous. Empty is the right answer more often than ",
    "not, and an empty list is better than a guess.\n",
    "- commands[].command: exactly what the user would type, with nothing else on the line. No ",
    "prompt character, no surrounding backticks, no explanation.\n",
    "- Use {{PLACEHOLDER}} for any value the user has to supply, such as {{FILE}} or {{PORT}}. A ",
    "{{PLACEHOLDER}} already in the output is a secret that was removed on the user's machine. ",
    "Keep it as it is, never guess what it stood for, and never suggest a command that needs to ",
    "know what it stood for.\n",
    "- commands[].title: short, specific, ordinary words. Not a slug.\n",
    "- commands[].why: one sentence on what running it would tell them.\n",
    "- commands[].kind: command for a single command, sequence for ordered steps, script for a ",
    "shebang or shell control flow, snippet for configuration or code.\n",
    "- commands[].shell: bash, fish, zsh, powershell, or null when it does not matter.\n",
    "- commands[].risk_suggestion: safe, caution, or destructive, judged by what it does on a ",
    "normal machine. This is advice. Command Center runs its own rules and keeps the stricter ",
    "verdict.\n",
    "- commands[].risk_reasons: short phrases, empty when the command is ordinary.\n",
    "- uncertainty: what you would need to see to be sure, such as the command that produced this, ",
    "the lines above the paste, a version, or a configuration file. Empty string only when the ",
    "output genuinely stands on its own.\n",
    "- Never invent a file path, a flag, a tool, a version, or a log location that the output does ",
    "not mention. Guessing a plausible path is the failure mode to avoid here.\n",
    "- When the pasted text is not an error at all, say so in summary and leave checks and commands ",
    "empty rather than inventing a problem.",
);

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": { "type": "string" },
            "cause": { "type": "string" },
            "confidence": {
                "type": "string",
                "enum": ["high", "medium", "low"]
            },
            "relevant_lines": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "line": { "type": ["integer", "null"] },
                        "quote": { "type": "string" },
                        "why": { "type": "string" }
                    },
                    "required": ["line", "quote", "why"],
                    "additionalProperties": false
                }
            },
            "checks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "check": { "type": "string" },
                        "why": { "type": "string" }
                    },
                    "required": ["check", "why"],
                    "additionalProperties": false
                }
            },
            "commands": {
                "type": "array",
                "items": super::proposal_schema()
            },
            "uncertainty": { "type": "string" }
        },
        "required": [
            "summary",
            "cause",
            "confidence",
            "relevant_lines",
            "checks",
            "commands",
            "uncertainty"
        ],
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::super::{MAX_ANALYSIS_ITEMS, PROMPT_REVISION};
    use super::*;

    #[test]
    fn analysis_instructions_state_the_limits_the_parser_enforces() {
        assert!(INSTRUCTIONS.contains(&MAX_ANALYSIS_ITEMS.to_string()));
        assert!(INSTRUCTIONS.contains("{{PLACEHOLDER}}"));
        assert!(INSTRUCTIONS.contains(PROMPT_REVISION));
        // The local rules are the authority, and the prompt says so.
        assert!(INSTRUCTIONS.contains("stricter verdict"));
    }

    /// Pasted output is nearly always a fragment. A prompt that does not say so
    /// gets a confident answer about a cause that was never on screen.
    #[test]
    fn the_prompt_makes_room_for_not_knowing() {
        assert!(INSTRUCTIONS.contains("Low is a useful answer"));
        assert!(INSTRUCTIONS.contains("an empty list is better than a guess"));
        assert!(INSTRUCTIONS.contains("Never invent a file path"));

        let required = schema()["required"].as_array().unwrap().clone();
        let names: Vec<&str> = required.iter().map(|value| value.as_str().unwrap()).collect();
        assert!(names.contains(&"confidence"));
        assert!(names.contains(&"uncertainty"));
    }

    /// Line numbers are how the analysis points at the paste without the app
    /// having to match text back to the source.
    #[test]
    fn relevant_lines_are_cited_by_number() {
        assert!(INSTRUCTIONS.contains("line number and a pipe character"));
        assert_eq!(
            schema()["properties"]["relevant_lines"]["items"]["properties"]["line"]["type"],
            json!(["integer", "null"])
        );
    }
}
