//! A member's in-request tool calls, bounded (#1649).
//!
//! A registry tool on a stored project ([`super::stored`]) runs inside the
//! HTTP request, on tokio's blocking pool, on the one machine every member
//! shares. Web MCP's rate (`http::mcp::CALLS_PER_MINUTE`) bounds how often a
//! member calls, not how many of those calls run at once or for how long, so
//! two more bounds hold here:
//!
//! - **At most [`AT_ONCE`] at a time per member**: a fourth waits for a
//!   permit. The permit goes into the blocking work itself and is let go only
//!   when that work ends, so a call its caller stopped waiting for still holds
//!   one while its thread runs.
//! - **A deadline of [`DEADLINE`]** from when the call starts running: past
//!   it the tool is told to stop and the caller is answered with a refusal
//!   saying so ([`within`]). The tool is handed the call's own cancel — one
//!   per call, on web MCP and in the assistant alike — so the deadline trips
//!   that one, and a Stop, a hang-up or the deadline all reach the tool
//!   through it. A call whose request is dropped while its tool runs trips it
//!   on the way out ([`Unheard`]).
//!
//! One semaphore is kept per member who has called, for the life of the
//! process: an entry is a few words, and members are accounts.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use scorsese_render::Cancel;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;

use super::Toolbox;
use crate::db::UserId;

/// How many in-request tool calls one member may have running at once.
pub const AT_ONCE: usize = 3;

/// How long one in-request tool call may run.
pub const DEADLINE: Duration = Duration::from_secs(120);

/// The permits each member's calls take, and how long each may run.
#[derive(Clone)]
pub(super) struct Bounds {
    at_once: usize,
    deadline: Duration,
    held: Arc<Mutex<HashMap<UserId, Arc<Semaphore>>>>,
}

impl Toolbox {
    /// The same tools, with at most `at_once` of a member's registry tool
    /// calls running at a time and each stopped past `deadline` — a test's,
    /// which cannot wait [`DEADLINE`].
    pub fn bounded(mut self, at_once: usize, deadline: Duration) -> Self {
        self.bounds = Bounds::new(at_once, deadline);
        self
    }
}

impl Bounds {
    /// At most `at_once` calls per member, each for at most `deadline`.
    pub(super) fn new(at_once: usize, deadline: Duration) -> Self {
        Self {
            at_once: at_once.max(1),
            deadline,
            held: Arc::default(),
        }
    }

    /// One of `user`'s permits, once one is free — or `None` once `asked` is
    /// tripped while it waits, so a Stop reaches a call still in line.
    pub(super) async fn permit(
        &self,
        user: UserId,
        asked: &Cancel,
    ) -> Option<Arc<OwnedSemaphorePermit>> {
        let semaphore = {
            let mut held = self
                .held
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            held.entry(user)
                .or_insert_with(|| Arc::new(Semaphore::new(self.at_once)))
                .clone()
        };
        let mut acquiring = Box::pin(semaphore.acquire_owned());
        loop {
            if asked.is_cancelled() {
                return None;
            }
            tokio::select! {
                biased;
                permit = &mut acquiring => {
                    let permit = permit.expect("a member's semaphore is never closed");
                    return Some(Arc::new(permit));
                }
                () = tokio::time::sleep(LOOK) => {}
            }
        }
    }

    /// How long one call may run.
    pub(super) fn deadline(&self) -> Duration {
        self.deadline
    }
}

/// How often a call waiting for a permit looks at whether its caller
/// stopped it.
const LOOK: Duration = Duration::from_millis(50);

/// Why a call's work was not answered by its deadline.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Late {
    /// The deadline passed with the work still running.
    Overran,
    /// The caller stopped it, and it had not ended by the deadline.
    Stopped,
}

/// `working`'s answer, if it comes by `until`; past `until`, `cancel` is
/// tripped and the work is left to end on its own, answered as [`Late`].
pub(super) async fn within<F: Future>(
    working: F,
    until: Instant,
    cancel: &Cancel,
) -> Result<F::Output, Late> {
    tokio::select! {
        // Work that is done by the deadline is answered, never thrown away.
        biased;
        done = working => Ok(done),
        () = tokio::time::sleep_until(until) => {
            let stopped = cancel.is_cancelled();
            cancel.cancel();
            Err(if stopped { Late::Stopped } else { Late::Overran })
        }
    }
}

