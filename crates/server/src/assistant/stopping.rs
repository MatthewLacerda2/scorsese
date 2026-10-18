//! Which turns have been asked to stop, and the tool call each is running.
//!
//! A Stop is heard between a turn's steps ([`Stopping::requested`]), and also
//! by the call running at that moment, through its [`Cancel`]: a `still`
//! waiting on its pages can wait a minute and a half, and a stopped turn must
//! not hold a thread that long for an answer nobody reads (#1469).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};

use scorsese_render::Cancel;

/// The turns asked to stop, and the cancel of each one's running call.
/// Cheap to clone: every clone is the same.
#[derive(Clone, Default)]
pub(super) struct Stopping {
    asked: Arc<Mutex<HashSet<i64>>>,
    calls: Arc<Mutex<HashMap<i64, Cancel>>>,
}

/// The cancel of one turn's running call, forgotten once dropped.
pub(super) struct Calling {
    stopping: Stopping,
    turn: i64,
    /// Tripped when the turn is asked to stop.
    pub(super) cancel: Cancel,
}

impl Stopping {
    /// Ask `turn` to stop before its next step, and stop its running call.
    pub(super) fn ask(&self, turn: i64) {
        lock(&self.asked).insert(turn);
        if let Some(cancel) = lock(&self.calls).get(&turn) {
            cancel.cancel();
        }
    }

    /// Whether `turn` was asked to stop; forgets the ask.
    pub(super) fn requested(&self, turn: i64) -> bool {
        lock(&self.asked).remove(&turn)
    }

    /// A cancel for the call `turn` is about to make: tripped at once when
    /// the turn was asked to stop already, and by any later ask until the
    /// call is over.
    pub(super) fn calling(&self, turn: i64) -> Calling {
        let cancel = Cancel::new();
        // Registered before looking at the asks, so an ask made in between
        // finds it.
        lock(&self.calls).insert(turn, cancel.clone());
        if lock(&self.asked).contains(&turn) {
            cancel.cancel();
        }
        Calling {
            stopping: self.clone(),
            turn,
            cancel,
        }
    }
}

impl Drop for Calling {
    fn drop(&mut self) {
        lock(&self.stopping.calls).remove(&self.turn);
    }
}

fn lock<T>(held: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // A set is still a set after a panic elsewhere held the lock.
    held.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stop_trips_the_running_call_and_is_still_heard_between_steps() {
        let stopping = Stopping::default();
        let calling = stopping.calling(7);
        let other = stopping.calling(8);

        stopping.ask(7);

        assert!(calling.cancel.is_cancelled());
        assert!(!other.cancel.is_cancelled(), "only the turn asked");
        assert!(stopping.requested(7));
    }

    #[test]
    fn a_call_made_after_a_stop_is_stopped_before_it_runs() {
        let stopping = Stopping::default();
        stopping.ask(7);

        assert!(stopping.calling(7).cancel.is_cancelled());
    }

    #[test]
    fn a_finished_call_is_forgotten() {
        let stopping = Stopping::default();
        let first = stopping.calling(7);
        drop(first);

        stopping.ask(7);

        assert!(lock(&stopping.calls).is_empty());
    }
}
