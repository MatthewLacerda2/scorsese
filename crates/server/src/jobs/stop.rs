//! Stopping a job its owner no longer wants (#660) — the module doc of
//! [`jobs`](super), *Stopping one*, has the argument.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use scorsese_render::{Cancel, Progress, Reading};
use sqlx::postgres::PgPool;

use super::{JobView, Queue, State, kinds, store};
use crate::db::UserId;

/// The flag and the progress readout of every job this process is running,
/// by job id.
///
/// In memory, because what it answers — *is anybody still waiting for this
/// render, and how far has it got?* — is about this process: after a restart
/// the job is waiting again and a cancel finds it in the table, not here.
#[derive(Clone, Default)]
pub(super) struct Running(Arc<Mutex<HashMap<i64, (Cancel, Progress)>>>);

impl Running {
    /// The flag for job `id`, made if nobody has asked for it yet. The worker
    /// asks as it starts the job, and a cancel that lands between the claim
    /// and that finds the same flag, already tripped.
    pub(super) fn flag(&self, id: i64) -> Cancel {
        self.lock().entry(id).or_default().0.clone()
    }

    /// The progress readout for job `id`, made beside its flag if need be.
    pub(super) fn progress(&self, id: i64) -> Progress {
        self.lock().entry(id).or_default().1.clone()
    }

    /// How far job `id` has got, if this process is running it — without
    /// making anything for a job it is not.
    pub(super) fn reading(&self, id: i64) -> Option<Reading> {
        self.lock().get(&id).map(|(_, progress)| progress.read())
    }

    /// Forget job `id`'s flag: it is no longer running.
    pub(super) fn forget(&self, id: i64) {
        self.lock().remove(&id);
    }

    /// Trip every flag and forget them all — the worker is stopping, and what
    /// it ran will be recovered and run again with a fresh one.
    pub(super) fn stop_all(&self) {
        for (_, (cancel, _)) in self.lock().drain() {
            cancel.cancel();
        }
    }

    /// No code under the lock can panic, so a poisoned map is used as it is.
    fn lock(&self) -> MutexGuard<'_, HashMap<i64, (Cancel, Progress)>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Why a job could not be cancelled.
#[derive(Debug, thiserror::Error)]
pub enum CancelError {
    /// There is no such job of theirs — whether or not it is somebody else's.
    #[error("there is no such job of yours")]
    NotFound,
    /// A kind that is never stopped, named.
    #[error(
        "a {0} job cannot be stopped: only renders and previews can — a generation is billed \
         whether or not anybody still wants it"
    )]
    Unstoppable(String),
    /// The database failed.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}

/// Stop `user`'s job `id`: a waiting one is `cancelled` now, a running one is
/// asked to stop and becomes `cancelled` when its handler has — within a frame,
/// for a render. One already finished is left as it is. Hands back the job as
/// it is after asking, and tells its owner of any change.
pub async fn cancel(
    pool: &PgPool,
    queue: &Queue,
    user: UserId,
    id: i64,
) -> Result<JobView, CancelError> {
    let job = store::get(pool, user, id)
        .await?
        .ok_or(CancelError::NotFound)?;
    if !kinds::stoppable(&job.kind) {
        return Err(CancelError::Unstoppable(job.kind));
    }
    if let Some(cancelled) = store::cancel_waiting(pool, user, id).await? {
        queue.tell(user, cancelled.clone());
        return Ok(cancelled);
    }
    // Running — or claimed a moment ago, which the flag covers — or finished.
    queue.running().flag(id).cancel();
    let now = store::get(pool, user, id)
        .await?
        .ok_or(CancelError::NotFound)?;
    if now.state != State::Running {
        // Finished before the flag was made: nobody will forget it but us.
        queue.running().forget(id);
    }
    Ok(queue.progressed(now))
}
