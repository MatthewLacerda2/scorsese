//! The web app's assistant (#540): a model editing a user's project with
//! scorsese's tools, while the browser watches.
//!
//! A user writes "make an intro for my channel". A **turn** begins: the
//! server sends the conversation to the model the project has chosen (#705,
//! *Which model* below) with scorsese's whole tool surface — the same
//! [`Toolbox`](crate::tools::Toolbox) web MCP serves, called in-process as
//! [`Client::Assistant`](crate::tools::Client) — runs every tool the model
//! asks for against that user's project and library, sends the results back,
//! and loops until it answers (`turn`). Everything it
//! does is on the user's event stream as it happens (`crate::events`): its
//! words, each tool call and how it answered, a
//! project's new revision, what the turn has cost and the balance left.
//!
//! ## Which model
//!
//! A project names its model (`projects.assistant_model`): one of the three
//! `scorsese_providers::chat::Model` offers — Claude Opus 5.5 by default
//! (#1444), Claude Sonnet 5.5 or Claude Fable 5.1 — and its owner changes it whenever they like, mid-conversation too (`model`). Every
//! turn runs on the model its project names when it starts, and is charged at
//! that model's rates. This module talks to `scorsese_providers::chat` and
//! never to a vendor: what differs between them is behind that seam. Gemini
//! 3.8 Flash and 3.5 Flash Lite were offered too until 2026-10-12, when the
//! maintainer cut them as not good enough at the job (#1220).
//!
//! Choosing a model for **this one client** does not make the tools that
//! model's (`CLAUDE.md`, *The web app's built-in assistant runs on the model
//! each project picks*). The tool surface
//! it calls is the one a user's own Gemini or GPT gets over web MCP, word for
//! word: nothing in `crate::tools` knows who is calling, and this module adds
//! nothing to it. What it adds is around the tools — a system prompt, the
//! conversation, the quote box — never a tool of its own.
//!
//! ## The conversation is append-only, and stored as the text that was sent
//!
//! A project has conversations (`store`: `chat_sessions`), each a run of
//! turns. A turn's messages are kept twice: as the exact JSON text its model
//! was first sent (`messages`), and as a vendor-neutral record (`record`). The
//! next turn's history is every earlier turn, in order — its own bytes when it
//! ran on the same model, its record translated otherwise
//! (`scorsese_providers::chat::replay`). Nothing is ever edited or
//! re-serialised: on Claude a thinking block is valid only while everything
//! before it is byte-for-byte unchanged, and an unchanged prefix is also what
//! the prompt cache reads. A change of model costs one uncached
//! turn, and the web app says so before it switches.
//!
//! Except that a long conversation compacts itself (#1438, `compact`): a new
//! turn that finds it past 150K tokens opens from a summary the model wrote
//! of everything before, and the history starts at that turn
//! (`chat_turns.compacted`). Nothing is edited even then: the older turns stay
//! stored as they were sent, and only where the history begins moves. Facts the server vouches
//! for — which project this is, that the user confirmed a quote — arrive as
//! server notes (a mid-conversation `system` message), which neither the user nor a tool's output can forge.
//!
//! ## Paid tools: the user's yes, never the model's
//!
//! `generate` quotes before it spends and spends only when called again with
//! the quote's token (#538). Handed to a model, that token would let it
//! confirm its own quote. So the model **never sees one** (`calls`): when a
//! call issues a quote, the token is cut out of the reply the model reads, the
//! quote is held on the turn and shown to the user as a confirmation box
//! (`chat_quote`), and a call that names `confirm` is refused outright. The
//! quote **pauses the turn** (#1440), as a question does (*A question
//! mid-edit* below), and the user's answer (`POST /api/chat/turns/{id}/quote`,
//! `quote`) resumes it as the result of the call that quoted. A yes is what
//! spends — the server makes that call itself, recorded as the user's — and
//! the result is what the spend answered, so the model carries on in the same
//! turn. A no, a change the user asks for instead (#709: the result carries
//! their words as a change to the quoted items), or a new message instead of
//! an answer, withdraws the token. The box shows each item's description
//! beside its price (`described`), so a yes is never to money alone.
//!
//! ## When the work lands, it carries on
//!
//! A generation takes minutes. When the last of the jobs a turn's yes ordered
//! has ended, the server starts the next turn itself (#1441, `wake`), with a
//! note saying what landed — unless the person wrote meanwhile, whose next
//! message carries the note instead. It is an ordinary turn: charged for its
//! tokens, paused by a quote, never spending on its own.
//!
//! ## A question mid-edit
//!
//! Beside the tools, the loop declares one function of its own to the model,
//! `ask_user` (#710, `ask`): one short question with two to four options. It
//! is **not a tool** — never in `crate::tools`, so web MCP never lists it —
//! and calling it **pauses the turn** (`asking`) until the user answers,
//! from the question's card or by simply writing their next message. The
//! answer resumes the same turn as that call's result, so the model carries
//! on with its plan intact. Nothing is charged while it waits, and the
//! question does not expire; Stop or a new conversation sets it aside.
//!
//! The same pause shows stock pictures instead of words (#901, `pick_stock`):
//! a few results the turn's own searches found, for the user to pick one or
//! more of when the choice is taste. What they pick is imported as their own
//! call, from the toolbox's stock library (`crate::tools`, *Stock*) — the
//! one the model's own `stock_search` and `stock_import` answer from — and the
//! turn resumes with the new assets, or for a Lottie (#908) the file under
//! `pages/` and the page that plays it.
//!
//! ## Money
//!
//! Every call to the model is charged from the tokens its reply reports —
//! input, output, cache writes, cache reads, each at the model's own rate for
//! it, so a cache hit is cheaper for the user too — at cost × 1.8
//! (`credits::Product::Assistant`), against the turn
//! (`credits::ledger::charge_assistant`). Not reserved for: the cost exists
//! only once counted. So a turn is **refused up front at a balance of zero or
//! less** — the balance the assistant knows, paid credit and the free
//! allowance together (`credits::ledger::spendable`, #1530) — and stops between calls once the balance runs out or the turn has
//! cost the operator's **per-turn cap** ([`Assistant::cap_micros`]). The cap
//! is set for Sonnet 5.5 and scales with the turn's model's blended rate
//! (`cost`, #1461), so a turn may do the same work on any model. The call
//! that crosses either line has already been made and is charged; one call's
//! cost is bounded by `max_tokens`.
//!
//! ## Effort: the project's, `high` unless chosen
//!
//! Effort trades a model's quality for credits within the model the user
//! chose. #717 fixed it at `high` on the grounds that the model is where that
//! trade is made; but one project gets both "nudge the title" and "build the
//! whole cut", and paying `high` for a nudge is wasted credit (#769). So a
//! project keeps a level beside its model (`projects.assistant_effort`,
//! `effort`, #1443), which the chat panel offers beside the model picker as
//! low, medium, high or extra (`xhigh` on the wire). A new project's is
//! `high`: editing a video is long, many-step tool work, which is where
//! Anthropic's guidance puts it. Every turn records its effort, and a turn
//! that resumes — after a question (#710) or a quote's answer — keeps the one
//! it started with. `max` is for gains somebody has measured, which nobody has
//! yet (#567).
//!
//! A change of level costs no cache: the request's top-level effort never
//! moves, and a turn at another level opens with an effort change in the
//! conversation (`scorsese_providers::chat`, *Effort is part of the
//! conversation*).
//!
//! ## Thinking is hidden
//!
//! Both vendors stream a model's thinking as short notes (Claude's `updates`,
//! Gemini's thought summaries); the provider seam still hands them over as
//! `Streamed::Progress`, but the relay drops them and the browser never hears
//! them (#767). Nobody read them, there were too many to read, and they put
//! the model's working in front of a user who never deals with
//! implementation. The thinking itself still happens and is still billed as
//! output: hiding it saves reading, not credits. The vendors are still asked
//! for the notes, because what they ask for is part of each turn's stored
//! bytes and cached prefix (*The conversation is append-only* above), and
//! changing that buys nothing a user can see.

