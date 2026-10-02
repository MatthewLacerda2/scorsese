//! The job queue: long work as rows in Postgres, run by the server's worker
//! and picked up again after a crash (#536).
//!
//! Renders take a while and Veo shots take minutes, on a home machine that can
//! lose power. Without a durable queue an interruption loses work silently —
//! and for Veo it can spend money twice. **No message broker** (#527): at a
//! handful of users a `jobs` table does the same job with one fewer container
//! to run and back up.
//!
//! ## The life of a job
//!
//! `waiting → running → done | failed | stuck | cancelled`. A feature enqueues one
//! ([`store::enqueue`], inside the caller's own scoped transaction, so a job
//! and whatever it belongs to are written together) and announces it
//! ([`Queue::announce`]). The worker ([`work`]) claims it, hands it to the
//! [`Handler`] its kind is registered with, and records the [`Outcome`].
//!
//! ## Claiming: privileged, and only the claim
//!
//! The worker serves every user, so *which job next* is cross-user by nature —
//! the same case as finding whose a cookie is, and it runs
//! [`privileged`](crate::db::privileged). Scoping it instead would mean asking
//! each user in turn, which is a slower way to write the same privileged
//! query. The privilege ends at the claim: it returns the owner, and
//! **everything after it runs [`scoped`](crate::db::scoped) as that user** —
//! keeping a ticket, finishing, whatever a handler reads through
//! [`Context::scoped`]. So the privileged list grows by two queries (the claim
//! and crash recovery) and a handler cannot touch another user's rows.
//!
//! The claim is `FOR UPDATE SKIP LOCKED`, so two claims never take one job;
//! it is **fair between users** — the job whose owner has the fewest running
//! goes first, oldest first among equals — so one person's batch of twenty
//! does not make everybody else wait behind all of it.
//!
//! ## Concurrency is per kind
//!
//! Each [`Kind`] declares how many may run at once ([`kinds`]). The machine
//! has four cores: a render gets a couple of slots and the rest wait in line,
//! visibly, while a Veo shot — minutes of waiting on Google — takes almost no
//! machine and gets more.
//!
//! ## After a crash
//!
//! **One worker per database**, held by a Postgres advisory lock, so a job
//! found `running` when the worker starts belongs to a process that is gone.
//! [`store::recover`] puts each back to `waiting` and stamps `interrupted_at`,
//! which is how the operator sees who a power cut affected
//! (`scorsese-server job interrupted`). A job interrupted [`MAX_ATTEMPTS`]
//! times is not tried again: it fails — or, holding a ticket, goes `stuck`.
//! A graceful stop takes the same path: the worker stops claiming and drops
//! what it was running, and the next start recovers it. One path for both
//! means the crash path runs on every deploy rather than only on bad days.
//!
//! ## Veo: never pay twice
//!
//! The ticket a provider hands back is the only record that money was spent,
//! and the design carries it the way `scorsese_providers::video` does for a
//! local project (its `run.rs`):
//!
//! - **The moment the provider accepts, the ticket is committed to the job's
//!   row** ([`Context::keep_ticket`]) — before the handler does anything else.
//!   A transaction cannot span Google, so "the same transaction" means nothing
//!   happens between the acceptance and the commit; the window left is the
//!   one a local project has too.
//! - **A job with a ticket polls; it never submits.** [`Job::ticket`] is what a
//!   recovered job arrives with, and a handler that holds one must only ask
//!   after it. Google keeps generating and bills either way, and the finished
//!   video stays fetchable for two days.
//! - **Patience is [`kinds::PROVIDER_PATIENCE`]**, fifteen minutes: shots have
//!   been seen taking ten, and the CLI's five is for a person at a terminal.
//!   Past it the job is [`Outcome::Stuck`] — not lost: the ticket is still in
//!   the row for a later collect.
//!
//! The Veo and ElevenLabs handlers pay through
//! [`credits::generations`](crate::credits::generations) (#537: reserve
//! before submitting, settle on the answer, a failure free), keep what they
//! make in the library ([`Library::keep_generated`](crate::library::Library::keep_generated),
//! #535) — [`crate::generations`], enqueued by web `generate` (#539). A
//! library item's thumbnail (#535) and a render ([`crate::renders::job`],
//! #541) have theirs too — see [`kinds::registry`].
//!
//! ## Stopping one
//!
//! Its owner may stop a render or a preview ([`kinds::STOPPABLE`], #660) —
//! nothing else: a paid generation is billed whether or not anybody still
//! wants it, and a thumbnail or proxy is the server's own housekeeping. A
//! waiting one is marked `cancelled` there and then and is never claimed. A
//! running one is stopped through the [`Cancel`] the worker gives every job
//! it runs ([`Context::cancel`]), held in the [`Queue`] by job id: the handler
//! sees it tripped, stops, and returns [`Outcome::Cancelled`] — recorded as
//! `cancelled`, not `failed`, because nothing went wrong. [`cancel`] is the
//! whole of it, for `POST /api/jobs/{id}/cancel` and web MCP's `job_cancel`
//! alike. A graceful stop of the worker trips every flag too, so a render
//! does not keep a core busy for an hour in a process that is shutting down.
//!
//! ## Live state
//!
//! Every change of state is pushed to the owner over [`crate::events`], the
//! stream the assistant (#540) reuses for its own events.

