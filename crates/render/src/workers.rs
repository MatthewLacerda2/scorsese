//! How many threads composite at once.
//!
//! A render's own choice, not the project's: it says nothing about what the
//! file will contain, only how much of the machine is spent producing it —
//! which is why it lives here rather than in [`RenderSettings`], where
//! everything decides something about the delivered file.
//!
//! [`RenderSettings`]: crate::RenderSettings

use std::thread::available_parallelism;

/// How many frames a render composites at the same time.
///
/// Never zero: a pool of no workers is a render that never finishes, so both
/// ways of making one take the floor rather than refusing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Workers(usize);

impl Workers {
    /// One fewer than the machine says it can run at once, and at least one.
    ///
    /// The thread left over is not politeness. A render is already running an
    /// ffmpeg per layer with a source and one more encoding, and the producer
    /// and reorder stages share a thread of their own — so a pool sized to
    /// every core would be taking cores from the processes that feed it.
    ///
    /// A machine that will not say how parallel it is counts as one.
    pub fn available() -> Self {
        Self(
            available_parallelism()
                .map(|threads| threads.get().saturating_sub(1))
                .unwrap_or(1)
                .max(1),
        )
    }

    /// Exactly this many, and at least one.
    ///
    /// What `--threads` reaches, and it is not a nice-to-have: a container told
    /// it has eight cores when it has been granted two would otherwise
    /// oversubscribe itself, and headless-in-a-container is a first-class way
    /// scorsese runs.
    pub fn new(count: usize) -> Self {
        Self(count.max(1))
    }

    /// How many there are.
    pub const fn get(self) -> usize {
        self.0
    }
}

impl Default for Workers {
    /// [`Workers::available`] — what a render does when nobody said.
    fn default() -> Self {
        Self::available()
    }
}
