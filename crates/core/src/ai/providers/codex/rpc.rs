//! JSON-RPC message shapes for the Codex app server.
//!
//! The wire format is newline-delimited JSON. Codex omits the `"jsonrpc"`
//! header, so messages are told apart by which fields they carry:
//!
//! - `id` plus `result` or `error`  -> a response to something we sent
//! - `method` plus `id`            -> a request *from* the server, which
//!   must be answered
//! - `method` without `id`         -> a notification
//!
//! This module is pure. It parses and builds messages and never touches the
//! process, so every branch below is testable without a Codex install.

use serde::Serialize;
use serde_json::{json, Value};

/// The largest single JSON-RPC line Command Center will accept.
///
/// Structured task output is bounded well below this by the workflow schemas.
/// The ceiling exists so a wedged or hostile process cannot make the app
/// allocate without limit.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;

/// JSON-RPC error codes Command Center sends back to the server.
pub mod error_code {
    /// The server asked for something this client does not implement.
    pub const METHOD_NOT_FOUND: i64 = -32601;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

/// One message read from the server.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Response {
        id: i64,
        result: Value,
    },
    ErrorResponse {
        id: i64,
        error: RpcError,
    },
    /// A request from the server that must be answered, such as an approval.
    ServerRequest {
        id: Value,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The line was not JSON, or not a JSON object.
    Malformed,
    /// The line was valid JSON but matched no known message shape.
    Unrecognized,
    /// The line exceeded [`MAX_MESSAGE_BYTES`].
    TooLarge,
}

/// Parses one line from the server's stdout.
pub fn parse_incoming(line: &str) -> Result<Incoming, ParseError> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(ParseError::TooLarge);
    }

    let value: Value = serde_json::from_str(line).map_err(|_| ParseError::Malformed)?;
    let object = value.as_object().ok_or(ParseError::Malformed)?;

    let id = object.get("id");
    let method = object.get("method").and_then(Value::as_str);

    match (id, method) {
        // A request from the server. Its id may be a number or a string, and
        // it has to be echoed back exactly as received.
        (Some(id), Some(method)) => Ok(Incoming::ServerRequest {
            id: id.clone(),
            method: method.to_owned(),
            params: object.get("params").cloned().unwrap_or(Value::Null),
        }),
        (None, Some(method)) => Ok(Incoming::Notification {
            method: method.to_owned(),
            params: object.get("params").cloned().unwrap_or(Value::Null),
        }),
        (Some(id), None) => {
            // Responses to our own requests always carry the numeric id we
            // chose. A response with any other id shape is not ours.
            let id = id.as_i64().ok_or(ParseError::Unrecognized)?;
            if let Some(error) = object.get("error") {
                return Ok(Incoming::ErrorResponse {
                    id,
                    error: parse_error_body(error),
                });
            }
            let result = object.get("result").ok_or(ParseError::Unrecognized)?;
            Ok(Incoming::Response {
                id,
                result: result.clone(),
            })
        }
        (None, None) => Err(ParseError::Unrecognized),
    }
}

fn parse_error_body(value: &Value) -> RpcError {
    RpcError {
        code: value.get("code").and_then(Value::as_i64).unwrap_or(0),
        message: value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Codex reported an error without a message.")
            .to_owned(),
    }
}

/// Builds a request line. The caller owns id allocation.
pub fn request_line(id: i64, method: &str, params: impl Serialize) -> Result<String, serde_json::Error> {
    let params = serde_json::to_value(params)?;
    encode(&json!({ "id": id, "method": method, "params": params }))
}

/// Builds a notification line.
pub fn notification_line(method: &str, params: impl Serialize) -> Result<String, serde_json::Error> {
    let params = serde_json::to_value(params)?;
    encode(&json!({ "method": method, "params": params }))
}

/// Builds a rejection for a server request Command Center does not implement.
///
/// Every server request gets an answer. Ignoring one leaves the server waiting
/// and can wedge a turn.
pub fn rejection_line(id: &Value, message: &str) -> Result<String, serde_json::Error> {
    encode(&json!({
        "id": id,
        "error": { "code": error_code::METHOD_NOT_FOUND, "message": message },
    }))
}

