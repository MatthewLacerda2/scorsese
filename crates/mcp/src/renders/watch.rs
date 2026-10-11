//! Waiting on a render while the call that asked for it stays open — `render`
//! with `wait`, or `jobs` with `wait` on one already running in the background
//! (#1056) — and saying how far it has got as it goes.
//!
//! The two differ in what cancelling the call means. A render waited on from
//! the start belongs to its call, so cancelling the call stops it ([`watch`]).
//! One started in the background was meant to outlive any call, so cancelling
//! the wait only ends the wait, and the render goes on ([`await_end`]): `jobs`'
//! `cancel` is how it is stopped.
//!
//! The saying is MCP's `notifications/progress`, sent only when the call
//! carried a `progressToken`: a client that asks for them shows a bar with no
//! tool call at all. They are throttled, because the readout changes once a
//! frame and a client wants a bar that moves, not a line per frame.

use std::time::{Duration, Instant};

use scorsese_render::{Cancel, Phase, Reading};

use super::{Job, Outcome, doing};

/// How often the readout is looked at while a render is waited on.
const TICK: Duration = Duration::from_millis(100);

/// The least time between two notifications within one phase. A new phase is
/// said at once: it is the moment the words change.
const EVERY: Duration = Duration::from_millis(500);

/// Somewhere to say how far a call has got: the progress, out of 100, and
/// what is happening.
pub(crate) type Say<'a> = dyn FnMut(f64, &str) + 'a;

/// A [`Say`], as a call is handed one.
pub(crate) type Report<'a> = &'a mut Say<'a>;

/// Waits for `job` to end, stopping it if `cancel` is tripped, and reports its
/// progress to `report` on the way when there is somewhere to report it.
pub(crate) fn watch(job: &Job, cancel: &Cancel, report: Option<&mut Say<'_>>) -> Outcome {
    follow(job, cancel, report, Cancelled::Stop)
        .unwrap_or_else(|| unreachable!("a stopped job is followed until it ends"))
}

/// Waits for `job` to end, reporting its progress as [`watch`] does, and
/// gives up waiting — `None`, the render untouched — if `cancel` is tripped.
pub(crate) fn await_end(
    job: &Job,
    cancel: &Cancel,
    report: Option<&mut Say<'_>>,
) -> Option<Outcome> {
    follow(job, cancel, report, Cancelled::Leave)
}

/// What a tripped cancel does to the job being followed.
#[derive(Clone, Copy, PartialEq)]
enum Cancelled {
    /// Stops it, and follows it on until it has.
    Stop,
    /// Leaves it running, and stops following it.
    Leave,
}

fn follow(
    job: &Job,
    cancel: &Cancel,
    mut report: Option<&mut Say<'_>>,
    cancelled: Cancelled,
) -> Option<Outcome> {
    let mut said: Option<(f64, Phase, Instant)> = None;
    loop {
        if cancel.is_cancelled() {
            if cancelled == Cancelled::Leave {
                return None;
            }
            // The job's own cancel when the call's was handed to it; tripped
            // here too, so a job started under another still stops.
            job.cancel();
        } else if let Some(report) = report.as_deref_mut() {
            let reading = job.reading();
            if due(said, reading, Instant::now()) {
                report(rising(reading), &doing(reading));
                said = Some((rising(reading), reading.phase, Instant::now()));
            }
        }
        if let Some(outcome) = job.ended_within(TICK) {
            return Some(outcome);
        }
    }
}

/// Whether `reading` is worth a notification, given the last one `said`.
///
/// Only a number higher than the last: the specification asks that progress
/// rise with every notification. Then at once for a new phase, and otherwise
/// no more often than [`EVERY`].
fn due(said: Option<(f64, Phase, Instant)>, reading: Reading, now: Instant) -> bool {
    let Some((value, phase, at)) = said else {
        return true;
    };
    rising(reading) > value && (reading.phase != phase || now.duration_since(at) >= EVERY)
}

/// The progress a notification carries: the percentage, with the phase's place
/// in the order as a hundredth on top.
///
/// The percentage alone stands still across a change of phase — `0` through
/// preparing and mixing, `99` from the last frame to the finished file — and
/// the specification wants every notification's number higher than the last.
/// The hundredth keeps it rising there, without moving what a client shows as
/// a whole percentage. Capturing pages, the longest stretch at `0`, also
/// carries its share of page frames in the thousandths below the next phase's
/// place, so its count keeps being said as it moves.
pub(super) fn rising(reading: Reading) -> f64 {
    let place = match reading.phase {
        Phase::Waiting => 0.0,
        Phase::Preparing => 0.01,
        Phase::Capturing if reading.of == 0 => 0.02,
        Phase::Capturing => 0.02 + 0.009 * reading.done as f64 / reading.of as f64,
        Phase::Mixing => 0.03,
        Phase::Drawing => 0.04,
        Phase::Finishing => 0.05,
        Phase::Done => 0.06,
    };
    f64::from(reading.percent()) + place
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(phase: Phase, done: u64, of: u64) -> Reading {
        Reading { phase, done, of }
    }

    #[test]
    fn the_number_rises_through_every_phase_change_at_the_same_percentage() {
        let order = [
            at(Phase::Waiting, 0, 0),
            at(Phase::Preparing, 0, 0),
            at(Phase::Capturing, 0, 400),
            at(Phase::Capturing, 200, 400),
            at(Phase::Capturing, 400, 400),
            at(Phase::Mixing, 0, 90),
            at(Phase::Drawing, 0, 90),
            at(Phase::Drawing, 45, 90),
            at(Phase::Drawing, 90, 90),
            at(Phase::Finishing, 90, 90),
            at(Phase::Done, 90, 90),
        ];
        for pair in order.windows(2) {
            assert!(rising(pair[1]) > rising(pair[0]), "{pair:?}");
        }
        assert!((rising(at(Phase::Drawing, 45, 90)) - 50.04).abs() < 1e-9);
    }

    #[test]
    fn a_notification_is_sent_first_on_a_new_phase_and_otherwise_on_the_clock() {
        let start = Instant::now();
        let drawing = |done| at(Phase::Drawing, done, 100);
        let said = Some((rising(drawing(10)), Phase::Drawing, start));
        assert!(due(None, drawing(0), start), "the first is always sent");
        assert!(!due(said, drawing(20), start + EVERY / 2), "too soon");
        assert!(due(said, drawing(20), start + EVERY), "on the clock");
        assert!(!due(said, drawing(10), start + EVERY), "not higher");
        let mixing = Some((rising(at(Phase::Mixing, 0, 100)), Phase::Mixing, start));
        assert!(
            due(mixing, drawing(0), start),
            "a new phase is said at once"
        );
    }
}
