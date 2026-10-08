//! Generated stills: prompts in, JPEGs in `generated/`, asset state updated.
//!
//! The third prompted provider, and the cheapest picture scorsese can buy: a
//! 2K still is about a tenth of eight seconds of Veo, and unlike a shot it is
//! reused — the same backdrop three scenes later, the same character sheet
//! behind every picture of that character.
//!
//! # Shaped like [`speech`](crate::speech), briefed like [`video`](crate::video)
//!
//! **Like speech:** one call, and the picture comes back on it. A drawing is
//! seconds, not minutes, so a still drawn now has no ticket and no `queued`
//! state. A brief that cannot be gathered is an [`Outcome::Incomplete`] for
//! that asset, not the end of the run.
//!
//! # Or half price, within a day
//!
//! The one exception is chosen, never defaulted: a still ordered in a
//! **batch** ([`Order::Batch`], #894) costs half and comes back within 24
//! hours, so for that while it is shaped like a shot — `queued`, with the
//! batch job's name as its `operation`, collected by a later run ([`collect`],
//! and every [`generate`] collects first). Whether a still was batched is how
//! it was ordered, not what it is: it is not in the brief, the fingerprint or
//! the file name, so a batched picture and one drawn now are the same cache
//! entry.
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

mod batch;
mod brief;
mod collect;
mod error;
pub(crate) mod gemini;
mod provider;
mod run;

use scorsese_core::ProjectPath;

pub use batch::batch;
pub use brief::Brief;
pub use collect::collect;
pub use error::{ImageError, Incomplete};
pub use gemini::GeminiProvider;
pub use provider::{Batch, ImageProvider, ProviderError};
pub use run::{Plan, adopt, generate, pending, plan};

/// How a still is ordered: drawn on the call, or in a half-price batch that
/// answers within 24 hours (#894). Not part of the brief — see the module.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Order {
    /// Drawn on the call that asks, at the standard rate. The default.
    #[default]
    Now,
    /// Ordered as a batch job at half the rate, collected by a later run.
    Batch,
}

/// What happened to one still in a run — [`speech::Outcome`](crate::speech::Outcome)'s
/// four, and two more for a still ordered in a batch.
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
    /// Ordered in a batch on this run, its ticket written into the document.
    /// **This outcome spent money too** — half of [`Outcome::Generated`]'s,
    /// committed now and billed when the vendor gets to it.
    Ordered {
        /// The batch job's name.
        operation: String,
        /// What it was calculated to cost, at the batch rate.
        estimated_cost_cents: u64,
    },
    /// Collected from its batch and written into `generated/`. Spent nothing
    /// on this run: the run that ordered it did.
    Collected {
        /// Where it landed, project-relative.
        path: ProjectPath,
        /// How big it is.
        bytes: usize,
    },
    /// Still waiting in its batch. Nothing new was spent.
    Waiting {
        /// The batch job's name.
        operation: String,
    },
}

impl Outcome {
    /// Whether this outcome put a new file in `generated/` that nothing has
    /// measured yet — drawn now, or collected from a batch.
    pub fn landed(&self) -> bool {
        matches!(self, Self::Generated { .. } | Self::Collected { .. })
    }

    /// What this outcome is expected to have cost, in US cents — zero for
    /// everything that spent nothing, a cache hit and a collection included.
    pub fn spent_cents(&self) -> u64 {
        match self {
            Self::Generated {
                estimated_cost_cents,
                ..
            }
            | Self::Ordered {
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
            Self::Ordered {
                operation,
                estimated_cost_cents,
            } => format!(
                "ordered at half price, {} — ready within 24 hours; generate again to collect \
                 it ({operation})",
                crate::prices::dollars(*estimated_cost_cents)
            ),
            Self::Collected { path, bytes } => {
                format!("drawn in its batch — {path} ({bytes} bytes)")
            }
            Self::Waiting { operation } => {
                format!("still in its batch — collect again later ({operation})")
            }
        }
    }
}
