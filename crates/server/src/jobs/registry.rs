//! Which handler runs which kind, and what a handler is given to run with.

use std::future::Future;
use std::sync::Arc;

use futures_util::future::BoxFuture;
use scorsese_render::{Cancel, Progress};
use sqlx::postgres::PgPool;

use super::{Job, Kind, Outcome, Queue, store};
use crate::db::{self, Tx, UserId};

/// The work one kind of job does.
///
/// Implemented for any `Fn(Job, Context) -> impl Future<Output = Outcome>`,
/// so a handler is usually a function. It is handed the claimed [`Job`] and
/// returns how the run ended; the worker records that. A handler that holds a
/// provider ticket must honour [`Job::ticket`] — see the module doc of
/// [`jobs`](super).
pub trait Handler: Send + Sync + 'static {
    /// Run `job`.
    fn run(&self, job: Job, context: Context) -> BoxFuture<'static, Outcome>;
}

impl<F, Fut> Handler for F
where
    F: Fn(Job, Context) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Outcome> + Send + 'static,
{
    fn run(&self, job: Job, context: Context) -> BoxFuture<'static, Outcome> {
        Box::pin(self(job, context))
    }
}

/// Every kind the worker claims, each with its handler.
#[derive(Clone, Default)]
pub struct Registry {
    entries: Vec<(Kind, Arc<dyn Handler>)>,
}

impl Registry {
    /// A registry with no kinds: its worker claims nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Run jobs of `kind` with `handler`, replacing any handler it had.
    pub fn register(mut self, kind: Kind, handler: impl Handler) -> Self {
        self.entries.retain(|(known, _)| known.name != kind.name);
        self.entries.push((kind, Arc::new(handler)));
        self
    }

    /// Every registered kind.
    pub(super) fn kinds(&self) -> impl Iterator<Item = Kind> + '_ {
        self.entries.iter().map(|(kind, _)| *kind)
    }

    /// The handler for the kind named `name`.
    pub(super) fn handler(&self, name: &str) -> Option<(Kind, Arc<dyn Handler>)> {
        self.entries
            .iter()
            .find(|(kind, _)| kind.name == name)
            .map(|(kind, handler)| (*kind, Arc::clone(handler)))
    }
}

/// What a running handler may reach: the database, as the job's owner, and
/// the queue — for a handler whose work leaves more work behind it, like a
/// generated file that needs a thumbnail.
#[derive(Clone)]
pub struct Context {
    pool: PgPool,
    queue: Queue,
    job: i64,
    user: UserId,
    cancel: Cancel,
    progress: Progress,
}

impl Context {
    pub(super) fn new(
        pool: PgPool,
        queue: Queue,
        job: &Job,
        cancel: Cancel,
        progress: Progress,
    ) -> Self {
        Self {
            pool,
            queue,
            job: job.id,
            user: job.user,
            cancel,
            progress,
        }
    }

    /// Tripped when the job's owner asks it to stop, or the worker is
    /// stopping. A handler of a [`STOPPABLE`](super::kinds::STOPPABLE) kind
    /// hands it to the work — `Renderer::with_cancel` — and returns
    /// [`Outcome::Cancelled`] once it has stopped; any other may ignore it.
    pub fn cancel(&self) -> &Cancel {
        &self.cancel
    }

    /// Where the job's owner reads how far it has got (#698). A handler whose
    /// work reports progress hands it over — `Renderer::with_progress` — and
    /// the worker tells the owner as it moves; any other may ignore it, and
    /// its job simply shows none.
    pub fn progress(&self) -> &Progress {
        &self.progress
    }

    /// The queue this job came from, to announce a job this one enqueued.
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// A transaction scoped to the job's owner: their rows and nobody else's,
    /// exactly as a request of theirs would see.
    pub async fn scoped(&self) -> Result<Tx, sqlx::Error> {
        db::scoped(&self.pool, self.user).await
    }

    /// The pool itself, for the one handler step that is cross-user by
    /// nature: the render cache's quota, which is the whole machine's
    /// (`renders::evict`, on `db::scope`'s privileged list). Crate-only, so
    /// no handler outside it reaches past [`Context::scoped`].
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Commit a provider's ticket to the job's row, **now** — call it the
    /// moment the provider accepts, before anything else can go wrong. After a
    /// crash the job comes back with it in [`Job::ticket`], and polls.
    pub async fn keep_ticket(&self, ticket: &str) -> Result<(), sqlx::Error> {
        store::keep_ticket(&self.pool, self.user, self.job, ticket).await
    }
}
