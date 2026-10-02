//! Stopping a render that is already running.
//!
//! A render is the one call here that runs for minutes or hours, and "stop" is
//! the first thing a person reaches for once they have seen how long it will
//! take (#647). Whoever started it keeps one [`Cancel`] and trips it from
//! another thread; the render looks at it between frames and between stages,
//! and on seeing it tripped stops compositing, shuts its ffmpeg children down
//! and removes the file it had started — a truncated `.mp4` with no index looks
//! like a file and plays as nothing, which is the expensive kind of wrong.
//!
//! A flag rather than a channel or a callback, because the question the loop
//! asks is the cheapest one there is — *has anybody asked me to stop?* — and it
//! asks it once a frame.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A shared "stop now", cloned between whoever may ask for it and the render
/// that obeys it.
///
/// Every clone is the same flag. One that is never tripped costs one atomic
/// read per frame, which is why a [`crate::Renderer`] always has one.
#[derive(Debug, Clone, Default)]
pub struct Cancel {
    flag: Arc<AtomicBool>,
    /// For the tests only: trip by itself once it has been asked this many
    /// times, so a test can cancel at a known frame instead of racing a
    /// thread against the encoder.
    #[cfg(test)]
    budget: Option<Arc<std::sync::atomic::AtomicU64>>,
}

impl Cancel {
    /// A flag nobody has tripped yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks every render holding a clone of this to stop. Idempotent, and
    /// safe from any thread.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Whether anybody has asked to stop.
    pub fn is_cancelled(&self) -> bool {
        #[cfg(test)]
        if let Some(budget) = &self.budget
            && budget
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    left.checked_sub(1)
                })
                .is_err()
        {
            self.cancel();
        }
        self.flag.load(Ordering::SeqCst)
    }

    /// A flag that answers "no" `checks` times and trips itself on the next.
    #[cfg(test)]
    pub(crate) fn after(checks: u64) -> Self {
        Self {
            budget: Some(Arc::new(std::sync::atomic::AtomicU64::new(checks))),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_clone_is_the_same_flag() {
        let asked = Cancel::new();
        let render = asked.clone();
        assert!(!render.is_cancelled());
        asked.cancel();
        assert!(render.is_cancelled());
    }

    #[test]
    fn a_budget_trips_on_the_check_after_it_runs_out() {
        let cancel = Cancel::after(2);
        assert!(!cancel.is_cancelled());
        assert!(!cancel.is_cancelled());
        assert!(cancel.is_cancelled());
        assert!(cancel.is_cancelled(), "and stays tripped");
    }
}
