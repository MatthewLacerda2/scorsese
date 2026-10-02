//! Generated stills: prompts in, PNGs in `generated/`, asset state updated.
//!
//! The third prompted provider, and the cheapest picture scorsese can buy: a
//! 2K still is about a tenth of eight seconds of Veo, and unlike a shot it is
//! reused — the same backdrop three scenes later, the same character sheet
//! behind every picture of that character.
//!
//! # Shaped like [`speech`](crate::speech), briefed like [`video`](crate::video)
//!
//! **Like speech:** one call, and the picture comes back on it. A drawing is
//! seconds, not minutes, so there is no ticket, no `queued` state written into
//! the document and no retention window — every one of those would be a state
//! a reader has to rule out and that can never happen. A brief that cannot be
//! gathered is an [`Outcome::Incomplete`] for that asset, not the end of the
//! run.
//!
//! **Like video:** the brief names pictures by asset id and the fingerprint
//! hashes their bytes, so swapping a reference for another of the same name
//! is a new brief; and the provider is a trait, so no test spends a cent.
//!
//! # A still drawn from a still
//!
//! A reference may be another `generated_image` — the canonical character
//! sheet is the reason the field exists. It has to have been generated first,
//! and a run does **not** chain them: a still whose reference is still a sketch
//! is incomplete this run and drawn by the next. Chaining inside one run would
//! spend on a brief no quote could have priced, because its fingerprint
//! depends on a picture that did not exist when the quote was made.

mod brief;
mod error;
mod gemini;
mod provider;
mod run;

use scorsese_core::ProjectPath;

pub use brief::Brief;
pub use error::{ImageError, Incomplete};
pub use gemini::GeminiProvider;
pub use provider::{ImageProvider, ProviderError};
pub use run::{Plan, adopt, generate, pending, plan};

/// What happened to one still in a run — [`speech::Outcome`](crate::speech::Outcome)'s
/// four, for its reason: nothing here is ever in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The brief's file was already there. Nothing was sent and nothing spent.
    Cached {
        /// Where it is, project-relative.
        path: ProjectPath,
    },
    /// Drawn on this run and written into `generated/`. **This is the outcome
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
    /// Not ready to be drawn, and nothing was sent.
    Incomplete {
        /// What it is still missing, in the words [`Incomplete`] puts it.
        why: String,
    },
    /// The vendor took the brief and drew nothing. The asset is left as it was
    /// so the prompt can be edited and tried again.
    Failed {
        /// The vendor's own sentence, which is the part worth reading.
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

    /// The sentence this outcome reads as, after an asset's id — phrased here,
    /// once, for every client that reports a run, so they cannot drift apart.
    pub fn says(&self) -> String {
        match self {
            Self::Cached { path } => format!("already drawn — {path}"),
            Self::Generated { path, bytes, .. } => format!("drawn — {path} ({bytes} bytes)"),
            Self::Incomplete { why } => format!("not yet — {why}"),
            Self::Failed { message } => format!("refused — {message}"),
        }
    }
}
