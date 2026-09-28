//! What is sent: one Messages API request, and the pieces it is made of.
//!
//! Serialise-only. Field names are the API's own; the order a request
//! renders in — `tools`, then `system`, then `messages` — is the API's, not
//! this struct's, which is why the prompt-caching breakpoints are placed by
//! meaning (the last system block, the conversation's tail) rather than by
//! position here.

use serde::Serialize;
use serde_json::Value;
use serde_json::value::RawValue;

/// One call to `POST /v1/messages`.
#[derive(Debug, Clone, Serialize)]
pub struct Request {
    /// The model id, e.g. `claude-opus-5-5`.
    pub model: String,
    /// The most the reply may be, thinking included.
    pub max_tokens: u32,
    /// Always `true`: see the module doc of [`super`].
    pub stream: bool,
    /// How the model thinks, and what of it comes back.
    pub thinking: Thinking,
    /// How hard it thinks.
    pub output_config: OutputConfig,
    /// Automatic caching of the conversation's tail: the API puts this
    /// breakpoint on the last cacheable block and moves it as the
    /// conversation grows.
    pub cache_control: CacheControl,
    /// What the model may call. Rendered first, so it leads the cached prefix.
    pub tools: Vec<Tool>,
    /// The instructions, rendered after the tools.
    pub system: Vec<SystemText>,
    /// The conversation so far, each message **as the exact JSON text it was
    /// first sent as** — never re-serialised, because a replayed message must
    /// be byte-for-byte what the model saw (see [`Message::raw`]).
    pub messages: Vec<Box<RawValue>>,
}

/// `thinking`: always adaptive on Claude Opus 5.5 — it cannot be switched
/// off, and effort is the control — with `display` choosing what comes back.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Thinking {
    /// `adaptive`.
    #[serde(rename = "type")]
    pub kind: ThinkingKind,
    /// What the thinking blocks carry.
    pub display: Display,
}

/// The one kind of thinking the models scorsese runs accept.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingKind {
    /// The model decides how much to think; effort bounds it.
    Adaptive,
}

/// What a thinking block's text holds.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Display {
    /// Reasoning stays hidden; the short notes between tool calls come back
    /// as text. Needs the `thinking-display-updates-2026-08-18` beta.
    Updates,
}

/// `output_config`.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct OutputConfig {
    /// How much the model thinks, and so what a call costs.
    pub effort: Effort,
}

/// The effort levels the API names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    /// The least thinking.
    Low,
    /// Claude Opus 5.5's API default.
    Medium,
    /// More thinking than the default.
    High,
    /// More than `high`.
    Xhigh,
    /// Uncapped.
    Max,
}

impl Effort {
    /// As the API spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
}

/// A prompt-caching breakpoint.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CacheControl {
    /// Always `ephemeral`.
    #[serde(rename = "type")]
    pub kind: Ephemeral,
    /// How long the entry lives after its last use; five minutes if absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<Ttl>,
}

/// The one kind of cache entry there is.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ephemeral {
    /// Lives for its TTL after its last read.
    Ephemeral,
}

/// A cache entry's lifetime.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Ttl {
    /// Five minutes; writes cost 1.25× input.
    #[serde(rename = "5m")]
    FiveMinutes,
    /// An hour; writes cost 2× input.
    #[serde(rename = "1h")]
    OneHour,
}

impl CacheControl {
    /// A breakpoint living `ttl`.
    pub const fn lasting(ttl: Ttl) -> Self {
        Self {
            kind: Ephemeral::Ephemeral,
            ttl: Some(ttl),
        }
    }
}

/// One tool the model may call.
#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    /// What the model calls it by.
    pub name: String,
    /// What it does, in words for the model.
    pub description: String,
    /// Its arguments, as JSON Schema.
    pub input_schema: Value,
}

/// A block of the top-level `system` prompt.
#[derive(Debug, Clone, Serialize)]
pub struct SystemText {
    /// Always `text`.
    #[serde(rename = "type")]
    pub kind: TextKind,
    /// The words.
    pub text: String,
    /// A breakpoint here caches the tools and this prompt together.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// The type tag of a text block.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextKind {
    /// Text.
    Text,
}

/// Who a message is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The person — and the results of the tools the model called.
    User,
    /// The model.
    Assistant,
    /// The operator, mid-conversation: an instruction appended after the
    /// history instead of an edit to the top-level prompt, so the cached
    /// prefix stays whole. It cannot be written by the user or by a tool,
    /// which is what makes it the channel for facts the server vouches for.
    System,
}

/// One message of the conversation, before it is frozen into raw JSON.
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    /// Who it is from.
    pub role: Role,
    /// What it says.
    pub content: MessageContent,
}

/// A message's content: blocks, or — for an operator's note — plain text.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Content blocks.
    Blocks(Vec<super::content::Block>),
    /// Plain text: what a mid-conversation system message carries.
    Text(String),
}

impl Message {
    /// This message as the JSON text it will be sent as, once and for good.
    ///
    /// A conversation is replayed on every call, and on Claude Opus 5.5 a
    /// thinking block is valid only while everything before it is exactly
    /// what it was when the block was written — an edit to history is refused
    /// or silently drops the model's reasoning, and it misses the prompt cache
    /// either way. Freezing each message once, and storing and resending that
    /// text, makes the history append-only by construction.
    pub fn raw(&self) -> Result<Box<RawValue>, serde_json::Error> {
        serde_json::value::to_raw_value(self)
    }
}
