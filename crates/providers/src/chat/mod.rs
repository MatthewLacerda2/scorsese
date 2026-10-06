//! The assistant's model, whichever vendor serves it (#705).
//!
//! The hosted web app's assistant runs on the model its project names — one of
//! [`Model::ALL`], two from Anthropic and two from Google. Everything that
//! drives a turn talks to this module and to nothing vendor-shaped: one
//! streamed call in, one [`Reply`] out, behind a trait ([`Chat`]) so that no
//! test ever makes the call.
//!
//! # Seam
//!
//! What a caller holds, and what stays behind the line:
//!
//! - **[`Request`]**: the model, how hard it thinks ([`Effort`]), the system
//!   prompt, the tools (name, description, JSON Schema — the registry's own),
//!   and the conversation **already in the model's own wire**, one frozen
//!   message per entry. Building that conversation is [`replay`]'s job, below.
//! - **[`Reply`]**: what the model said as a vendor-neutral [`Message`] (words
//!   and tool calls), the same message as the exact bytes to send back
//!   ([`Reply::native`]), why it stopped ([`Stop`]), and the tokens it was
//!   billed for as a vendor-neutral [`Usage`] — the five kinds both vendors'
//!   counts reduce to, priced by one table ([`crate::prices::chat`]). A
//!   caller recording a call (#707) needs nothing vendor-shaped to do it.
//! - **[`Streamed`]**: the reply's words and progress notes as they arrive —
//!   Claude's thinking `updates` and Gemini's thought summaries are both
//!   progress notes.
//! - **Behind the line**: each vendor's wire ([`crate::api::anthropic`],
//!   [`crate::api::gemini::chat`]), its caching layout, its thinking
//!   settings, its token limit. [`anthropic`] and [`gemini`] own those.
//!
//! # A conversation survives a change of model
//!
//! A project's model can be changed between any two turns, so a conversation
//! is kept twice ([`record`]): every message as the bytes its model was sent
//! (byte-for-byte replay is what Claude's thinking blocks and every prompt
//! cache need), and as a neutral [`Message`]. [`replay`] builds a request's
//! history turn by turn — a turn that ran on the model being asked replays its
//! own bytes; any other is translated from its record by [`freeze`], which is
//! deterministic, so the same history translates to the same bytes every time
//! and the cache keeps working from the second turn after a switch on.
//!
//! What a switch loses is what only one vendor understands: Claude's thinking
//! blocks and Gemini's thought signatures stay with the bytes of the model
//! that wrote them, and the first turn on the new model reads its whole
//! history uncached. That costs more once, and the web app warns before it.

pub mod anthropic;
pub mod gemini;
mod model;
pub mod record;

use std::sync::Arc;

use serde_json::Value;
use serde_json::value::RawValue;

pub use crate::claude::{ClaudeError, Stop, Streamed};
pub use crate::prices::chat::Usage;
pub use model::{Model, Vendor};
pub use record::{Call, Message, Part, ResultPart};

use crate::credentials::Secret;

/// How hard a model thinks — Claude's `effort`, Gemini's `thinkingLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effort {
    /// The least thinking every model offered accepts.
    Low,
    /// The middle.
    Medium,
    /// More thinking: long, many-step tool work.
    High,
}

impl Effort {
    /// As both vendors spell it, and as a turn records it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// The level `name` spells, as [`Self::as_str`] writes it; `None` for
    /// anything else.
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Low, Self::Medium, Self::High]
            .into_iter()
            .find(|effort| effort.as_str() == name)
    }
}

/// One tool the model may call: the registry's own entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    /// What the model calls it by.
    pub name: String,
    /// What it does, in words for the model.
    pub description: String,
    /// Its arguments, as JSON Schema.
    pub schema: Value,
}

