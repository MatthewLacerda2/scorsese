//! The worker: one loop in the server that claims jobs and runs them.

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt;
use sqlx::Executor;
use sqlx::postgres::PgPool;
use tokio::sync::{Notify, watch};
use tokio::task::{Id, JoinSet};

use super::{Context, Job, JobView, Outcome, Registry, store};
use crate::db::UserId;
use crate::events::{Event, Events};

/// How often the worker looks for work nobody announced.
///
/// A job enqueued without [`Queue::announce`], or by another process, waits at
/// most this long; one that was announced does not wait at all.
const LOOK_EVERY: Duration = Duration::from_secs(1);

/// The advisory lock that makes the worker the only one on its database.
///
/// Any fixed number: advisory locks belong to one database, so this collides
/// with nothing but another scorsese worker.
const WORKER_LOCK: i64 = 536;

/// How the rest of the server reaches the worker: announce a job, and its
/// owner hears about every change to it. Cheap to clone.
#[derive(Clone)]
pub struct Queue {
    events: Events,
    wake: Arc<Notify>,
}

impl Queue {
    /// A queue whose changes are told on `events`.
    pub fn new(events: Events) -> Self {
        Self {
            events,
            wake: Arc::new(Notify::new()),
        }
    }

    /// Tell `user` that `job` changed, and the worker to look now. Call it
    /// once the enqueue has committed.
    pub fn announce(&self, user: UserId, job: &JobView) {
        self.tell(user, job.clone());
        self.wake.notify_one();
    }

    /// Tell `user` that `job` changed.
    fn tell(&self, user: UserId, job: JobView) {
        self.events.send(user, Event::Job(job));
    }
}

/// Run jobs of every kind `registry` has, until `stop` turns true.
///
/// Waits first for the worker lock, so two servers on one database never both
/// work; then recovers whatever a dead process left running; then claims and
/// runs, each kind up to its limit. On `stop`, it stops claiming and drops what
/// it was running: those rows stay `running`, and the next start recovers
/// them exactly as it would after a crash.
pub async fn work(pool: PgPool, registry: Registry, queue: Queue, stop: watch::Receiver<bool>) {
    let mut stop = stop;
    let started = tokio::select! {
        started = start(&pool, &queue) => started,
        _ = stop.wait_for(|stopping| *stopping) => return,
    };
    // Held until this function returns; dropping it closes the connection,
    // and the lock with it.
    let Some(_lock) = started else { return };

    let mut running: HashMap<&'static str, usize> = HashMap::new();
    let mut kinds_of: HashMap<Id, &'static str> = HashMap::new();
    let mut tasks = JoinSet::new();
    loop {
        claim_what_fits(
            &pool,
            &registry,
            &queue,
            &mut running,
            &mut kinds_of,
            &mut tasks,
        )
        .await;
        tokio::select! {
            _ = stop.wait_for(|stopping| *stopping) => break,
            () = queue.wake.notified() => {}
            Some(joined) = tasks.join_next_with_id() => {
                let id = match joined {
                    Ok((id, ())) => id,
                    Err(error) => error.id(),
                };
                if let Some(count) = kinds_of.remove(&id).and_then(|kind| running.get_mut(kind)) {
                    *count -= 1;
                }
            }
            () = tokio::time::sleep(LOOK_EVERY) => {}
        }
    }
    tasks.abort_all();
}

/// Take the worker lock and recover. `None` if the database would not let us,
/// which is said in the log: the server keeps serving, and a restart retries.
async fn start(pool: &PgPool, queue: &Queue) -> Option<sqlx::PgConnection> {
    let started = async {
        let mut lock = pool.acquire().await?.detach();
        lock.execute(sqlx::query("SELECT pg_advisory_lock($1)").bind(WORKER_LOCK))
            .await?;
        let recovered = store::recover(pool).await?;
        for (user, job) in recovered {
            eprintln!(
                "scorsese-server: job {} ({}) of user {} was interrupted; now {:?}",
                job.id,
                job.kind,
                user.get(),
                job.state
            );
            queue.tell(user, job);
        }
        Ok::<_, sqlx::Error>(lock)
    };
    match started.await {
        Ok(lock) => Some(lock),
        Err(error) => {
            eprintln!("scorsese-server: the job worker could not start: {error}");
            None
        }
    }
}

/// Claim jobs until every registered kind is at its limit or nothing waits.
async fn claim_what_fits(
    pool: &PgPool,
    registry: &Registry,
    queue: &Queue,
    running: &mut HashMap<&'static str, usize>,
    kinds_of: &mut HashMap<Id, &'static str>,
    tasks: &mut JoinSet<()>,
) {
    loop {
        let open: Vec<&str> = registry
            .kinds()
            .filter(|kind| running.get(kind.name).copied().unwrap_or(0) < kind.limit)
            .map(|kind| kind.name)
            .collect();
        if open.is_empty() {
            return;
        }
        let (job, view) = match store::claim(pool, &open).await {
            Ok(Some(claimed)) => claimed,
            Ok(None) => return,
            Err(error) => {
                eprintln!("scorsese-server: could not claim a job: {error}");
                return;
            }
        };
        let Some((kind, handler)) = registry.handler(&job.kind) else {
            return; // claimed only among registered kinds, so never
        };
        queue.tell(job.user, view);
        *running.entry(kind.name).or_default() += 1;
        let handle = tasks.spawn(run(pool.clone(), handler, job, queue.clone()));
        kinds_of.insert(handle.id(), kind.name);
    }
}

/// Run one job to its end and record how it ended.
async fn run(pool: PgPool, handler: Arc<dyn super::Handler>, job: Job, queue: Queue) {
    let context = Context::new(pool.clone(), &job);
    let running = {
        let job = job.clone();
        async move { handler.run(job, context).await }
    };
    // A panic is a bug in a handler, and it must not leave the row `running`
    // until the next restart, nor take the worker down with it.
    let outcome = AssertUnwindSafe(running)
        .catch_unwind()
        .await
        .unwrap_or_else(|_| Outcome::Failed("the job crashed on the server; that is a bug".into()));
    match store::finish(&pool, &job, &outcome).await {
        Ok(view) => queue.tell(job.user, view),
        // Left `running`: the next start recovers it and runs it again, and a
        // job holding a ticket polls rather than paying twice.
        Err(error) => eprintln!("scorsese-server: could not record job {}: {error}", job.id),
    }
}
