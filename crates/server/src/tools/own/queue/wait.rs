//! `jobs` with `wait: true` (#1301): answer once a job has ended, or after a
//! minute with how it stands then.
//!
//! The local `jobs` waits on a render for as long as it takes (#1056). Here a
//! call is an HTTP request and, from the built-in assistant, a chat turn, and a
//! batch of stills can take a day, so the wait is **bounded** at [`BOUND`]:
//! the call answers with the job's line either way, and one that has not ended
//! says so and can be waited on again. Without it a client that needs a result
//! asks over and over — #1275 counted twelve calls and $0.55 of a user's
//! credits for one shot.
//!
//! It is a **long poll on the job announcements** ([`Queue::subscribe`]), not
//! a loop of reads: the worker already tells the owner of every change of
//! state, so the waiting costs the server a parked task. The subscription is
//! taken before the row is read, so an end that lands between the two is
//! heard rather than missed. Whatever wakes the wait — the job's end, events
//! dropped ([`Event::Resync`]), the bus closing for a shutdown, the bound — the
//! answer is the row read afresh: the stream is the nudge, the database the
//! record, as everywhere else on it.

use std::time::Duration;

use futures_util::{Stream, StreamExt};
use scorsese_mcp::Reply;

use super::super::super::{Caller, database};
use super::line;
use crate::events::Event;
use crate::jobs::{JobView, State, store as jobs};

/// The longest a call waits: under the hundred seconds Cloudflare holds an
/// idle request and nginx's five minutes, and long enough that a
/// shot of a few minutes is a handful of calls rather than a dozen.
const BOUND: Duration = Duration::from_secs(60);

/// The line of the caller's job `id` once it has ended, or once
/// [`BOUND`] has passed with it still going.
pub(super) async fn waited(caller: &Caller<'_>, id: i64) -> Result<Reply, String> {
    let toolbox = caller.toolbox;
    // Listening before the read, so an end between the two is not missed.
    let events = toolbox.queue.subscribe(caller.user);
    let read = || async {
        jobs::get(&toolbox.pool, caller.user, id)
            .await
            .map_err(database)?
            .ok_or_else(|| format!("there is no job {id} of yours"))
    };
    if !ended(read().await?.state) {
        heard_end(events, id, BOUND).await;
    }
    let job = toolbox.queue.progressed(read().await?);
    let said = line(&job);
    Ok(if ended(job.state) {
        said
    } else {
        format!("{said} — still going after a minute: wait again, or leave it running")
    }
    .into())
}

/// Whether a job in `state` is over: nothing more will happen to it on its own.
fn ended(state: State) -> bool {
    !matches!(state, State::Waiting | State::Running)
}

/// Wait until `events` says job `id` has ended, or says something the waiter
/// can no longer trust (dropped events, a closed bus), or until `bound` has
/// passed — whichever is first. The caller then reads the row.
async fn heard_end(events: impl Stream<Item = Event>, id: i64, bound: Duration) {
    let over = events.filter(|event| {
        std::future::ready(match event {
            Event::Job(JobView { id: job, state, .. }) => *job == id && ended(*state),
            Event::Resync => true,
            _ => false,
        })
    });
    let mut over = std::pin::pin!(over);
    // Timed out, heard the end, or the stream ended: all the same to the caller.
    let _ = tokio::time::timeout(bound, over.next()).await;
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use futures_util::stream;

    use super::*;

    fn job(id: i64, state: State) -> Event {
        Event::Job(JobView {
            id,
            kind: "veo_shot".into(),
            state,
            attempts: 1,
            result: None,
            error: None,
            created_at: 0,
            started_at: None,
            finished_at: None,
            interrupted_at: None,
            progress: None,
        })
    }

    /// `events`, then silence: a bus that stays open.
    fn then_quiet(events: Vec<Event>) -> impl Stream<Item = Event> {
        stream::iter(events).chain(stream::pending())
    }

    async fn took(events: impl Stream<Item = Event>, bound: Duration) -> Duration {
        let started = Instant::now();
        heard_end(events, 7, bound).await;
        started.elapsed()
    }

    const LONG: Duration = Duration::from_secs(30);

    #[tokio::test]
    async fn the_end_of_the_job_waited_on_answers_at_once() {
        for state in [State::Done, State::Failed, State::Stuck, State::Cancelled] {
            let heard = then_quiet(vec![job(7, State::Running), job(7, state)]);
            assert!(took(heard, LONG).await < LONG / 2, "{state:?}");
        }
    }

    #[tokio::test]
    async fn dropped_events_or_a_closed_bus_answer_at_once() {
        assert!(took(then_quiet(vec![Event::Resync]), LONG).await < LONG / 2);
        assert!(took(stream::empty(), LONG).await < LONG / 2);
    }

    #[tokio::test]
    async fn another_job_ending_or_this_one_running_waits_out_the_bound() {
        let bound = Duration::from_millis(150);
        let heard = then_quiet(vec![
            job(8, State::Done),
            job(7, State::Waiting),
            job(7, State::Running),
        ]);
        let took = took(heard, bound).await;
        assert!(took >= bound && took < LONG / 2, "{took:?}");
    }
}
