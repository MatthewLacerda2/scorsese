//! Generated speech: lines in, MP3s in `generated/`, asset state updated.
//!
//! The second prompted provider, and the cheap one. A line of narration costs
//! about two cents where eight seconds of Veo costs ninety-six, and it is the
//! thing in an edit that gets rewritten most — so the brief-hash cache is doing
//! more work here than anywhere else in scorsese, even though each hit saves
//! less.
//!
//! # What this shares with [`video`](crate::video), and what it does not
//!
//! **Shared:** the provider is a trait so no test spends a cent; a generation
//! lands at `generated/<asset-id>-<hash of the brief>.mp3` and that file
//! existing is the answer to *has this been paid for*; the ceiling and the
//! price arithmetic are the same machinery.
//!
//! **Not shared, and deliberately:** there is no ticket, no polling, and no
//! retention window, because speech is not long-running — see
//! [`SpeechProvider`]. And a brief that cannot be gathered does not stop the
//! run — see [`Incomplete`].
//!
//! # Word timings
//!
//! A line comes back with when each of its words is said (#811), and they are
//! kept beside the audio as `<the audio's name>.words.json` —
//! [`scorsese_core::words`] has the file. Content-addressed with the audio, so
//! a cache hit has whatever timings its generation came with, and a line spoken
//! before they were kept has none: it is never spoken again to get them.
//!
//! # The one thing this cannot do
//!
//! It cannot say how long a line came out. A shot is exactly as long as its
//! request asked; a reading is as long as the words take. Measuring an MP3
//! needs a decoder, this crate has none and must not grow one, so a generated
//! narration reaches the document with no `media` — and the mix reads
//! `duration_seconds`. **Whatever calls this has to probe afterwards**, which
//! is why the commands do.

mod brief;
mod elevenlabs;
mod error;
mod provider;
mod run;

use scorsese_core::ProjectPath;

pub use brief::Brief;
pub use elevenlabs::ElevenLabsProvider;
pub use error::{Incomplete, SpeechError};
pub use provider::{ProviderError, SpeechProvider, Spoken};
pub use run::{Plan, adopt, generate, pending, plan, quote};

/// What happened to one line in a run.
///
/// Four outcomes where video has six. The two missing ones are `Waiting` and
/// `Expired`, and their absence is the whole difference between the providers:
/// nothing here is ever in flight, so nothing can be waited on or outlive a
/// window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The brief's file was already there. Nothing was sent and nothing spent.
    Cached {
        /// Where it is, project-relative.
        path: ProjectPath,
    },
    /// Spoken on this run and written into `generated/`. **This is the outcome
    /// that spent money.**
    Generated {
        /// Where it landed, project-relative.
        path: ProjectPath,
        /// How big it is.
        bytes: usize,
        /// What it was calculated to cost, in US cents. Our arithmetic over a
        /// published rate table, never a bill — see [`crate::prices`].
        estimated_cost_cents: u64,
    },
    /// Not ready to be spoken, and nothing was sent.
    ///
    /// The ordinary state of a line somebody is still writing, which is why it
    /// is an outcome and not an error: a cut with one voiceless line in it is
    /// a cut being worked on, not a run to abandon.
    Incomplete {
        /// What it is still missing, in the words [`Incomplete`] puts it.
        why: String,
    },
    /// The provider took the line and refused it. Nothing was written, and the
    /// asset is left exactly as it was so the brief can be edited and retried.
    Failed {
        /// The provider's own sentence, which is the part worth reading.
        message: String,
    },
}

impl Outcome {
    /// What this outcome is expected to have cost, in US cents.
    ///
    /// Zero for everything that spent nothing, which includes a cache hit —
    /// the point of the cache.
    pub fn spent_cents(&self) -> u64 {
        match self {
            Self::Generated {
                estimated_cost_cents,
                ..
            } => *estimated_cost_cents,
            _ => 0,
        }
    }

    /// The sentence this outcome reads as, after an asset's id — phrased here,
    /// once, for every client that reports a run, so they cannot drift apart.
    pub fn says(&self) -> String {
        match self {
            Self::Cached { path } => format!("already spoken — {path}"),
            Self::Generated { path, bytes, .. } => format!("spoken — {path} ({bytes} bytes)"),
            // Not a failure and not phrased as one: a line nobody has chosen a
            // voice for yet is a cut being written, and the run carried on.
            Self::Incomplete { why } => format!("not yet — {why}"),
            Self::Failed { message } => format!("refused — {message}"),
        }
    }
}
