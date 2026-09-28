//! The stdio transport: a line in, a line out.
//!
//! What each line *means* is [`protocol`]'s; this is only the framing, and
//! running a tool on this thread against the project directory it names.

use std::io::{BufRead, Write};

use serde_json::Value;

use crate::protocol::{self, Handled};
use crate::tools;

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
fn handle(line: &str) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(problem) => return Some(protocol::unreadable(problem)),
    };
    match protocol::handle(message, listed) {
        Handled::Silent => None,
        Handled::Answered(reply) => Some(reply),
        Handled::Call(call) => Some(match tools::find(&call.name) {
            Some(tool) => {
                let outcome = tool.call(&call.arguments);
                call.answer(outcome)
            }
            None => call.unknown(),
        }),
    }
}

/// Every tool, as a client lists them.
fn listed() -> Vec<Value> {
    tools::registry()
        .iter()
        .map(|tool| protocol::listing(tool.as_ref()))
        .collect()
}
