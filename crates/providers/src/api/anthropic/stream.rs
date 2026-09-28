//! What comes back: a reply as server-sent events.
//!
//! The stream is `event:` / `data:` lines, an event ending at a blank line.
//! Every `data:` is a JSON object whose `type` repeats the event's name, so
//! the name line is not needed and the object alone is parsed. The events, in
//! the order the API sends them:
//!
//! - `message_start` — the message's id, model, and the input side of `usage`;
//! - per content block: `content_block_start`, some `content_block_delta`s
//!   (`text_delta`, `thinking_delta`, `signature_delta`, `input_json_delta`),
//!   `content_block_stop`;
//! - `message_delta` — the stop reason, and `usage` as it ended;
//! - `message_stop`.
//!
//! `ping` may arrive anywhere, and `error` in place of the rest (an
//! overloaded API mid-reply). Anything else is read as [`Event::Other`] and
//! skipped, so a new event type is not a broken stream.

use std::io::BufRead;

use serde::Deserialize;

/// One event of a streamed reply.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// The reply has begun.
    MessageStart {
        /// The message so far: its id, model and input usage.
        message: Started,
    },
    /// A content block has begun.
    ContentBlockStart {
        /// Its place in the reply.
        index: usize,
        /// What kind of block, and what it holds so far.
        content_block: BlockStart,
    },
    /// More of a content block.
    ContentBlockDelta {
        /// Which block.
        index: usize,
        /// What was added.
        delta: Delta,
    },
    /// A content block is complete.
    ContentBlockStop {
        /// Which block.
        index: usize,
    },
    /// The reply's end: why it stopped, and its usage.
    MessageDelta {
        /// Why it stopped.
        delta: Ending,
        /// The usage as it ended.
        #[serde(default)]
        usage: Option<UsageFields>,
    },
    /// The reply is complete.
    MessageStop,
    /// Nothing; the connection is alive.
    Ping,
    /// The API failed partway: the reply ends here.
    Error {
        /// What went wrong.
        error: ApiError,
    },
    /// An event this client does not know.
    #[serde(other)]
    Other,
}

/// `message_start`'s message.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Started {
    /// The message id.
    pub id: String,
    /// The model answering.
    pub model: String,
    /// The input side of the usage.
    #[serde(default)]
    pub usage: UsageFields,
}

/// What a content block is, as it starts.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockStart {
    /// Words.
    Text {
        /// Usually empty; the deltas carry the words.
        #[serde(default)]
        text: String,
    },
    /// Thinking.
    Thinking {
        /// Usually empty.
        #[serde(default)]
        thinking: String,
        /// Usually empty; a `signature_delta` carries it.
        #[serde(default)]
        signature: String,
    },
    /// Encrypted thinking, whole.
    RedactedThinking {
        /// Opaque.
        data: String,
    },
    /// A tool call; its arguments arrive as `input_json_delta`s.
    ToolUse {
        /// The call's id.
        id: String,
        /// The tool.
        name: String,
    },
    /// A block this client does not know.
    #[serde(other)]
    Other,
}

/// What a `content_block_delta` adds.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Delta {
    /// More words.
    TextDelta {
        /// The words.
        text: String,
    },
    /// More of a thinking block's text.
    ThinkingDelta {
        /// The text.
        thinking: String,
    },
    /// A thinking block's signature.
    SignatureDelta {
        /// The signature.
        signature: String,
    },
    /// More of a tool call's arguments, as a fragment of JSON text.
    InputJsonDelta {
        /// The fragment.
        partial_json: String,
    },
    /// A delta this client does not know.
    #[serde(other)]
    Other,
}

/// `message_delta`'s delta.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Ending {
    /// `end_turn`, `tool_use`, `max_tokens`, `refusal`, …
    pub stop_reason: Option<String>,
    /// Set only for a refusal: which classifier, and why.
    #[serde(default)]
    pub stop_details: Option<StopDetails>,
}

/// Why a reply was refused.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StopDetails {
    /// The classifier's category, e.g. `cyber`, `bio`.
    #[serde(default)]
    pub category: Option<String>,
    /// In words.
    #[serde(default)]
    pub explanation: Option<String>,
}

/// The `usage` object, as either end of a stream carries it. A field absent
/// from one event is not zero; it is left as the other event said.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct UsageFields {
    /// Input read at full price.
    pub input_tokens: Option<u64>,
    /// Output, thinking included.
    pub output_tokens: Option<u64>,
    /// Input written to the cache, both lifetimes together.
    pub cache_creation_input_tokens: Option<u64>,
    /// Input read from the cache.
    pub cache_read_input_tokens: Option<u64>,
    /// The cache writes by lifetime.
    pub cache_creation: Option<CacheCreation>,
}

/// Cache writes by lifetime, which are priced differently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct CacheCreation {
    /// Written to the five-minute cache.
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u64,
    /// Written to the one-hour cache.
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u64,
}

/// An `error` event's error.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ApiError {
    /// E.g. `overloaded_error`.
    #[serde(rename = "type")]
    pub kind: String,
    /// In words.
    pub message: String,
}

/// Why a stream could not be read.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// The connection failed partway.
    #[error("the reply stopped arriving: {0}")]
    Io(#[from] std::io::Error),
    /// An event's data was not the JSON it should be.
    #[error("the reply carried something unreadable ({error}): {data}")]
    Unreadable {
        /// What the parser said.
        error: serde_json::Error,
        /// The data line.
        data: String,
    },
}

/// The events `reader` carries, in order, until it ends.
pub fn events<R: BufRead>(reader: R) -> impl Iterator<Item = Result<Event, StreamError>> {
    let mut lines = reader.lines();
    std::iter::from_fn(move || {
        let mut data = String::new();
        loop {
            match lines.next() {
                None if data.is_empty() => return None,
                None => break,
                Some(Err(error)) => return Some(Err(error.into())),
                Some(Ok(line)) if line.is_empty() && !data.is_empty() => break,
                Some(Ok(line)) => {
                    if let Some(more) = line.strip_prefix("data:") {
                        if !data.is_empty() {
                            data.push('\n');
                        }
                        data.push_str(more.strip_prefix(' ').unwrap_or(more));
                    }
                }
            }
        }
        Some(serde_json::from_str(&data).map_err(|error| StreamError::Unreadable { error, data }))
    })
}
