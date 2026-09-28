//! Claude, as the hosted web app's assistant calls it (#540).
//!
//! The model is **Claude Opus 5.5** ([`MODEL`]), decided by the maintainer and
//! not downgraded to save credits (`CLAUDE.md`). What this module owns is
//! everything about *calling* it that is not the wire ([`crate::api::anthropic`])
//! and not the conversation (the server's): one streamed call in, one
//! [`Response`] out, with the text and progress notes handed to a callback as
//! they arrive — and a trait, [`Claude`], so that no test ever makes the call.
//!
//! ## How a request is laid out: [`request`]
//!
//! Every call of an agent loop resends the whole conversation, so **prompt
//! caching is where the money is** (#540). Two breakpoints:
//!
//! - **The tools and the system prompt**, one breakpoint on the system prompt
//!   (the API renders tools first, so it covers both), with the **one-hour**
//!   lifetime. That prefix is byte-identical for every user of the server —
//!   the tool list is the same registry, the prompt a constant — and the
//!   server's traffic is a handful of people with gaps longer than five
//!   minutes between them. A five-minute entry would be written again after
//!   most gaps at 1.25× input; an hour-long one is written at 2× and read at
//!   0.05× for the rest of the hour.
//! - **The conversation's tail**, by the API's automatic breakpoint (five
//!   minutes), which moves forward as the conversation grows: inside one turn
//!   the calls are seconds apart, and each reads everything the last wrote.
//!   The longer lifetime comes first, as the API requires.
//!
//! What would silently break it is anything that changes the prefix: a
//! timestamp or a user's name in the system prompt, a tool list that varies
//! per user, an edited earlier message. The prompt is a constant, the tools
//! are one list, and messages are frozen as JSON text once ([`Message::raw`])
//! and never re-serialised.
//!
//! ## Thinking and effort
//!
//! Thinking is always on for this model; `effort` is the control, and the
//! caller chooses it ([`Settings`]). `display` is `updates`, so the short notes
//! the model writes between tool calls arrive as text — the progress lines the
//! web app shows — while its reasoning stays hidden.
//!
//! [`Message::raw`]: crate::api::anthropic::request::Message::raw

mod assemble;

use std::io::BufRead;

use serde_json::Value;
use serde_json::value::RawValue;

pub use assemble::Assembler;

use crate::api::anthropic::Messages;
use crate::api::anthropic::content::Block;
use crate::api::anthropic::request::{
    CacheControl, Display, Effort, OutputConfig, Request, SystemText, TextKind, Thinking,
    ThinkingKind, Tool, Ttl,
};
use crate::api::anthropic::stream::{StreamError, events};
use crate::api::http::HttpError;
use crate::credentials::Secret;
use crate::prices::claude::Usage;

pub use crate::prices::claude::MODEL;

/// The most one reply may be, thinking included. Streamed replies can be
/// long; a bound sized for a reply without thinking cuts replies off, and
/// Anthropic's guidance for long agentic turns on this model is 64K.
pub const MAX_TOKENS: u32 = 64_000;

/// What a caller decides about every request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The model id.
    pub model: String,
    /// How hard it thinks.
    pub effort: Effort,
    /// The most one reply may be.
    pub max_tokens: u32,
}

/// A request laid out for caching (see the module doc): `tools` and `system`
/// as the cached prefix, `messages` — each already frozen — after it.
pub fn request(
    settings: &Settings,
    system: &str,
    tools: Vec<Tool>,
    messages: Vec<Box<RawValue>>,
) -> Request {
    Request {
        model: settings.model.clone(),
        max_tokens: settings.max_tokens,
        stream: true,
        thinking: Thinking {
            kind: ThinkingKind::Adaptive,
            display: Display::Updates,
        },
        output_config: OutputConfig {
            effort: settings.effort,
        },
        cache_control: CacheControl::lasting(Ttl::FiveMinutes),
        tools,
        system: vec![SystemText {
            kind: TextKind::Text,
            text: system.to_owned(),
            cache_control: Some(CacheControl::lasting(Ttl::OneHour)),
        }],
        messages,
    }
}

