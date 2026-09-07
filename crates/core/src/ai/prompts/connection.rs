//! The connection test. It asks for the smallest possible strict structured
//! response, because the point is proving the account can do that at all.

use std::sync::OnceLock;

use serde_json::json;

use super::StructuredTask;

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
