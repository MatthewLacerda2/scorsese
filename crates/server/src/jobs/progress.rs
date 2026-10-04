//! How far a running render or preview has got (#698), told to its owner
//! while it runs.
//!
//! The renderer publishes its [`Progress`] once a frame (#697); a person wants
//! to see it a couple of times a second. So the worker **samples** it beside
//! the job's handler ([`watch`]) and sends an [`Event::JobProgress`] only when
//! what a person would see has moved — the percentage or the phase — and at
//! most every [`EVERY`]. Nothing is written to Postgres: the readout lives in
//! memory beside the job's cancel flag, and a job read while it runs has it
//! folded in ([`super::Queue::progressed`]). A finished job, or one a restart
//! put back in line, simply has none.

use std::convert::Infallible;
use std::future::Future;
use std::time::Duration;

use scorsese_render::{Phase, Progress, Reading};
use serde::Serialize;

use super::Queue;
use crate::db::UserId;
use crate::events::Event;

/// How often a running job's progress is looked at: twice a second is as
/// smooth as a bar needs, and a long render sends at most a hundred-odd
/// events however many frames it has.
pub(super) const EVERY: Duration = Duration::from_millis(500);

/// How far a running job has got, as its owner sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProgressView {
    /// From `0` to `100`, counting frames only: `0` while preparing and
    /// mixing, `99` while finishing, `100` only once the file is there.
    pub percent: u8,
    /// What it is doing, for the stretches the percentage does not move in:
    /// `preparing`, `mixing`, `drawing`, `finishing` or `done`.
    pub phase: &'static str,
    /// Frames encoded so far.
    pub done: u64,
    /// Frames in all; `0` until planned, and for a sound-only render.
    pub of: u64,
}

impl ProgressView {
    /// What a person sees of `reading`; `None` before any render has started
    /// with it — a handler that never attached it, or one still laying out.
    pub fn of(reading: Reading) -> Option<Self> {
        let phase = match reading.phase {
            Phase::Waiting => return None,
            Phase::Preparing => "preparing",
            Phase::Mixing => "mixing",
            Phase::Drawing => "drawing",
            Phase::Finishing => "finishing",
            Phase::Done => "done",
        };
        Some(Self {
            percent: reading.percent(),
            phase,
            done: reading.done,
            of: reading.of,
        })
    }

    /// Whether a person who last saw `self` would see something else in
    /// `now`. Frame counts alone do not count: they move every frame.
    fn moved(last: Option<Self>, now: Self) -> bool {
        last.is_none_or(|last| (last.percent, last.phase) != (now.percent, now.phase))
    }
}

/// Run `work`, telling `user` how far job `id` has got through `progress`
/// while it runs, and answer what `work` answers.
pub(super) async fn watch<F: Future>(
    work: F,
    queue: &Queue,
    user: UserId,
    id: i64,
    progress: Progress,
) -> F::Output {
    tokio::select! {
        output = work => output,
        never = tell(queue, user, id, progress) => match never {},
    }
}

/// Look at `progress` every [`EVERY`] and say it whenever it has moved. Never
/// ends: [`watch`] drops it when the work does.
async fn tell(queue: &Queue, user: UserId, id: i64, progress: Progress) -> Infallible {
    let mut every = tokio::time::interval(EVERY);
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last = None;
    loop {
        every.tick().await;
        let Some(now) = ProgressView::of(progress.read()) else {
            continue;
        };
        if ProgressView::moved(last, now) {
            queue
                .events()
                .send(user, Event::JobProgress { id, progress: now });
            last = Some(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(phase: Phase, done: u64, of: u64) -> Reading {
        Reading { phase, done, of }
    }

    #[test]
    fn nothing_is_shown_before_a_render_starts() {
        assert_eq!(ProgressView::of(at(Phase::Waiting, 0, 0)), None);
    }

    #[test]
    fn a_reading_is_shown_with_its_phase_in_words() {
        let shown = |phase| ProgressView::of(at(phase, 3, 4)).map(|view| view.phase);
        assert_eq!(shown(Phase::Preparing), Some("preparing"));
        assert_eq!(shown(Phase::Mixing), Some("mixing"));
        assert_eq!(shown(Phase::Finishing), Some("finishing"));
        assert_eq!(shown(Phase::Done), Some("done"));
        assert_eq!(
            ProgressView::of(at(Phase::Drawing, 760, 1800)),
            Some(ProgressView {
                percent: 42,
                phase: "drawing",
                done: 760,
                of: 1800
            })
        );
    }

    #[test]
    fn only_a_new_percentage_or_phase_is_worth_saying() {
        let view = |phase, done| ProgressView::of(at(phase, done, 1000)).unwrap();
        let first = view(Phase::Drawing, 100);
        assert!(ProgressView::moved(None, first), "the first is always said");
        assert!(!ProgressView::moved(Some(first), view(Phase::Drawing, 105)));
        assert!(ProgressView::moved(Some(first), view(Phase::Drawing, 110)));
        let finishing = view(Phase::Finishing, 1000);
        assert!(ProgressView::moved(
            Some(view(Phase::Drawing, 999)),
            finishing
        ));
        assert!(ProgressView::moved(
            Some(finishing),
            view(Phase::Done, 1000)
        ));
    }
}