/// One call to a model.
#[derive(Debug, Clone)]
pub struct Request {
    /// Which model.
    pub model: Model,
    /// How hard it thinks.
    pub effort: Effort,
    /// The instructions. Constant across users and turns: it leads every
    /// vendor's cached prefix.
    pub system: String,
    /// What it may call, in a fixed order — also part of the cached prefix.
    pub tools: Vec<Tool>,
    /// The conversation so far, each message frozen in **this model's**
    /// wire ([`replay`], [`freeze`]).
    pub messages: Vec<Box<RawValue>>,
}

/// One whole reply.
#[derive(Debug, Clone)]
pub struct Reply {
    /// The model that answered, as the vendor names it.
    pub model: String,
    /// What it said, as no vendor in particular writes it.
    pub message: Message,
    /// The same message as the exact bytes sent back to this model next call.
    pub native: Box<RawValue>,
    /// Why it stopped.
    pub stop: Stop,
    /// The tokens it was billed for.
    pub usage: Usage,
    /// How many of `usage.output` were thinking, when the vendor says so
    /// apart: Gemini does (`thoughtsTokenCount`), Anthropic's `usage` does not
    /// — its output count includes the thinking, undivided — so `None` there.
    pub thinking: Option<u64>,
}

impl Reply {
    /// Its words, joined.
    pub fn text(&self) -> String {
        self.message.text()
    }

    /// The tools it calls, in order.
    pub fn calls(&self) -> Vec<Call> {
        self.message.calls()
    }
}

/// Something that answers a request — a vendor's API, or a test's script.
pub trait Chat: Send + Sync {
    /// Send `request` and wait for the whole reply, telling `on` about its
    /// words and progress notes as they stream in.
    fn reply(
        &self,
        request: &Request,
        on: &mut dyn FnMut(Streamed<'_>),
    ) -> Result<Reply, ChatError>;
}

/// Why a call produced no reply.
#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    /// Anthropic's side.
    #[error(transparent)]
    Claude(#[from] ClaudeError),
    /// Google's side.
    #[error(transparent)]
    Gemini(#[from] gemini::GeminiError),
    /// The request named a model this client does not serve.
    #[error("{0} is not served by this client")]
    Misrouted(&'static str),
}

impl ChatError {
    /// Whether the same request is worth sending again: the vendor was busy,
    /// failed on its side, or the connection did.
    pub fn retryable(&self) -> bool {
        match self {
            Self::Claude(error) => error.retryable(),
            Self::Gemini(error) => error.retryable(),
            Self::Misrouted(_) => false,
        }
    }
}

/// The client for `vendor`, spending `key`.
pub fn client(vendor: Vendor, key: &Secret) -> Arc<dyn Chat> {
    match vendor {
        Vendor::Anthropic => Arc::new(crate::claude::Anthropic::new(key)),
        Vendor::Google => Arc::new(gemini::Gemini::new(key)),
    }
}

/// `message` as `model`'s wire writes it, frozen: the bytes it is sent as from
/// now on.
pub fn freeze(model: Model, message: &Message) -> Result<Box<RawValue>, serde_json::Error> {
    match model.vendor() {
        Vendor::Anthropic => anthropic::freeze(message),
        Vendor::Google => gemini::freeze(message),
    }
}

/// One earlier turn, as it was kept.
#[derive(Debug, Clone)]
pub struct Kept {
    /// The model it ran on, by id.
    pub model: String,
    /// Its messages as that model was sent them.
    pub native: Vec<Box<RawValue>>,
    /// The same messages, neutral.
    pub record: Vec<Message>,
}

/// The history a turn on `model` continues: each earlier turn's own bytes
/// when it ran on `model`, and its record translated otherwise.
pub fn replay(model: Model, turns: &[Kept]) -> Result<Vec<Box<RawValue>>, serde_json::Error> {
    let mut messages = Vec::new();
    for turn in turns {
        if turn.model == model.id() {
            messages.extend(turn.native.iter().cloned());
        } else {
            for message in &turn.record {
                messages.push(freeze(model, message)?);
            }
        }
    }
    Ok(messages)
}
