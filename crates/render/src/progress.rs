//! How far a running render has got.
//!
//! A render runs for minutes or hours, and three surfaces want to say how far
//! along it is — the web app's bar, the desktop app's popup, the assistant
//! saying "it's 40% done" (#697). Only the renderer knows, so it publishes the
//! number here and every surface reads the same one.
//!
//! The same shape as [`crate::Cancel`], for the same reason: a handle cloned
//! between the render and whoever watches it, updated with atomics once a
//! frame, read from any thread at any moment without a lock. A callback was
//! the alternative and is the wrong way round — it would run a surface's code
//! on the frame path, at the frame rate, when every surface only wants to look
//! now and then.
//!
//! # What the percentage means
//!
//! Drawing the frames is nearly all of a video render's time, and it is the
//! only stage whose length is known in advance — the mix, the loudness
//! rehearsal and the encoder's flush depend on the material and the codec.
//! Giving those stages fixed slices of the bar would be inventing numbers, so
//! [`Reading::percent`] counts **frames only**, and the [`Phase`] is shown
//! beside it for the stretches frames say nothing about:
//!
//! - before the first frame (preparing, mixing) it is `0`;
//! - while drawing it is the share of frames encoded, held below `100`;
//! - after the last frame (the encoder's flush, the final measure) it is `99`,
//!   because the file is not there yet;
//! - `100` only once the render has returned its report.
//!
//! A sound-only render draws no frames, so it goes from `0` straight to `99`
//! when the mix is in hand and is being encoded.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

/// Which part of a render is under way.
///
/// In the order a render passes through them. A render that fails or is
/// cancelled stays at the phase it stopped in; its `Result` says why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// Not started: the handle has not been given to a render yet.
    Waiting,
    /// Probing the media, planning the timeline and measuring the sources.
    Preparing,
    /// Mixing the sound and rehearsing its loudness through the codec.
    Mixing,
    /// Compositing and encoding frames — the stage [`Reading::done`] counts.
    Drawing,
    /// Every frame is in; the encoder is flushing and the file is being
    /// measured. For a sound-only render, the mix is being encoded.
    Finishing,
    /// The render returned its report and the file is complete.
    Done,
}

impl Phase {
    const ALL: [Self; 6] = [
        Self::Waiting,
        Self::Preparing,
        Self::Mixing,
        Self::Drawing,
        Self::Finishing,
        Self::Done,
    ];
}

/// One look at a render's progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// The part of the render under way.
    pub phase: Phase,
    /// Frames encoded so far. Never more than [`Reading::of`].
    pub done: u64,
    /// Frames the render will encode in all: `0` until the timeline has been
    /// planned, and `0` for a sound-only render.
    pub of: u64,
}

impl Reading {
    /// How far along the render is, from `0` to `100`, counting frames only
    /// (the module doc has why). `100` means the file is finished.
    pub fn percent(&self) -> u8 {
        match self.phase {
            Phase::Waiting | Phase::Preparing | Phase::Mixing => 0,
            Phase::Drawing if self.of == 0 => 0,
            Phase::Drawing => {
                let share = self.done.min(self.of).saturating_mul(100) / self.of;
                u8::try_from(share.min(99)).unwrap_or(99)
            }
            Phase::Finishing => 99,
            Phase::Done => 100,
        }
    }
}

/// A shared progress readout, cloned between a render that writes it and
/// whoever reads it.
///
/// Every clone is the same readout. A [`crate::Renderer`] always has one, so a
/// render nobody watches pays one atomic store per frame. Handing the same one
/// to a second render starts it over from [`Phase::Preparing`].
#[derive(Debug, Clone, Default)]
pub struct Progress {
    state: Arc<State>,
}

#[derive(Debug, Default)]
struct State {
    phase: AtomicU8,
    done: AtomicU64,
    of: AtomicU64,
    /// For the tests only: every reading the render published, in order, so a
    /// test can check the sequence instead of racing a thread to sample it.
    #[cfg(test)]
    seen: std::sync::Mutex<Vec<Reading>>,
}