pub mod kinds;
mod registry;
mod stop;
pub mod store;
mod worker;

pub use registry::{Context, Handler, Registry};
pub use stop::{CancelError, cancel};
pub use worker::{Queue, work};

use serde::Serialize;
use serde_json::Value;

use crate::db::UserId;

/// How many times a job may be interrupted before recovery gives up on it.
///
/// A job that takes the process down with it — a render that runs the machine
/// out of memory — would otherwise be claimed, crash, recover and be claimed
/// again for ever.
pub const MAX_ATTEMPTS: i32 = 3;

/// A kind of job: which handler runs it, and how many may run at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kind {
    /// What the row's `kind` column says: lower snake case.
    pub name: &'static str,
    /// At most this many run at once.
    pub limit: usize,
}

/// Where a job is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// In line.
    Waiting,
    /// A worker has it.
    Running,
    /// Finished, with a result.
    Done,
    /// Did not work, and the error says why.
    Failed,
    /// Gave up waiting on a provider; the ticket is kept for a later collect.
    Stuck,
    /// Stopped by its owner, before or while it ran.
    Cancelled,
}

impl TryFrom<String> for State {
    type Error = String;

    fn try_from(state: String) -> Result<Self, String> {
        Ok(match state.as_str() {
            "waiting" => Self::Waiting,
            "running" => Self::Running,
            "done" => Self::Done,
            "failed" => Self::Failed,
            "stuck" => Self::Stuck,
            "cancelled" => Self::Cancelled,
            _ => return Err(format!("{state:?} is not a job state")),
        })
    }
}

/// A claimed job, as its handler receives it.
#[derive(Debug, Clone)]
pub struct Job {
    /// Its row.
    pub id: i64,
    /// Whose it is.
    pub user: UserId,
    /// Its kind's name.
    pub kind: String,
    /// What to do, in the kind's own terms.
    pub payload: Value,
    /// Times claimed, this one included.
    pub attempts: i32,
    /// The provider's ticket, if an earlier attempt was accepted. **Poll it;
    /// never submit again.**
    pub ticket: Option<String>,
}

/// How a handler's run ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// It worked; this is what it produced.
    Done(Value),
    /// It did not, and the message says why, in words for the job's owner.
    Failed(String),
    /// A provider outlasted [`kinds::PROVIDER_PATIENCE`]. The ticket stays.
    Stuck(String),
    /// Its owner stopped it ([`Context::cancel`]); the message says how far
    /// it got.
    Cancelled(String),
}

/// A job as its owner sees it: in `GET /api/jobs` and on the event stream.
#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct JobView {
    /// Its id.
    pub id: i64,
    /// Its kind's name.
    pub kind: String,
    /// Where it is.
    #[sqlx(try_from = "String")]
    pub state: State,
    /// Times claimed.
    pub attempts: i32,
    /// What it produced, once done.
    pub result: Option<Value>,
    /// Why it failed or is stuck, or how far it got before it was cancelled.
    pub error: Option<String>,
    /// When it was enqueued, in seconds since the Unix epoch.
    pub created_at: i64,
    /// When it was last claimed.
    pub started_at: Option<i64>,
    /// When it last stopped running: done, failed, stuck or cancelled.
    pub finished_at: Option<i64>,
    /// When a crash or restart last cut it off.
    pub interrupted_at: Option<i64>,
}
