//! The dispatch loop: a line in, a line out.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::rpc::{Failure, Request, Response};
use crate::tools;

/// What this server calls itself when a client asks.
const NAME: &str = "scorsese";

/// The MCP revisions this server knows how to speak.
///
/// A client names the one it wants and the server answers with one they share.
/// Newest first, so the fallback for a client asking for something unknown is
/// the most capable thing on offer rather than the oldest.
const PROTOCOLS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// Reads requests from `input` until it ends, writing a reply to `output` for
/// each one that is not a notification.
///
/// A malformed line is answered and the loop carries on. A client that sends
/// one line of nonsense has not ended the conversation, and a server that
/// exits on it would take a whole session down over a typo.
pub fn serve(input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Some(response) = handle(&line) else {
            continue;
        };
        let encoded = serde_json::to_string(&response)
            .unwrap_or_else(|_| r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"could not encode the reply"}}"#.to_owned());
        writeln!(output, "{encoded}")?;
        // Flushed every line: the client is waiting on this answer before it
        // sends the next request, so a buffered reply is a hung session.
        output.flush()?;
    }
    Ok(())
}

/// One line's reply, or `None` when the line was a notification.
pub fn handle(line: &str) -> Option<Response> {
    let request: Request = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(problem) => {
            return Some(Response::unidentified(
                Failure::Parse,
                format!("not a JSON-RPC request: {problem}"),
            ));
        }
    };
    if request.is_notification() {
        // `notifications/initialized` and friends: acted on by existing, and
        // answered by saying nothing at all.
        return None;
    }
    let id = request.id.clone().unwrap_or(Value::Null);
    Some(match dispatch(&request) {
        Ok(result) => Response::ok(id, result),
        Err((failure, message)) => Response::failed(id, failure, message),
    })
}

/// Which method, and what it answers with.
fn dispatch(request: &Request) -> Result<Value, (Failure, String)> {
    match request.method.as_str() {
        "initialize" => Ok(initialize(request.params.as_ref())),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": listed() })),
        "tools/call" => call(request.params.as_ref()),
        other => Err((
            Failure::NoSuchMethod,
            format!("this server has no method `{other}`"),
        )),
    }
}

/// The handshake: what this server is and what it can do.
fn initialize(params: Option<&Value>) -> Value {
    let wanted = params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str);
    // Answer with the client's own version when it is one we speak, because
    // that is the version the conversation then uses. Otherwise offer the
    // newest we know and let the client decide whether it can hold it.
    let protocol = wanted
        .filter(|version| PROTOCOLS.contains(version))
        .unwrap_or(PROTOCOLS[0]);
    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": NAME, "version": env!("CARGO_PKG_VERSION") }
    })
}

/// Every tool, as a client lists them.
fn listed() -> Vec<Value> {
    tools::registry()
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name(),
                "description": tool.description(),
                "inputSchema": tool.schema()
            })
        })
        .collect()
}

/// Runs one tool.
///
/// A tool that refuses comes back as `isError` on a *successful* call rather
/// than as a protocol error, which is the distinction MCP draws: the call
/// worked, and what it has to say is that the thing could not be done. A
/// client shows that to whoever asked instead of treating it as a fault in the
/// connection.
fn call(params: Option<&Value>) -> Result<Value, (Failure, String)> {
    let params = params.ok_or((Failure::BadParams, "no parameters given".to_owned()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((Failure::BadParams, "`name` is required".to_owned()))?;
    let tool = tools::find(name).ok_or((
        Failure::NoSuchMethod,
        format!("this server has no tool `{name}`"),
    ))?;

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
    // A refusal is words and nothing else — there is no picture of something
    // that did not happen — so it takes the same shape a plain answer does.
    let (reply, failed) = match tool.call(&arguments) {
        Ok(reply) => (reply, false),
        Err(text) => (text.into(), true),
    };
    Ok(json!({
        "content": reply.content(),
        "isError": failed
    }))
}
