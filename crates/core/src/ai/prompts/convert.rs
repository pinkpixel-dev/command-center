//! Shell conversion: one saved command and a target shell in, the converted
//! command and its behavioural differences out.
//!
//! The prompt never asks whether the conversion is correct, only how close it
//! is. There is no "exact" answer available, because two shells that print the
//! same output can still differ on globbing, quoting, exit status, or what
//! happens when a file is missing.

use std::sync::OnceLock;

use serde_json::{json, Value};

use super::StructuredTask;

pub fn shell_conversion() -> &'static StructuredTask {
    static TASK: OnceLock<StructuredTask> = OnceLock::new();
    TASK.get_or_init(|| StructuredTask {
        name: "command_center_shell_conversion",
        instructions: INSTRUCTIONS,
        schema: schema(),
    })
}

const INSTRUCTIONS: &str = concat!(
    "Command Center shell conversion, revision 2026-07-27.1.\n\n",
    "You rewrite one command, script, or snippet so it works in a different shell. The person ",
    "asking can read a terminal and already knows what the original does. They want the rewrite ",
    "and an honest account of what changed underneath it.\n\n",
    "Nothing here is a guarantee. Two shells can print the same output and still differ on ",
    "globbing, word splitting, quoting, exit status, signal handling, or what happens when a file ",
    "is missing. Your job is to name those differences, not to hide them.\n\n",
    "Rules:\n",
    "- converted.command: the command as the user would type it in the target shell, with nothing ",
    "else on the line. No prompt character, no surrounding backticks, no explanation.\n",
    "- Rewrite the syntax, not the intent. Keep the same tools, flags, paths, and order unless the ",
    "target shell genuinely cannot express them that way.\n",
    "- When the original already works unchanged in the target shell, return it unchanged and say ",
    "so in notes. Do not invent a difference to look useful.\n",
    "- Use the target shell's idiom rather than a translation that merely parses. Prefer fish's ",
    "`set -x NAME value` over an `export` that fish does not have, and prefer PowerShell's ",
    "cmdlets and objects over a pipeline of Unix tools that may not exist on that machine.\n",
    "- Keep any {{PLACEHOLDER}} text exactly as it is. A placeholder is a value the user fills in, ",
    "or a secret that was removed on their machine. Never guess what one stood for.\n",
    "- converted.title: short, specific, ordinary words. Not a slug.\n",
    "- converted.why: one sentence on what the converted command does.\n",
    "- converted.kind: command for a single command, sequence for ordered steps, script for a ",
    "shebang or shell control flow, snippet for configuration or code.\n",
    "- converted.shell: the target shell.\n",
    "- converted.risk_suggestion: safe, caution, or destructive, judged by what the converted ",
    "command does on a normal machine. This is advice. Command Center runs its own rules and keeps ",
    "the stricter verdict.\n",
    "- converted.risk_reasons: short phrases, empty when the command is ordinary.\n",
    "- equivalence: close when it does the same job with the same result on a normal machine, ",
    "partial when some of the original behaviour did not survive, uncertain when you are not ",
    "confident the rewrite is right. Never claim more than you can support.\n",
    "- differences: how the converted command behaves differently, at most 8 short entries. Cover ",
    "quoting, globbing, variable scope, exit status, error handling, and stream redirection when ",
    "they actually differ. Empty when nothing meaningful changed.\n",
    "- unsupported: parts of the original the target shell cannot express, or that you are not ",
    "sure about, at most 8 short entries. Say what was dropped or approximated. Empty when ",
    "everything carried over.\n",
    "- notes: one or two sentences of context that does not fit the lists, or an empty string.\n",
    "- Never invent a flag, a builtin, a cmdlet, or a module. If the target shell has no equivalent ",
    "for something, say so in unsupported rather than inventing a plausible name.\n",
    "- Converting to PowerShell often means changing tools, not just syntax. Say so in differences ",
    "when the rewrite depends on a cmdlet that behaves differently from the Unix tool it replaces.",
);

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "converted": super::proposal_schema(),
            "equivalence": {
                "type": "string",
                "enum": ["close", "partial", "uncertain"]
            },
            "differences": { "type": "array", "items": { "type": "string" } },
            "unsupported": { "type": "array", "items": { "type": "string" } },
            "notes": { "type": "string" }
        },
        "required": ["converted", "equivalence", "differences", "unsupported", "notes"],
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::super::{MAX_CONVERSION_NOTES, PROMPT_REVISION};
    use super::*;

    #[test]
    fn conversion_instructions_state_the_limits_the_parser_enforces() {
        assert!(INSTRUCTIONS.contains(&MAX_CONVERSION_NOTES.to_string()));
        assert!(INSTRUCTIONS.contains("{{PLACEHOLDER}}"));
        assert!(INSTRUCTIONS.contains(PROMPT_REVISION));
        // The local rules are the authority, and the prompt says so.
        assert!(INSTRUCTIONS.contains("stricter verdict"));
    }

    /// A conversion is never presented as a guaranteed equivalent, so the
    /// schema has no value that would let the model claim one.
    #[test]
    fn no_equivalence_value_promises_the_conversion_is_exact() {
        let allowed = schema()["properties"]["equivalence"]["enum"].clone();

        assert_eq!(allowed, json!(["close", "partial", "uncertain"]));
        for forbidden in ["exact", "identical", "equivalent", "same"] {
            assert!(
                !allowed.as_array().unwrap().iter().any(|value| value == forbidden),
                "{forbidden} would read as a guarantee"
            );
        }
        assert!(INSTRUCTIONS.contains("Nothing here is a guarantee"));
    }

    #[test]
    fn the_converted_command_is_reviewed_like_any_other_proposal() {
        let converted = &schema()["properties"]["converted"];

        assert_eq!(converted["properties"]["command"]["type"], "string");
        assert_eq!(
            converted["properties"]["risk_suggestion"]["enum"],
            json!(["safe", "caution", "destructive"])
        );
    }
}
