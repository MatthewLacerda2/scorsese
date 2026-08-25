//! The server, driven the way a client drives it: lines of JSON-RPC in, lines
//! out.
//!
//! Through `serve` over real pipes rather than by calling the dispatch
//! directly, because the framing is half of what can be wrong — a reply that
//! is never flushed, or a notification that gets one, hangs a session just as
//! surely as a wrong result.

mod authoring;
mod briefing;
mod changing;
mod composing;
mod fixture;
mod guarding;
mod handshake;
mod importing;
mod looking;
mod pacing;
mod placing;
mod rendering;
mod searching;
mod seeing;
mod setting;
mod starting;
mod surveying;
mod tuning;
mod watching;

use serde_json::Value;

/// Runs `lines` through the server and returns what it wrote back, parsed.
pub(crate) fn exchange(lines: &[&str]) -> Vec<Value> {
    let input = lines.join("\n");
    let mut output = Vec::new();
    scorsese_mcp::serve(std::io::Cursor::new(input), &mut output).expect("the server runs");
    String::from_utf8(output)
        .expect("the server writes utf-8")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every line out is JSON"))
        .collect()
}

/// One request, one reply.
pub(crate) fn once(line: &str) -> Value {
    let replies = exchange(&[line]);
    assert_eq!(replies.len(), 1, "expected exactly one reply: {replies:?}");
    replies.into_iter().next().expect("checked above")
}

/// A `tools/call` frame.
pub(crate) fn call(name: &str, arguments: Value) -> Value {
    once(
        &serde_json::json!({
            "jsonrpc": "2.0", "id": 9, "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        })
        .to_string(),
    )
}

/// The text a tool call came back with, and whether it was a refusal.
pub(crate) fn said(reply: &Value) -> (String, bool) {
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text in {reply}"))
        .to_owned();
    (text, reply["result"]["isError"] == Value::Bool(true))
}
