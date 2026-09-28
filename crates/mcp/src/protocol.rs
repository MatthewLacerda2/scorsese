//! The protocol apart from any transport: one message in, what to do about it
//! out.
//!
//! Two transports speak it. [`serve`](crate::serve) reads lines from stdin for
//! a client that spawned this binary; the hosted server (`scorsese-server`,
//! #539) answers `POST /api/mcp` for a client somewhere on the internet. They
//! differ in how a message arrives and — the part that matters — in how a tool
//! runs: here a tool is handed a project directory and runs on this thread;
//! there it runs for one signed-in user, on a project that lives in Postgres.
//!
//! So [`handle`] answers everything the protocol itself decides — the
//! handshake, `ping`, the tool list it is given, a malformed request — and
//! hands a `tools/call` back as a [`Call`] for the transport to run however it
//! runs tools, and to answer with [`Call::answer`]. The wording of every reply
//! and every refusal is decided here once, so a client pointed at either
//! transport meets the same protocol.

use serde_json::{Value, json};

use crate::rpc::{Failure, Request, Response};
use crate::tools::{Reply, Tool};

/// What this server calls itself when a client asks.
pub const NAME: &str = "scorsese";

/// The MCP revisions this server knows how to speak.
///
/// A client names the one it wants and the server answers with one they share.
/// Newest first, so the fallback for a client asking for something unknown is
/// the most capable thing on offer rather than the oldest.
pub const PROTOCOLS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// What to do about one message.
#[derive(Debug)]
pub enum Handled {
    /// A notification: acted on by existing, and answered by saying nothing
    /// at all. Replying to one is a protocol error.
    Silent,
    /// The reply, ready to send.
    Answered(Value),
    /// A tool to run. The transport runs it and sends [`Call::answer`].
    Call(Call),
}

/// A `tools/call`: which tool, with what, and the id its answer carries.
#[derive(Debug)]
pub struct Call {
    id: Value,
    /// The tool asked for, by name.
    pub name: String,
    /// Its arguments — an empty object when the client sent none.
    pub arguments: Value,
}

impl Call {
    /// The reply to send once the tool has run.
    ///
    /// A tool that refuses comes back as `isError` on a *successful* call
    /// rather than as a protocol error, which is the distinction MCP draws: the
    /// call worked, and what it has to say is that the thing could not be done.
    /// A client shows that to whoever asked instead of treating it as a fault
    /// in the connection.
    pub fn answer(self, outcome: Result<Reply, String>) -> Value {
        // A refusal is words and nothing else — there is no picture of
        // something that did not happen — so it takes the shape a plain
        // answer does.
        let (reply, failed) = match outcome {
            Ok(reply) => (reply, false),
            Err(text) => (Reply::from(text), true),
        };
        encode(Response::ok(
            self.id,
            json!({ "content": reply.content(), "isError": failed }),
        ))
    }

    /// The reply when this server has no tool by that name.
    pub fn unknown(self) -> Value {
        let message = format!("this server has no tool `{}`", self.name);
        encode(Response::failed(self.id, Failure::NoSuchMethod, message))
    }
}

/// What to do about `message`, a JSON-RPC request or notification.
///
/// `tools` is asked for the tool list only when a client asks for it: each
/// entry as [`listing`] writes one. It is the transport's, because the two
/// transports do not serve the same set.
pub fn handle(message: Value, tools: impl FnOnce() -> Vec<Value>) -> Handled {
    let request: Request = match serde_json::from_value(message) {
        Ok(request) => request,
        Err(problem) => return Handled::Answered(unreadable(problem)),
    };
    if request.is_notification() {
        // `notifications/initialized` and friends.
        return Handled::Silent;
    }
    let id = request.id.clone().unwrap_or(Value::Null);
    let answered = match request.method.as_str() {
        "initialize" => Ok(initialize(request.params.as_ref())),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => match call(id.clone(), request.params) {
            Ok(call) => return Handled::Call(call),
            Err(refused) => Err(refused),
        },
        other => Err((
            Failure::NoSuchMethod,
            format!("this server has no method `{other}`"),
        )),
    };
    Handled::Answered(encode(match answered {
        Ok(result) => Response::ok(id, result),
        Err((failure, message)) => Response::failed(id, failure, message),
    }))
}

/// The reply to something that is not a JSON-RPC request at all — a line
/// that is not JSON, or JSON that is not a request. No id could be read, so
/// the specification says to answer with a null one.
pub fn unreadable(problem: impl std::fmt::Display) -> Value {
    encode(Response::unidentified(
        Failure::Parse,
        format!("not a JSON-RPC request: {problem}"),
    ))
}

/// One tool as `tools/list` shows it.
pub fn listing(tool: &dyn Tool) -> Value {
    json!({
        "name": tool.name(),
        "description": tool.description(),
        "inputSchema": tool.schema()
    })
}

/// The revision to speak with a client that asked for `wanted`.
///
/// The client's own when it is one we speak, because that is the version the
/// conversation then uses. Otherwise the newest we know, and the client
/// decides whether it can hold it.
pub fn negotiate(wanted: Option<&str>) -> &'static str {
    wanted
        .and_then(|wanted| PROTOCOLS.iter().find(|known| **known == wanted))
        .copied()
        .unwrap_or(PROTOCOLS[0])
}

/// The handshake: what this server is and what it can do.
fn initialize(params: Option<&Value>) -> Value {
    let wanted = params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str);
    json!({
        "protocolVersion": negotiate(wanted),
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": NAME, "version": env!("CARGO_PKG_VERSION") }
    })
}

/// A `tools/call`'s name and arguments, or why they are not there.
fn call(id: Value, params: Option<Value>) -> Result<Call, (Failure, String)> {
    let params = params.ok_or((Failure::BadParams, "no parameters given".to_owned()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((Failure::BadParams, "`name` is required".to_owned()))?
        .to_owned();
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
    Ok(Call {
        id,
        name,
        arguments,
    })
}

/// A reply as the JSON it goes on the wire as.
fn encode(response: Response) -> Value {
    serde_json::to_value(response).unwrap_or_else(|_| {
        json!({
            "jsonrpc": crate::rpc::VERSION, "id": null,
            "error": { "code": -32603, "message": "could not encode the reply" }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_revision_is_kept_and_an_unknown_one_is_answered_with_the_newest() {
        assert_eq!(negotiate(Some("2025-03-26")), "2025-03-26");
        assert_eq!(negotiate(Some("1999-01-01")), PROTOCOLS[0]);
        assert_eq!(negotiate(None), PROTOCOLS[0]);
    }

    #[test]
    fn a_call_is_handed_back_and_everything_else_is_answered() {
        let message = json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": { "name": "project_read" }
        });
        let Handled::Call(call) = handle(message, Vec::new) else {
            panic!("a tools/call is the transport's to run");
        };
        assert_eq!(call.name, "project_read");
        assert_eq!(call.arguments, json!({}));
        let answered = call.answer(Err("no".into()));
        assert_eq!(answered["id"], json!(4));
        assert_eq!(answered["result"]["isError"], json!(true));

        let ping = json!({ "jsonrpc": "2.0", "id": 5, "method": "ping" });
        assert!(matches!(handle(ping, Vec::new), Handled::Answered(_)));
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(matches!(handle(note, Vec::new), Handled::Silent));
    }
}
