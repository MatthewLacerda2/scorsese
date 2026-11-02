//! Which handler runs which kind, and what a handler is given to run with.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use futures_util::future::BoxFuture;
use scorsese_render::{Cancel, Progress};
use sqlx::postgres::PgPool;

use super::{Allowance, Budget, Job, Kind, Outcome, Queue, store};
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

/// How many of a [`Budget`]'s kinds run at once when nobody said
/// ([`Registry::budget`]): one, so a budget somebody forgot to size is slow,
/// never a flood of the vendor it stands for.
pub const UNSIZED: usize = 1;

/// Every kind the worker claims, each with its handler, how big each budget
/// they share is, and how many of a kind one user may have running.
#[derive(Clone, Default)]
pub struct Registry {
    entries: Vec<(Kind, Arc<dyn Handler>)>,
    budgets: HashMap<Budget, usize>,
    per_user: HashMap<&'static str, usize>,
    allowances: HashMap<&'static str, Allowance>,
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

    /// Let at most `size` jobs of `budget`'s kinds run at once, together.
    pub fn budget(mut self, budget: Budget, size: usize) -> Self {
        self.budgets.insert(budget, size);
        self
    }

    /// Let at most `most` jobs of `kind` run at once for any one user: the
    /// rest of theirs wait in line while other users' are claimed.
    pub fn per_user(mut self, kind: Kind, most: usize) -> Self {
        self.per_user.insert(kind.name, most);
        self
    }

    /// Stop a job of `kind` once it has run past `allowance` for its work
    /// ([`Allowance`], #1634), and fail it.
    pub fn allowance(mut self, kind: Kind, allowance: Allowance) -> Self {
        self.allowances.insert(kind.name, allowance);
        self
    }

    /// How long a job of the kind called `name` may run, if it is bounded.
    pub(super) fn allowance_of(&self, name: &str) -> Option<Allowance> {
        self.allowances.get(name).copied()
    }

    /// Each kind one user may run only so many of at once, and how many.
    pub(super) fn per_user_caps(&self) -> impl Iterator<Item = (&'static str, usize)> + '_ {
        self.per_user.iter().map(|(name, most)| (*name, *most))
    }

    /// How many of `budget`'s kinds may run at once: what
    /// [`budget`](Self::budget) said, or [`UNSIZED`].
    pub(super) fn size(&self, budget: Budget) -> usize {
        self.budgets.get(&budget).copied().unwrap_or(UNSIZED)
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
    lingering: Lingering,
    work: Work,
}

/// What a job's handler left running when it returned, its kind's slot held
/// until it ends ([`Context::hold_slot_until`]).
#[derive(Clone, Default)]
pub(super) struct Lingering(Arc<Mutex<Vec<BoxFuture<'static, ()>>>>);

impl Lingering {
    /// Waits until everything left running has ended.
    pub(super) async fn out(&self) {
        let left = std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner));
        futures_util::future::join_all(left).await;
    }
}

/// How much work a running job said it is ([`Context::measure`]), and the
/// time it was granted on top for work another process bounds
/// ([`Work::extend`]), shared with the worker's timer.
#[derive(Debug, Clone, Default)]
pub(crate) struct Work {
    seconds: Arc<AtomicU64>,
    extra_millis: Arc<AtomicU64>,
}

impl Work {
    /// The seconds last measured; none until the job says.
    pub(super) fn seconds(&self) -> f64 {
        f64::from_bits(self.seconds.load(Ordering::Relaxed))
    }

    /// The time granted on top of the allowance.
    pub(super) fn extra(&self) -> Duration {
        Duration::from_millis(self.extra_millis.load(Ordering::Relaxed))
    }

    /// Grants the job `more` time on top of its allowance, for work bounded
    /// by a deadline of its own elsewhere: the page captures a render waits
    /// on, each held to `captures::deadline` by the launcher.
    pub(crate) fn extend(&self, more: Duration) {
        let millis = u64::try_from(more.as_millis()).unwrap_or(u64::MAX);
        let _ = self
            .extra_millis
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |was| {
                Some(was.saturating_add(millis))
            });
    }

    /// Records `seconds` of work; a negative or unreadable count as none.
    pub(super) fn set(&self, seconds: f64) {
        let seconds = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
        self.seconds.store(seconds.to_bits(), Ordering::Relaxed);
    }
}

impl Context {
    pub(super) fn new(
        pool: PgPool,
        queue: Queue,
        job: &Job,
        (cancel, progress): (Cancel, Progress),
        work: Work,
    ) -> Self {
        Self {
            pool,
            queue,
            job: job.id,
            user: job.user,
            cancel,
            progress,
            lingering: Lingering::default(),
            work,
        }
    }

    /// What the worker waits out once the outcome is recorded.
    pub(super) fn lingering(&self) -> Lingering {
        self.lingering.clone()
    }

    /// Keeps the job's slot taken after its outcome is recorded, until `work`
    /// ends: work the handler had to let run out that still uses what the
    /// slot rations — a stopped transcription's request, which the Whisper
    /// server finishes either way. The job shows its outcome at once; the
    /// next of its kind starts only after.
    pub fn hold_slot_until(&self, work: impl Future<Output = ()> + Send + 'static) {
        let mut lingering = self
            .lingering
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        lingering.push(Box::pin(work));
    }

    /// Say how long this job's work is, in seconds — of picture rendered, of
    /// sound heard: what a kind with an [`Allowance`] is given more time for
    /// (#1634). Call it once the work is known and before it starts; until
    /// then the job has its allowance's base alone.
    pub fn measure(&self, seconds: f64) {
        self.work.set(seconds);
    }

    /// What the worker's timer reads, for blocking work that grants itself
    /// more time as it learns how much it waits on ([`Work::extend`]).
    pub(crate) fn work(&self) -> Work {
        self.work.clone()
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

    /// Commit to the job's row that its vendor is about to be called — call
    /// it just before the call, and make no call when it fails. After a stop
    /// the job comes back with [`Job::sent`], and is never sent again.
    pub async fn sending(&self) -> Result<(), sqlx::Error> {
        store::sending(&self.pool, self.user, self.job).await
    }
}