impl Progress {
    /// A readout at [`Phase::Waiting`], nothing done of nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Where the render is now. Safe from any thread, at any moment.
    pub fn read(&self) -> Reading {
        let phase = Phase::ALL
            .get(usize::from(self.state.phase.load(Ordering::SeqCst)))
            .copied()
            .unwrap_or(Phase::Waiting);
        let of = self.state.of.load(Ordering::SeqCst);
        let done = self.state.done.load(Ordering::SeqCst).min(of);
        Reading { phase, done, of }
    }

    /// A render starting over: nothing done, `of` not yet known.
    pub(crate) fn start(&self) {
        self.state.done.store(0, Ordering::SeqCst);
        self.state.of.store(0, Ordering::SeqCst);
        self.enter(Phase::Preparing);
    }

    /// The timeline is planned and will be `of` frames.
    pub(crate) fn planned(&self, of: u64) {
        self.state.of.store(of, Ordering::SeqCst);
        self.publish();
    }

    /// The render has moved on to `phase`.
    pub(crate) fn enter(&self, phase: Phase) {
        let index = Phase::ALL.iter().position(|p| *p == phase).unwrap_or(0);
        self.state
            .phase
            .store(u8::try_from(index).unwrap_or(0), Ordering::SeqCst);
        self.publish();
    }

    /// `done` frames are encoded.
    pub(crate) fn drew(&self, done: u64) {
        self.state.done.store(done, Ordering::SeqCst);
        self.publish();
    }

    /// Records what a reader would see now, for the tests; nothing otherwise.
    fn publish(&self) {
        #[cfg(test)]
        self.state
            .seen
            .lock()
            .expect("nothing panics holding it")
            .push(self.read());
    }

    /// Every reading published so far, in order.
    #[cfg(test)]
    pub(crate) fn seen(&self) -> Vec<Reading> {
        self.state
            .seen
            .lock()
            .expect("nothing panics holding it")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(phase: Phase, done: u64, of: u64) -> Reading {
        Reading { phase, done, of }
    }

    #[test]
    fn a_new_readout_is_waiting_on_nothing() {
        assert_eq!(Progress::new().read(), at(Phase::Waiting, 0, 0));
    }

    #[test]
    fn every_clone_is_the_same_readout() {
        let watched = Progress::new();
        let render = watched.clone();
        render.start();
        render.planned(10);
        render.enter(Phase::Drawing);
        render.drew(4);
        assert_eq!(watched.read(), at(Phase::Drawing, 4, 10));
        render.start();
        assert_eq!(watched.read(), at(Phase::Preparing, 0, 0), "starts over");
    }

    #[test]
    fn the_percentage_counts_frames_and_keeps_100_for_the_finished_file() {
        assert_eq!(at(Phase::Preparing, 0, 90).percent(), 0);
        assert_eq!(at(Phase::Mixing, 0, 90).percent(), 0);
        assert_eq!(at(Phase::Drawing, 0, 0).percent(), 0);
        assert_eq!(at(Phase::Drawing, 1, 3).percent(), 33);
        assert_eq!(at(Phase::Drawing, 45, 90).percent(), 50);
        assert_eq!(at(Phase::Drawing, 90, 90).percent(), 99);
        assert_eq!(at(Phase::Finishing, 90, 90).percent(), 99);
        assert_eq!(at(Phase::Finishing, 0, 0).percent(), 99);
        assert_eq!(at(Phase::Done, 90, 90).percent(), 100);
    }

    #[test]
    fn a_reading_never_has_more_done_than_of() {
        let progress = Progress::new();
        progress.drew(7);
        assert_eq!(progress.read().done, 0);
        assert_eq!(at(Phase::Drawing, 9, 3).percent(), 99);
    }

    #[test]
    fn phases_read_back_as_written_in_order() {
        let progress = Progress::new();
        for phase in Phase::ALL {
            progress.enter(phase);
            assert_eq!(progress.read().phase, phase);
        }
        assert!(Phase::ALL.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
