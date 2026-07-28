//! JSON-RPC 2.0, as much of it as MCP over stdio uses.
//!
//! One request per line in, one response per line out. That is the whole
//! transport, and it is why this crate takes no async runtime: there is one
//! stream, it is read in order, and a blocking read is exactly the right
//! shape for it.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The only version of JSON-RPC this speaks.
pub const VERSION: &str = "2.0";

/// A message from the client.
///
/// `id` absent makes it a **notification**: something to act on and say
/// nothing about. Replying to one is a protocol error, which is why the
/// distinction is in the type rather than in a comment.
#[derive(Debug, Deserialize)]
pub struct Request {
    /// Which method is being called.
    pub method: String,
    /// Its arguments, whatever shape the method takes.
    #[serde(default)]
    pub params: Option<Value>,
    /// What to echo back on the response. Absent for a notification.
    #[serde(default)]
    pub id: Option<Value>,
}

impl Request {
    /// True when nothing may be sent back.
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// Why a call failed, in the codes JSON-RPC reserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The line was not JSON.
    Parse,
    /// It was JSON, but not a request.
    BadRequest,
    /// No such method.
    NoSuchMethod,
    /// The method exists; the arguments do not fit it.
    BadParams,
    /// Everything else — including a tool that ran and refused.
    Internal,
}

impl Failure {
    /// The wire code, from the JSON-RPC specification's reserved range.
    const fn code(self) -> i32 {
        match self {
            Self::Parse => -32700,
            Self::BadRequest => -32600,
            Self::NoSuchMethod => -32601,
            Self::BadParams => -32602,
            Self::Internal => -32603,
        }
    }
}

/// A reply, ready to be written as one line.
#[derive(Debug, Serialize)]
pub struct Response {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl Response {
    /// A successful reply.
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: VERSION,
            id,
            result: Some(result),
            error: None,
        }
    }

    /// A failure, with something a caller can act on in the message.
    pub fn failed(id: Value, failure: Failure, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: VERSION,
            id,
            result: None,
            error: Some(json!({ "code": failure.code(), "message": message.into() })),
        }
    }

    /// A failure that arrived before any id could be read — a line that was
    /// not JSON at all. The specification says to answer with a null id.
    pub fn unidentified(failure: Failure, message: impl Into<String>) -> Self {
        Self::failed(Value::Null, failure, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_without_an_id_is_a_notification() {
        let notification: Request =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .expect("parses");
        assert!(notification.is_notification());

        let call: Request =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)
                .expect("parses");
        assert!(!call.is_notification());
    }

    /// An id of `0` is a real id, and a falsy-looking one is exactly where a
    /// hand-rolled check would go wrong.
    #[test]
    fn an_id_of_zero_is_still_an_id() {
        let call: Request =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":0,"method":"ping"}"#).expect("parses");
        assert!(!call.is_notification());
    }

    #[test]
    fn a_reply_carries_either_a_result_or_an_error_and_never_both() {
        let ok = serde_json::to_value(Response::ok(json!(1), json!({}))).expect("serialises");
        assert!(ok.get("result").is_some() && ok.get("error").is_none());

        let bad = serde_json::to_value(Response::failed(json!(1), Failure::BadParams, "nope"))
            .expect("serialises");
        assert!(bad.get("result").is_none());
        assert_eq!(bad["error"]["code"], json!(-32602));
        assert_eq!(bad["error"]["message"], json!("nope"));
    }
}
