//! The conversational assistant: a bounded conversation in, one answer and up
//! to four proposed commands out.

use std::sync::OnceLock;

use serde_json::{json, Value};

use super::StructuredTask;

pub fn assistant() -> &'static StructuredTask {
    static TASK: OnceLock<StructuredTask> = OnceLock::new();
    TASK.get_or_init(|| StructuredTask {
        name: "command_center_assistant",
        instructions: INSTRUCTIONS,
        schema: schema(),
    })
}

const INSTRUCTIONS: &str = concat!(
    "Command Center assistant, revision 2026-07-27.1.\n\n",
    "You help one person work with their own terminal command library. They can read a terminal ",
    "and they are asking you because they want a straight answer, not a lesson.\n\n",
    "You cannot run anything, read their filesystem, or search their library. You see only what ",
    "the message contains. Say so plainly when the answer depends on something you cannot see.\n\n",
    "The conversation is given to you as labelled turns. Earlier turns are context; answer the ",
    "last user message.\n\n",
    "Rules:\n",
    "- reply: plain sentences, no Markdown headings, no bullet characters, no code fences. Keep it ",
    "to what was asked. A short answer to a short question is the right answer.\n",
    "- Never put a runnable command inside reply. Commands belong in commands, and the reply ",
    "refers to them in words.\n",
    "- commands: the commands you are proposing, in the order the reader should consider them. ",
    "Empty when the question does not call for one.\n",
    "- commands[].command: exactly what the user would type, with nothing else on the line. No ",
    "prompt character, no surrounding backticks, no explanation.\n",
    "- Use {{PLACEHOLDER}} for any value the user has to supply, such as {{FILE}} or {{PORT}}. A ",
    "{{PLACEHOLDER}} already in the conversation is a secret that was removed on the user's ",
    "machine. Keep it as it is and never guess what it stood for.\n",
    "- commands[].title: short, specific, ordinary words. Not a slug.\n",
    "- commands[].why: one sentence on what this command does for the request.\n",
    "- commands[].kind: command for a single command, sequence for ordered steps, script for a ",
    "shebang or shell control flow, snippet for configuration or code.\n",
    "- commands[].shell: bash, fish, zsh, powershell, or null when it does not matter.\n",
    "- commands[].risk_suggestion: safe, caution, or destructive, judged by what it does on a ",
    "normal machine. This is advice. Command Center runs its own rules and keeps the stricter ",
    "verdict.\n",
    "- commands[].risk_reasons: short phrases, empty when the command is ordinary.\n",
    "- Never invent a flag, a tool, or a path. If you are unsure a flag exists, leave it out and ",
    "say what you are unsure about in reply.\n",
    "- At most 4 commands. Fewer is usually better.",
);

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "reply": { "type": "string" },
            "commands": {
                "type": "array",
                "items": super::proposal_schema()
            }
        },
        "required": ["reply", "commands"],
        "additionalProperties": false
    })
}

#[cfg(test)]
mod tests {
    use super::super::{MAX_ASSISTANT_PROPOSALS, PROMPT_REVISION};
    use super::*;

    #[test]
    fn assistant_instructions_state_the_limits_the_parser_enforces() {
        assert!(INSTRUCTIONS.contains(&MAX_ASSISTANT_PROPOSALS.to_string()));
        assert!(INSTRUCTIONS.contains("{{PLACEHOLDER}}"));
        assert!(INSTRUCTIONS.contains(PROMPT_REVISION));
        // The local rules are the authority, and the prompt says so.
        assert!(INSTRUCTIONS.contains("stricter verdict"));
    }

    /// A proposal has to arrive as its own field. A command buried in prose
    /// would skip the local risk check and the review-and-save path with it.
    #[test]
    fn proposed_commands_are_a_separate_field_rather_than_prose() {
        assert!(INSTRUCTIONS.contains("Never put a runnable command inside reply"));

        let command = &schema()["properties"]["commands"]["items"];
        assert_eq!(command["properties"]["command"]["type"], "string");
        assert_eq!(
            command["properties"]["risk_suggestion"]["enum"],
            json!(["safe", "caution", "destructive"])
        );
    }
}
