//! The web app's assistant (#540): Claude editing a user's project with
//! scorsese's tools, while the browser watches.
//!
//! A user writes "make an intro for my channel". A **turn** begins: the
//! server sends the conversation to Claude Opus 5.5 with scorsese's whole
//! tool surface — the same [`Toolbox`](crate::tools::Toolbox) web MCP serves,
//! called in-process as [`Client::Assistant`](crate::tools::Client) — runs
//! every tool Claude asks for against that user's project and library, sends
//! the results back, and loops until Claude answers (`turn`). Everything it
//! does is on the user's event stream as it happens (`crate::events`): its
//! words, its short progress notes, each tool call and how it answered, a
//! project's new revision, what the turn has cost and the balance left.
//!
//! ## Choosing Claude does not make the tools Claude's
//!
//! The model is a decision about **this one client** (`CLAUDE.md`, *The web
//! app's built-in assistant is Claude*). The tool surface it calls is the one
//! a user's own Gemini or GPT gets over web MCP, word for word: nothing in
//! `crate::tools` knows who is calling, and this module adds nothing to it.
//! What it adds is around the tools — a system prompt, the conversation, the
//! quote box — never a tool of its own.
//!
//! ## The conversation is append-only, and stored as the text that was sent
//!
//! A project has conversations (`store`: `chat_sessions`), each a run of
//! turns; a turn's messages are kept as the exact JSON text first sent, and
//! the next turn's history is every earlier turn's messages, concatenated.
//! Nothing is ever edited or re-serialised: on this model a thinking block is
//! valid only while everything before it is byte-for-byte unchanged, and an
//! unchanged prefix is also what the prompt cache reads
//! (`scorsese_providers::claude` has the caching layout). Facts the server
//! vouches for — which project this is, that the user confirmed a quote —
//! arrive as mid-conversation `system` messages, which neither the user nor a
//! tool's output can forge.
//!
//! ## Paid tools: the user's yes, never the model's
//!
//! `generate` quotes before it spends and spends only when called again with
//! the quote's token (#538). Handed to a model, that token would let it
//! confirm its own quote. So the model **never sees one** (`calls`): when a
//! call issues a quote, the token is cut out of the reply the model reads, the
//! quote is held on the turn and shown to the user as a confirmation box
//! (`chat_quote`), and a call that names `confirm` is refused outright. The
//! user's yes (`POST /api/chat/turns/{id}/quote`, `quote`) is what spends —
//! the server makes that call itself, recorded as the user's — and a new turn
//! then tells the model, as a system message, what the spend did. A no, or
//! a new message instead of an answer, withdraws the token.
//!
//! ## Money
//!
//! Every call to Claude is charged from the tokens its reply reports — input,
//! output, both cache writes, cache reads — at cost plus 10%, against the turn
//! (`credits::ledger::charge_assistant`). Not reserved for: the cost exists
//! only once counted. So a turn is **refused up front at a balance of zero or
//! less**, and stops between calls once the balance runs out or the turn has
//! cost the operator's **per-turn cap** ([`Assistant::cap_micros`]). The call
//! that crosses either line has already been made and is charged; one call's
//! cost is bounded by `max_tokens`.
//!
//! ## Effort: `high`
//!
//! Effort is the one setting that trades the model's quality for credits, and
//! the maintainer's rule is not to make that trade (#527). Claude Opus 5.5's
//! API default is `medium`, the cost-saving level; editing a video is long,
//! many-step tool work, which is where Anthropic's guidance puts `high`; and
//! `xhigh` or `max` are for gains somebody has measured, which nobody has yet.
//! Measure with real turns (#567) before moving it.

mod calls;
mod prompt;
mod quote;
mod relay;
mod start;
mod store;
mod turn;

use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};

use scorsese_providers::api::anthropic::request::Effort;
use scorsese_providers::claude::{Anthropic, Claude, MAX_TOKENS, MODEL, Settings};
use scorsese_providers::credentials::{Provider, resolve};

pub use quote::{Answered, answer as answer_quote};
pub use start::{Opening, start};
pub use store::{Conversation, QuoteView, ToolCallView, TurnDetail, TurnView};
pub use store::{conversation, detail, recover};