fn encode(value: &Value) -> Result<String, serde_json::Error> {
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_response_is_matched_to_its_request_id() {
        let parsed = parse_incoming(r#"{"id":0,"result":{"codexHome":"/tmp/home"}}"#).unwrap();

        match parsed {
            Incoming::Response { id, result } => {
                assert_eq!(id, 0);
                assert_eq!(result["codexHome"], "/tmp/home");
            }
            other => panic!("expected a response, got {other:?}"),
        }
    }

    #[test]
    fn an_error_response_keeps_its_code_and_message() {
        let parsed =
            parse_incoming(r#"{"id":4,"error":{"code":-32001,"message":"Server overloaded"}}"#)
                .unwrap();

        assert_eq!(
            parsed,
            Incoming::ErrorResponse {
                id: 4,
                error: RpcError {
                    code: -32001,
                    message: "Server overloaded".into(),
                },
            }
        );
    }

    #[test]
    fn an_error_body_without_a_message_still_parses() {
        let parsed = parse_incoming(r#"{"id":4,"error":{}}"#).unwrap();
        match parsed {
            Incoming::ErrorResponse { error, .. } => {
                assert_eq!(error.code, 0);
                assert!(!error.message.is_empty());
            }
            other => panic!("expected an error response, got {other:?}"),
        }
    }

    #[test]
    fn a_notification_has_no_id() {
        // Taken from a real handshake against Codex 0.147.0.
        let line = r#"{"method":"remoteControl/status/changed","params":{"status":"disabled"}}"#;

        match parse_incoming(line).unwrap() {
            Incoming::Notification { method, params } => {
                assert_eq!(method, "remoteControl/status/changed");
                assert_eq!(params["status"], "disabled");
            }
            other => panic!("expected a notification, got {other:?}"),
        }
    }

    #[test]
    fn a_notification_without_params_is_still_a_notification() {
        match parse_incoming(r#"{"method":"thread/closed"}"#).unwrap() {
            Incoming::Notification { method, params } => {
                assert_eq!(method, "thread/closed");
                assert_eq!(params, Value::Null);
            }
            other => panic!("expected a notification, got {other:?}"),
        }
    }

    #[test]
    fn a_server_request_is_never_mistaken_for_a_response() {
        let line = r#"{"id":7,"method":"item/commandExecution/requestApproval","params":{"itemId":"i1"}}"#;

        match parse_incoming(line).unwrap() {
            Incoming::ServerRequest { id, method, params } => {
                assert_eq!(id, json!(7));
                assert_eq!(method, "item/commandExecution/requestApproval");
                assert_eq!(params["itemId"], "i1");
            }
            other => panic!("expected a server request, got {other:?}"),
        }
    }

    #[test]
    fn a_server_request_with_a_string_id_keeps_that_id_shape() {
        let line = r#"{"id":"req-9","method":"attestation/generate","params":{}}"#;

        match parse_incoming(line).unwrap() {
            Incoming::ServerRequest { id, .. } => assert_eq!(id, json!("req-9")),
            other => panic!("expected a server request, got {other:?}"),
        }
    }

    #[test]
    fn malformed_and_unrecognized_lines_are_told_apart() {
        assert_eq!(parse_incoming("not json"), Err(ParseError::Malformed));
        assert_eq!(parse_incoming("[1,2,3]"), Err(ParseError::Malformed));
        assert_eq!(parse_incoming("{}"), Err(ParseError::Unrecognized));
        // A response id we could not have issued.
        assert_eq!(
            parse_incoming(r#"{"id":"nope","result":{}}"#),
            Err(ParseError::Unrecognized)
        );
        // An id with neither result nor error.
        assert_eq!(
            parse_incoming(r#"{"id":3}"#),
            Err(ParseError::Unrecognized)
        );
    }

    #[test]
    fn an_oversized_line_is_rejected_before_it_is_parsed() {
        let flood = format!(r#"{{"id":1,"result":"{}"}}"#, "a".repeat(MAX_MESSAGE_BYTES));
        assert_eq!(parse_incoming(&flood), Err(ParseError::TooLarge));
    }

    #[test]
    fn built_lines_are_single_newline_terminated_json() {
        let line = request_line(3, "account/read", json!({ "refreshToken": false })).unwrap();

        assert!(line.ends_with('\n'));
        assert_eq!(line.matches('\n').count(), 1);

        let parsed: Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(parsed["id"], 3);
        assert_eq!(parsed["method"], "account/read");
        assert_eq!(parsed["params"]["refreshToken"], false);
    }

    #[test]
    fn a_notification_line_carries_no_id() {
        let line = notification_line("initialized", json!({})).unwrap();
        let parsed: Value = serde_json::from_str(line.trim_end()).unwrap();

        assert_eq!(parsed["method"], "initialized");
        assert!(parsed.get("id").is_none());
    }

    #[test]
    fn a_rejection_echoes_the_server_request_id_unchanged() {
        let line = rejection_line(&json!("req-9"), "Command Center declines tool requests.").unwrap();
        let parsed: Value = serde_json::from_str(line.trim_end()).unwrap();

        assert_eq!(parsed["id"], "req-9");
        assert_eq!(parsed["error"]["code"], error_code::METHOD_NOT_FOUND);
        assert!(parsed["error"]["message"].as_str().unwrap().contains("declines"));
    }

    #[test]
    fn a_round_trip_survives_the_codec() {
        let line = request_line(11, "thread/start", json!({ "ephemeral": true })).unwrap();
        // Feed our own request back through the parser: it is a request with an
        // id, so it reads as a server request, which proves the id and method
        // survive encoding.
        match parse_incoming(line.trim_end()).unwrap() {
            Incoming::ServerRequest { id, method, params } => {
                assert_eq!(id, json!(11));
                assert_eq!(method, "thread/start");
                assert_eq!(params["ephemeral"], true);
            }
            other => panic!("unexpected shape: {other:?}"),
        }
    }
}
