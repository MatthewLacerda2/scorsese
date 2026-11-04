//! Generated sound effects: descriptions in, MP3s in `generated/`, asset
//! state updated (#1265).
//!
//! The sibling of [`music`](crate::music), and the same shape: one call, the
//! audio on the same connection, nothing in flight and nothing to resume. A
//! sound is a `generated_audio` asset with a `sound` block
//! ([`scorsese_core::AudioBlock::Sound`]); the narration and music passes
//! never see one, nor this pass a line or a track.
//!
//! **A recipe comes first.** What a `synth_audio` patch can make costs
//! nothing and is reproducible; this pass is for what one cannot — a real
//! door, a crowd, rain on a window.
//!
//! **Shared with the other prompted kinds:** the provider is a trait, so no
//! test spends a cent; a sound lands at
//! `generated/<asset-id>-<hash of the brief>.mp3`, and that file existing is
//! the answer to *has this been paid for*; the ceiling and the price
//! arithmetic are the same machinery. Like a track, a sound reaches the
//! document with no `media`: **whatever calls this probes afterwards**.

mod brief;
mod elevenlabs;
mod error;
mod provider;
mod run;
mod stopped;

use scorsese_core::ProjectPath;

pub use brief::Brief;
pub use elevenlabs::ElevenLabsSounds;
pub use error::{Incomplete, SoundError};
pub use provider::{ProviderError, SoundProvider};
pub use run::{adopt, generate, pending, plan, quote};
pub use stopped::Stopped;

/// The vendor model every sound is made with.
pub use crate::api::elevenlabs::sound::MODEL_ID;

/// What a run would do with one sound — [`speech::Plan`](crate::speech::Plan),
/// whose shape is the same for the same reason: nothing is ever in flight.
pub use crate::speech::Plan;

/// What happened to one sound in a run — [`speech::Outcome`](crate::speech::Outcome)'s
/// four, in its own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The brief's file was already there. Nothing was sent and nothing spent.
    Cached {
        /// Where it is, project-relative.
        path: ProjectPath,
    },
    /// Made on this run and written into `generated/`. **This is the outcome
    /// that spent money.**
    Generated {
        /// Where it landed, project-relative.
        path: ProjectPath,
        /// How big it is.
        bytes: usize,
        /// What it was calculated to cost, in US cents. Our arithmetic over a
        /// published rate, never a bill — see [`crate::prices`].
        estimated_cost_cents: u64,
    },
    /// Not ready to be made, and nothing was sent — a sound with no length
    /// yet is a cut being written, not a run to abandon.
    Incomplete {
        /// What it is still missing, in the words [`Incomplete`] puts it.
        why: String,
    },
    /// The provider refused it, or answered with nothing to keep. Nothing
    /// was written, and the asset is left as it was so the brief can be
    /// edited and retried; a refusal the vendor bills is in the ledger at its
    /// quote all the same.
    Failed {
        /// The provider's own sentence.
        message: String,
        /// What this run was charged for it, in US cents: the quote, when the
        /// vendor completed the call and bills it
        /// ([`billed`](ProviderError::billed)); zero when it does not.
        estimated_cost_cents: u64,
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
            }
            | Self::Failed {
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
            Self::Cached { path } => format!("already made — {path}"),
            Self::Generated { path, bytes, .. } => format!("made — {path} ({bytes} bytes)"),
            Self::Incomplete { why } => format!("not yet — {why}"),
            Self::Failed {
                message,
                estimated_cost_cents,
            } => crate::generated::refused(message, *estimated_cost_cents),
        }
    }
}