/// What a turn is not allowed to cost by default, in micro-dollars: $2.
pub const DEFAULT_TURN_CAP_MICROS: i64 = 2_000_000;

/// The effort every call is made at; the module doc argues it.
pub const EFFORT: Effort = Effort::High;

/// The server's assistant: how it reaches Claude, what it runs, what a turn
/// may cost, and which turns have been asked to stop. Cheap to clone.
#[derive(Clone)]
pub struct Assistant {
    connect: Connect,
    settings: Settings,
    cap_micros: i64,
    stopping: Arc<Mutex<HashSet<i64>>>,
}

/// Where the Claude client comes from.
#[derive(Clone)]
enum Connect {
    /// Anthropic's API, with `ANTHROPIC_API_KEY` from the one credentials
    /// resolver, looked up when a turn starts.
    Environment,
    /// This one — a test's script.
    Fixed(Arc<dyn Claude>),
    /// None: every turn is refused as not configured.
    Unconfigured,
}

/// Why a turn could not start, or a quote could not be answered.
#[derive(Debug, thiserror::Error)]
pub enum AssistantError {
    /// The server has no key for Claude.
    #[error("the assistant is not configured on this server: {0}")]
    NotConfigured(String),
    /// The balance is zero or less.
    #[error("your balance is {0}; add credit to use the assistant")]
    NoCredit(String),
    /// A turn is already running in this conversation.
    #[error("the assistant is still working on your last message; wait for it, or stop it")]
    Busy,
    /// No such project, turn or quote of theirs.
    #[error("not found")]
    NotFound,
    /// The request itself is wrong.
    #[error("{0}")]
    Invalid(String),
    /// Something on the server's side failed; the detail is for the log.
    #[error("{0}")]
    Internal(String),
    /// The database refused.
    #[error("the database refused: {0}")]
    Database(#[from] sqlx::Error),
}

impl Default for Assistant {
    fn default() -> Self {
        Self::new(MODEL, DEFAULT_TURN_CAP_MICROS)
    }
}

impl Assistant {
    /// The real assistant: Claude on `model`, a turn capped at `cap_micros`.
    pub fn new(model: &str, cap_micros: i64) -> Self {
        Self {
            connect: Connect::Environment,
            settings: Settings {
                model: model.to_owned(),
                effort: EFFORT,
                max_tokens: MAX_TOKENS,
            },
            cap_micros,
            stopping: Arc::default(),
        }
    }

    /// The same assistant answering from `claude` instead of Anthropic.
    pub fn answered_by(mut self, claude: Arc<dyn Claude>) -> Self {
        self.connect = Connect::Fixed(claude);
        self
    }

    /// The same assistant with no Claude at all.
    pub fn unconfigured(mut self) -> Self {
        self.connect = Connect::Unconfigured;
        self
    }

    /// The most one turn may cost, in micro-dollars.
    pub fn cap_micros(&self) -> i64 {
        self.cap_micros
    }

    /// Ask turn `turn` to stop before its next step.
    pub fn stop(&self, turn: i64) {
        self.stopping().insert(turn);
    }

    /// The client a turn talks to, or why there is none.
    fn claude(&self) -> Result<Arc<dyn Claude>, AssistantError> {
        match &self.connect {
            Connect::Fixed(claude) => Ok(claude.clone()),
            Connect::Unconfigured => Err(AssistantError::NotConfigured(
                "no Claude client is set up".into(),
            )),
            Connect::Environment => match resolve(Provider::Anthropic) {
                Ok(key) => Ok(Arc::new(Anthropic::new(&key.secret))),
                // Where it looked names the server's own paths: the log's.
                Err(error) => {
                    eprintln!("scorsese-server: assistant: {error}");
                    Err(AssistantError::NotConfigured(format!(
                        "{} is not set",
                        Provider::Anthropic.variable()
                    )))
                }
            },
        }
    }

    /// Whether `turn` was asked to stop; forgets the ask.
    fn stop_requested(&self, turn: i64) -> bool {
        self.stopping().remove(&turn)
    }

    fn stopping(&self) -> std::sync::MutexGuard<'_, HashSet<i64>> {
        self.stopping.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
