//! Which handler runs which kind, and what a handler is given to run with.

use std::future::Future;
use std::sync::Arc;

use futures_util::future::BoxFuture;
use sqlx::postgres::PgPool;

use super::{Job, Kind, Outcome, store};
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

/// What a running handler may reach: the database, as the job's owner.
#[derive(Clone)]
pub struct Context {
    pool: PgPool,
    job: i64,
    user: UserId,
}

impl Context {
    pub(super) fn new(pool: PgPool, job: &Job) -> Self {
        Self {
            pool,
            job: job.id,
            user: job.user,
        }
    }

    /// A transaction scoped to the job's owner: their rows and nobody else's,
    /// exactly as a request of theirs would see.
    pub async fn scoped(&self) -> Result<Tx, sqlx::Error> {
        db::scoped(&self.pool, self.user).await
    }

    /// Commit a provider's ticket to the job's row, **now** — call it the
    /// moment the provider accepts, before anything else can go wrong. After a
    /// crash the job comes back with it in [`Job::ticket`], and polls.
    pub async fn keep_ticket(&self, ticket: &str) -> Result<(), sqlx::Error> {
        store::keep_ticket(&self.pool, self.user, self.job, ticket).await
    }
}