mod ask;
mod calls;
mod compact;
mod cost;
mod described;
mod effort;
mod failure;
mod model;
mod pictures;
mod prompt;
mod quote;
mod relay;
mod retarget;
mod start;
mod stopping;
mod store;
mod turn;
mod wake;

use std::sync::Arc;

use scorsese_providers::chat::{self, Chat, Model};
use scorsese_providers::credentials::resolve;
use scorsese_providers::prices;

pub use ask::{Answering, answer as answer_question, set_aside};
pub use effort::choose_effort;
pub use model::{Choice, choose};
pub use quote::{Answer, Answered, answer as answer_quote};
pub use retarget::{Retargeted, retarget};
pub use start::{Opening, start};
pub use store::{
    BriefKind, CandidateView, Conversation, QuestionView, QuoteItem, QuoteReply, QuoteView,
    ToolCallView, TurnDetail, TurnView,
};
pub use store::{conversation, detail, recover};
pub use wake::watch;

/// What a turn on Sonnet 5.5 is not allowed to cost by default, in
/// micro-dollars: $3.50. The server's one statement of it:
/// [`crate::config::ASSISTANT_TURN_CAP`] falls back to it, and that
/// variable's error names it in dollars. Every other model's cap is this
/// times its blended rate over Sonnet's ([`Assistant::cap_micros`], #1461):
/// about $7 on Opus 5.5, $17.50 on Fable 5.1.
///
/// What a user is charged, so it moved with the multipliers (#1247): six
/// turns on the hosted server (2026-10-13) averaged $0.74 (Gemini 3.8 Flash)
/// to $1.02 (Sonnet 5.5) of cost, $1.33 to $1.84 at ×1.8 — an average Sonnet
/// turn 16 cents under the old $2.00. $3.50 lets a turn spend about what
/// $2.00 did at ×1.1, $1.94 of cost against $1.82.
pub const DEFAULT_TURN_CAP_MICROS: i64 = 3_500_000;