/// Trips the call's cancel if dropped before [`Unheard::heard`]: a request
/// dropped — the client hung up, the proxy gave up — while its tool runs
/// tells the tool to stop, since nobody will read its answer.
pub(super) struct Unheard(Option<Cancel>);

impl Unheard {
    /// Armed with the call's own cancel.
    pub(super) fn new(cancel: &Cancel) -> Self {
        Self(Some(cancel.clone()))
    }

    /// The answer reached the caller: nothing to stop.
    pub(super) fn heard(mut self) {
        self.0 = None;
    }
}

impl Drop for Unheard {
    fn drop(&mut self) {
        if let Some(cancel) = self.0.take() {
            cancel.cancel();
        }
    }
}

/// What a call past its deadline is answered with.
pub(super) fn overran(deadline: Duration) -> String {
    format!(
        "this ran past the {} seconds a tool call may take on the server, so it was stopped — \
         call again, on less of the project (a shorter stretch, fewer frames) if it is a big one",
        deadline.as_secs()
    )
}

/// What a call stopped while its tool ran is answered with: the deadline
/// covers the tool's own work, never a save, so nothing was saved.
pub(super) const STOPPED: &str = "stopped while it ran, as asked, so nothing was saved";

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use scorsese_render::Cancel;
    use tokio::time::Instant;

    use super::{Bounds, Late, Unheard, overran, within};
    use crate::db::UserId;

    #[tokio::test]
    async fn a_member_past_their_permits_waits_and_another_does_not() {
        let bounds = Bounds::new(2, Duration::from_secs(1));
        let (ana, bob, asked) = (UserId::from_row(1), UserId::from_row(2), Cancel::new());
        let held = [
            bounds.permit(ana, &asked).await,
            bounds.permit(ana, &asked).await,
        ];
        let wait = Duration::from_millis(200);
        let third = tokio::time::timeout(wait, bounds.permit(ana, &asked)).await;
        assert!(third.is_err(), "ana's third waits");
        let other = tokio::time::timeout(wait, bounds.permit(bob, &asked)).await;
        assert!(
            other.is_ok_and(|it| it.is_some()),
            "bob has permits of his own"
        );
        drop(held);
        let freed = tokio::time::timeout(wait, bounds.permit(ana, &asked)).await;
        assert!(
            freed.is_ok_and(|it| it.is_some()),
            "a permit let go is ana's again"
        );
    }

    #[tokio::test]
    async fn a_call_stopped_while_it_waits_for_a_permit_stops_waiting() {
        let bounds = Bounds::new(1, Duration::from_secs(1));
        let (ana, asked) = (UserId::from_row(1), Cancel::new());
        let _held = bounds.permit(ana, &Cancel::new()).await;
        let stopping = asked.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            stopping.cancel();
        });
        let waited = tokio::time::timeout(Duration::from_secs(5), bounds.permit(ana, &asked)).await;
        assert!(
            waited.is_ok_and(|it| it.is_none()),
            "stopped, not given one"
        );
    }

    #[tokio::test]
    async fn work_past_its_deadline_is_stopped_and_answered_late() {
        let cancel = Cancel::new();
        let until = Instant::now() + Duration::from_millis(60);
        let never = std::future::pending::<()>();
        assert_eq!(within(never, until, &cancel).await, Err(Late::Overran));
        assert!(cancel.is_cancelled(), "told to stop");

        let cancel = Cancel::new();
        let later = Instant::now() + Duration::from_secs(5);
        assert_eq!(within(async { 7 }, later, &cancel).await, Ok(7));
        assert!(!cancel.is_cancelled());
    }

    #[tokio::test]
    async fn work_stopped_that_runs_on_is_answered_as_stopped_not_overran() {
        let cancel = Cancel::new();
        cancel.cancel();
        let until = Instant::now() + Duration::from_millis(60);
        let ignores = std::future::pending::<()>();
        assert_eq!(within(ignores, until, &cancel).await, Err(Late::Stopped));
    }

    #[test]
    fn a_call_dropped_unheard_trips_its_cancel_and_one_heard_does_not() {
        let cancel = Cancel::new();
        drop(Unheard::new(&cancel));
        assert!(cancel.is_cancelled());

        let cancel = Cancel::new();
        Unheard::new(&cancel).heard();
        assert!(!cancel.is_cancelled());
    }

    #[test]
    fn the_refusal_names_the_deadline() {
        assert!(overran(Duration::from_secs(120)).contains("120 seconds"));
    }
}
