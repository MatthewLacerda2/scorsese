//! The stdio transport: a line in, a line out.
//!
//! What each line *means* is [`protocol`]'s; this is only the framing, and
//! running a tool on this thread against the project directory it names.
//!
//! ## Two threads, because a client may say "stop"
//!
//! Requests are still run one at a time, in the order they arrived. What reads
//! them is a thread of its own, and for one reason: a `render` runs for minutes
//! or hours, and MCP's `notifications/cancelled` for it arrives on the same
//! stream *while it runs*. A server that read a line only after answering the
//! last one could not see that until the render had finished — which is the
//! whole of #647. So the reader looks at each line as it arrives: a cancel
//! trips the [`Cancel`] of the call it names, and everything else is queued
//! for this thread in order.
//!
//! A `render` asked to `wait` is that case. One that is not answers at once and
//! renders on its own thread, one of the session's renders: they are held
//! here, beside the output, and stopped when the session ends (#700). And a
//! call that carries a `progressToken` may write `notifications/progress` to
//! the output before its reply, which is why a call is handed the output.
//!
//! A cancelled call gets **no reply**, as the specification asks of a receiver.
//! What it would have said — a render's "cancelled after 412 of 1890 frames" —
//! goes to stderr instead, which a client shows as the server's log.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};

use scorsese_render::Cancel;
use serde_json::Value;

use crate::protocol::{self, Handled};
use crate::renders::{Renders, Say};
use crate::tools::{self, Context};

/// The calls read but not yet answered, by the id the client gave them —
/// written as JSON, since an id may be a number or a string and `1` and `"1"`
/// are different ids.
type InFlight = Arc<Mutex<HashMap<String, Cancel>>>;

/// One line for the serving thread, with the cancel its call runs under when
/// it is a call.
type Queued = std::io::Result<(String, Option<(String, Cancel)>)>;

/// Reads requests from `input` until it ends, writing a reply to `output` for
/// each one that is not a notification and was not cancelled.
///
/// A malformed line is answered and the loop carries on. A client that sends
/// one line of nonsense has not ended the conversation, and a server that
/// exits on it would take a whole session down over a typo.
pub fn serve(input: impl BufRead + Send + 'static, mut output: impl Write) -> std::io::Result<()> {
    // Declared first, so it is dropped last: whichever way this returns, the
    // renders still running are stopped and their files removed (#700).
    let renders = Renders::default();
    let in_flight = InFlight::default();
    let (send, queue) = channel();
    // Detached rather than scoped: should answering fail — the client gone —
    // this returns at once, and a reader blocked on stdin must not hold it.
    // It ends by itself at the next line, when there is nobody to send it to.
    let reading = Arc::clone(&in_flight);
    std::thread::spawn(move || read(input, &reading, &send));

    for queued in queue {
        let (line, call) = queued?;
        let cancel = call.as_ref().map(|(_, cancel)| cancel);
        let response = handle(&line, cancel, &renders, &mut output);
        if let Some((id, cancel)) = call {
            lock(&in_flight).remove(&id);
            if cancel.is_cancelled() {
                if let Some(said) = response.as_ref().and_then(said) {
                    eprintln!("scorsese-mcp: request {id} was cancelled: {said}");
                }
                continue;
            }
        }
        let Some(response) = response else {
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

/// The reading thread: every line, as it arrives.
///
/// A call is registered under its id *here*, before it is queued, so that a
/// cancel read a moment later finds it whether or not the serving thread has
/// started on it yet. One that has not starts already stopped, which is the
/// cheapest render there is.
fn read(input: impl BufRead, in_flight: &InFlight, send: &Sender<Queued>) {
    for line in input.lines() {
        let line = match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => line,
            Err(problem) => {
                let _ = send.send(Err(problem));
                return;
            }
        };
        let message: Option<Value> = serde_json::from_str(&line).ok();
        let method = message.as_ref().and_then(|m| m.get("method"));
        let call = match method.and_then(Value::as_str) {
            Some("notifications/cancelled") => {
                let named = message
                    .as_ref()
                    .and_then(|m| m.pointer("/params/requestId"))
                    .map(Value::to_string);
                if let Some(cancel) = named.and_then(|id| lock(in_flight).get(&id).cloned()) {
                    cancel.cancel();
                }
                continue;
            }
            Some("tools/call") => message
                .as_ref()
                .and_then(|m| m.get("id"))
                .map(|id| (id.to_string(), Cancel::new())),
            _ => None,
        };
        if let Some((id, cancel)) = &call {
            lock(in_flight).insert(id.clone(), cancel.clone());
        }
        if send.send(Ok((line, call))).is_err() {
            return;
        }
    }
}

/// The table of calls in flight. A panic while it was held — there is no code
/// under the lock that can panic — would leave it poisoned but intact, so the
/// map is used as it stands.
fn lock(in_flight: &InFlight) -> std::sync::MutexGuard<'_, HashMap<String, Cancel>> {
    in_flight
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// What a reply says, as text: the words of its first block, or its error.
fn said(reply: &Value) -> Option<String> {
    reply
        .pointer("/result/content/0/text")
        .or_else(|| reply.pointer("/error/message"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// One line's reply, or `None` when the line was a notification.
///
/// `output` is for what a call says before its reply: the progress a client
/// asked for with a `progressToken`, written as it happens.
fn handle(
    line: &str,
    cancel: Option<&Cancel>,
    renders: &Renders,
    output: &mut impl Write,
) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(problem) => return Some(protocol::unreadable(problem)),
    };
    match protocol::handle(message, listed) {
        Handled::Silent => None,
        Handled::Answered(reply) => Some(reply),
        Handled::Call(call) => Some(match tools::find(&call.name) {
            Some(tool) => {
                let unnamed = Cancel::new();
                let cancel = cancel.unwrap_or(&unnamed);
                let token = call.progress_token().cloned();
                let mut notify = |progress: f64, message: &str| {
                    if let Some(token) = &token {
                        // Best effort: a pipe that will not take this will
                        // not take the reply either, and that one is reported.
                        let note = protocol::progress(token, progress, message);
                        let _ = writeln!(output, "{note}").and_then(|()| output.flush());
                    }
                };
                let report = token.is_some().then_some(&mut notify as &mut Say<'_>);
                let outcome =
                    tool.call_in(&call.arguments, &mut Context::new(cancel, renders, report));
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
