//! A conversation past 150K tokens compacts at the start of its next turn
//! (#1438): one summary and the new prompt, nothing older replayed — and a
//! smaller one, or a turn resuming from a pause, is left as it was (`opens`).
//! A summary that does not come back leaves the turn running uncompacted,
//! and a summarising call never counts towards the size (`failing`).

mod failing;
mod opens;

use scorsese_providers::chat::Reply;
use serde_json::Value;

pub(super) use super::Script;

/// `reply`, reported as having read `size` tokens in all.
fn sized(mut reply: Reply, size: u64) -> Reply {
    reply.usage.cache_read = size - reply.usage.input;
    reply
}

/// The words of a request's message, whichever side wrote it.
fn words(message: &Value) -> String {
    match &message["content"] {
        Value::String(text) => text.clone(),
        content => content.to_string(),
    }
}

/// Whether request `n` asked for a summary.
pub(super) fn summarising(script: &Script, n: usize) -> bool {
    script
        .messages(n)
        .concat()
        .contains("do not call any tool.")
}