/// The server's assistant: how it reaches each model, what a turn may cost,
/// and which turns have been asked to stop. Cheap to clone.
#[derive(Clone)]
pub struct Assistant {
    connect: Connect,
    baseline_micros: i64,
    stopping: stopping::Stopping,
}

/// Where a model's client comes from.
#[derive(Clone)]
enum Connect {
    /// The vendor's API, with its key from the one credentials resolver,
    /// looked up when a turn starts.
    Environment,
    /// This one, for every model — a test's script.
    Fixed(Arc<dyn Chat>),
    /// None: every turn is refused as not configured.
    Unconfigured,
}

/// Why a turn could not start, or a quote could not be answered.
#[derive(Debug, thiserror::Error)]
pub enum AssistantError {
    /// The server has no key for the project's model.
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
        Self::new(DEFAULT_TURN_CAP_MICROS)
    }
}

impl Assistant {
    /// The real assistant: each model through its vendor's API, a turn on
    /// Sonnet 5.5 capped at `baseline_micros` and one on any other model at
    /// that scaled to its rates.
    pub fn new(baseline_micros: i64) -> Self {
        Self {
            connect: Connect::Environment,
            baseline_micros,
            stopping: stopping::Stopping::default(),
        }
    }

    /// The same assistant answering every model from `chat` instead.
    pub fn answered_by(mut self, chat: Arc<dyn Chat>) -> Self {
        self.connect = Connect::Fixed(chat);
        self
    }

    /// The same assistant with no model at all.
    pub fn unconfigured(mut self) -> Self {
        self.connect = Connect::Unconfigured;
        self
    }

    /// The most one turn on `model` may cost, in micro-dollars, at today's
    /// rates (#718): the Sonnet 5.5 baseline times `model`'s blended rate over
    /// Sonnet's (`cost`, #1461).
    pub fn cap_micros(&self, model: Model) -> i64 {
        let today = prices::Checked::today();
        cost::cap(self.baseline_micros, model, |model| {
            prices::chat::rate(model, today)
        })
    }

    /// Ask turn `turn` to stop before its next step, and stop the tool call
    /// it is running (#1469).
    pub fn stop(&self, turn: i64) {
        self.stopping.ask(turn);
    }

    /// The client a turn on `model` talks to, or why there is none — in the
    /// words the user is shown, naming the missing key (#705: a stack let in
    /// without its keys is a setup mistake that should be loud).
    fn chat(&self, model: Model) -> Result<Arc<dyn Chat>, AssistantError> {
        match &self.connect {
            Connect::Fixed(chat) => Ok(chat.clone()),
            Connect::Unconfigured => Err(AssistantError::NotConfigured(
                "no model client is set up".into(),
            )),
            Connect::Environment => {
                let provider = model.vendor().provider();
                match resolve(provider) {
                    Ok(key) => Ok(chat::client(model.vendor(), &key.secret)),
                    // Where it looked names the server's own paths: the log's.
                    Err(error) => {
                        eprintln!("scorsese-server: assistant: {error}");
                        Err(AssistantError::NotConfigured(format!(
                            "{} is not set, so {} cannot answer",
                            provider.variable(),
                            model.label()
                        )))
                    }
                }
            }
        }
    }

    /// Whether a turn on `model` could start now, as far as keys go.
    fn available(&self, model: Model) -> bool {
        match &self.connect {
            Connect::Fixed(_) => true,
            Connect::Unconfigured => false,
            Connect::Environment => resolve(model.vendor().provider()).is_ok(),
        }
    }

    /// Whether `turn` was asked to stop; forgets the ask.
    fn stop_requested(&self, turn: i64) -> bool {
        self.stopping.requested(turn)
    }
}