/// Something that answers a request — Anthropic's API, or a test's script.
pub trait Claude: Send + Sync {
    /// Send `request` and wait for the whole reply, telling `on` about its
    /// text and progress notes as they stream in.
    fn reply(
        &self,
        request: &Request,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<Response, ClaudeError>;
}

/// A piece of a reply, as it arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Streamed<'a> {
    /// More of a text block.
    Text(&'a str),
    /// More of a progress note (a thinking block's visible text).
    Progress(&'a str),
    /// The block being written is complete.
    BlockEnd,
}

/// One whole reply.
#[derive(Debug, Clone)]
pub struct Response {
    /// The message id Anthropic gave it.
    pub id: String,
    /// The model that answered, as the API names it — what it is charged at.
    pub model: String,
    /// Its blocks, in order: what is sent back, unchanged, as the assistant's
    /// message.
    pub content: Vec<Block>,
    /// Why it stopped.
    pub stop: Stop,
    /// The tokens it was billed for.
    pub usage: Usage,
}

/// A tool call out of a reply, its arguments parsed for running it.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// The call's id, which its result names.
    pub id: String,
    /// The tool.
    pub name: String,
    /// Its arguments.
    pub input: Value,
}

impl Response {
    /// Its text blocks, joined — what the model said in words.
    pub fn text(&self) -> String {
        let texts: Vec<&str> = self
            .content
            .iter()
            .filter_map(|block| match block {
                Block::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .filter(|text| !text.trim().is_empty())
            .collect();
        texts.join("\n\n")
    }

    /// The tools it calls, in order.
    pub fn calls(&self) -> Vec<Call> {
        self.content
            .iter()
            .filter_map(|block| match block {
                Block::ToolUse { id, name, input } => Some(Call {
                    id: id.clone(),
                    name: name.clone(),
                    // Checked as JSON when the block was assembled.
                    input: serde_json::from_str(input.get()).unwrap_or(Value::Null),
                }),
                _ => None,
            })
            .collect()
    }
}

/// Why a reply stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// It is finished: the answer.
    EndTurn,
    /// It wants tools run and their results sent back.
    ToolUse,
    /// It reached [`Settings::max_tokens`] before finishing.
    MaxTokens,
    /// A safety classifier declined it. A turn that ended without an answer,
    /// not a failure of the call.
    Refusal {
        /// The classifier's category, e.g. `cyber`.
        category: Option<String>,
        /// Why, in words.
        explanation: Option<String>,
    },
    /// Any other reason, by the API's name for it.
    Other(String),
}

impl Stop {
    /// The API's `stop_reason`.
    pub fn as_str(&self) -> &str {
        match self {
            Self::EndTurn => "end_turn",
            Self::ToolUse => "tool_use",
            Self::MaxTokens => "max_tokens",
            Self::Refusal { .. } => "refusal",
            Self::Other(reason) => reason,
        }
    }
}

/// Why a call produced no reply.
#[derive(Debug, thiserror::Error)]
pub enum ClaudeError {
    /// The request was refused or never arrived.
    #[error(transparent)]
    Http(#[from] HttpError),
    /// The stream broke or carried something unreadable.
    #[error(transparent)]
    Stream(#[from] StreamError),
    /// The API sent an `error` event partway.
    #[error("Anthropic's API failed partway ({kind}): {message}")]
    Api {
        /// E.g. `overloaded_error`.
        kind: String,
        /// In words.
        message: String,
    },
    /// The stream ended before `message_stop`.
    #[error("the reply ended before it was complete")]
    Cut,
    /// A tool call's arguments were not JSON.
    #[error("a tool call's arguments are not JSON: {0}")]
    Arguments(String),
}

impl ClaudeError {
    /// Whether the same request is worth sending again: the API was busy,
    /// failed on its side, or the connection did — never a request it
    /// refused for what it said.
    pub fn retryable(&self) -> bool {
        match self {
            Self::Http(HttpError::Unreachable { .. }) | Self::Stream(StreamError::Io(_)) => true,
            Self::Http(HttpError::Refused { status, .. }) => {
                matches!(status, 408 | 409 | 429 | 500..=504 | 529)
            }
            Self::Api { kind, .. } => {
                matches!(
                    kind.as_str(),
                    "overloaded_error" | "api_error" | "rate_limit_error"
                )
            }
            Self::Cut => true,
            Self::Http(_) | Self::Stream(_) | Self::Arguments(_) => false,
        }
    }
}

/// Anthropic's API, with one key.
#[derive(Debug, Clone)]
pub struct Anthropic {
    messages: Messages,
}

impl Anthropic {
    /// A client spending `key`.
    pub fn new(key: &Secret) -> Self {
        Self {
            messages: Messages::new(key),
        }
    }

    /// The same client, copying every reply into `tap` — for the live
    /// provider check ([`crate::live`]); see [`crate::api::tap`].
    pub fn tapped(mut self, tap: &crate::api::tap::Tap) -> Self {
        self.messages = self.messages.tapped(tap);
        self
    }
}

impl Claude for Anthropic {
    fn reply(
        &self,
        request: &Request,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<Response, ClaudeError> {
        let mut assembler = Assembler::default();
        for event in self.messages.stream(request)? {
            if let Some(done) = assembler.feed(event?, on)? {
                return Ok(done);
            }
        }
        Err(ClaudeError::Cut)
    }
}

/// A whole reply out of a recorded stream — what a fixture test reads.
pub fn replay(
    stream: impl BufRead,
    on: &mut dyn FnMut(Streamed<'_>),
) -> Result<Response, ClaudeError> {
    let mut assembler = Assembler::default();
    for event in events(stream) {
        if let Some(done) = assembler.feed(event?, on)? {
            return Ok(done);
        }
    }
    Err(ClaudeError::Cut)
}
