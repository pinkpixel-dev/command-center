//! Versioned instructions and strict response schemas. Every request the app
//! makes is defined under here, one file per workflow, so a prompt change is
//! reviewable on its own and no single file grows past the repository limit.

use serde_json::{json, Value};

mod chat;
mod connection;
mod convert;
mod diagnose;
mod explain;
mod import;

pub use chat::assistant;
pub use connection::{connection_test, CONNECTION_TEST_INPUT};
pub use convert::shell_conversion;
pub use diagnose::error_analysis;
pub use explain::explanation;
pub use import::import_extraction;

/// Bumped whenever the wording or a schema below changes, so cached results and
/// bug reports can be traced back to a known prompt.
pub const PROMPT_REVISION: &str = "2026-07-27.1";

/// Ceiling the extraction prompt states and the parser enforces.
pub const MAX_IMPORT_ITEMS: usize = 150;

/// Ceiling the explanation prompt states and the parser enforces, per list.
pub const MAX_EXPLANATION_ITEMS: usize = 12;

/// Ceiling the assistant prompt states and the parser enforces.
pub const MAX_ASSISTANT_PROPOSALS: usize = 4;

/// Ceiling the error-analysis prompt states and the parser enforces, per list.
pub const MAX_ANALYSIS_ITEMS: usize = 8;

/// Ceiling the conversion prompt states and the parser enforces, per list.
pub const MAX_CONVERSION_NOTES: usize = 8;

pub struct StructuredTask {
    pub name: &'static str,
    pub instructions: &'static str,
    pub schema: Value,
}

/// The shape every proposed command arrives in, whoever proposed it. Keeping
/// one fragment means the assistant, error analysis, and shell conversion all
/// parse into `ai::proposal::RawProposal` and all reach the screen through the
/// same local risk review.
fn proposal_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": { "type": "string" },
            "title": { "type": "string" },
            "why": { "type": "string" },
            "kind": {
                "type": "string",
                "enum": ["command", "sequence", "script", "snippet"]
            },
            "shell": { "type": ["string", "null"] },
            "risk_suggestion": {
                "type": "string",
                "enum": ["safe", "caution", "destructive"]
            },
            "risk_reasons": { "type": "array", "items": { "type": "string" } }
        },
        "required": [
            "command",
            "title",
            "why",
            "kind",
            "shell",
            "risk_suggestion",
            "risk_reasons"
        ],
        "additionalProperties": false
    })
}

// Instruction text is deliberately not shared between workflows. The schema is
// code and has to agree; the wording is copy, and each prompt says what its own
// task needs in the way that task needs it.

#[cfg(test)]
mod tests;
