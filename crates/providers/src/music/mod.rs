//! Generated music: descriptions in, MP3s in `generated/`, asset state
//! updated (#1264).
//!
//! The sibling of [`speech`](crate::speech), and the same shape: one call,
//! the audio on the same connection, nothing in flight and nothing to resume.
//! A track is a `generated_audio` asset with a `music` block
//! ([`scorsese_core::AudioBlock::Music`]); every other `generated_audio`
//! asset is a line, and the narration pass never sees a track — nor this pass
//! a line.
//!
//! **Shared with speech and the other prompted kinds:** the provider is a
//! trait, so no test spends a cent; a track lands at
//! `generated/<asset-id>-<hash of the brief>.mp3`, and that file existing is
//! the answer to *has this been paid for*; the ceiling and the price
//! arithmetic are the same machinery.
//!
//! **Not shared:** word timings, since nothing is said; and a voice, since
//! nobody says it. What a track needs before it is paid for is a description
//! and a length — see [`Incomplete`].
//!
//! Like a line, a track reaches the document with no `media`: this crate has
//! no decoder, so **whatever calls this probes afterwards**.

mod brief;
mod elevenlabs;
mod error;
mod provider;
mod run;
mod stopped;

use scorsese_core::ProjectPath;

pub use brief::Brief;
pub use elevenlabs::ElevenLabsComposer;
pub use error::{Incomplete, MusicError};
pub use provider::{MusicProvider, ProviderError};
pub use run::{adopt, generate, pending, plan, quote};
pub use stopped::Stopped;

/// What a run would do with one track — [`speech::Plan`](crate::speech::Plan),
/// whose shape is the same for the same reason: nothing is ever in flight.
pub use crate::speech::Plan;

/// What happened to one track in a run — [`speech::Outcome`](crate::speech::Outcome)'s
/// four, in its own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The brief's file was already there. Nothing was sent and nothing spent.
    Cached {
        /// Where it is, project-relative.
        path: ProjectPath,
    },
    /// Composed on this run and written into `generated/`. **This is the
    /// outcome that spent money.**
    Generated {
        /// Where it landed, project-relative.
        path: ProjectPath,
        /// How big it is.
        bytes: usize,
        /// What it was calculated to cost, in US cents. Our arithmetic over a
        /// published rate, never a bill — see [`crate::prices`].
        estimated_cost_cents: u64,
    },
    /// Not ready to be composed, and nothing was sent — a track with no
    /// length yet is a cut being written, not a run to abandon.
    Incomplete {
        /// What it is still missing, in the words [`Incomplete`] puts it.
        why: String,
    },
    /// The provider took the request and refused it. Nothing was written, and
    /// the asset is left as it was so the brief can be edited and retried.
    Failed {
        /// The provider's own sentence.
        message: String,
    },
}

impl Outcome {
    /// What this outcome is expected to have cost, in US cents — zero for
    /// everything that spent nothing, a cache hit included.
    pub fn spent_cents(&self) -> u64 {
        match self {
            Self::Generated {
                estimated_cost_cents,
                ..
            } => *estimated_cost_cents,
            _ => 0,
        }
    }

    /// The sentence this outcome reads as, after an asset's id — phrased
    /// here, once, for every client that reports a run.
    pub fn says(&self) -> String {
        match self {
            Self::Cached { path } => format!("already composed — {path}"),
            Self::Generated { path, bytes, .. } => format!("composed — {path} ({bytes} bytes)"),
            Self::Incomplete { why } => format!("not yet — {why}"),
            Self::Failed { message } => format!("refused — {message}"),
        }
    }
}
