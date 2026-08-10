//! Generated video: briefs in, files in `generated/`, asset state updated.
//!
//! The other kind of brief. [`synth`](crate::synth) realises a *recipe* —
//! locally, for free, deterministically. This realises a **prompt**: it needs a
//! key, needs a network, costs money, and cannot be reproduced from the project
//! alone. The two are kept apart deliberately, and this module doc is the third
//! place that says so, because collapsing them is the mistake that would make
//! an expensive call look like a cheap one.
//!
//! # Three things hold this together
//!
//! **The ticket lives in the document.** A submitted generation writes its
//! `operation` and `queued_at` into `project.json` before anything else can go
//! wrong, so waiting is not something a process has to sit and do. Close the
//! app, restart the machine, `scp -r` the project somewhere else — whoever
//! opens it next can [`collect`] what finished.
//!
//! **An unchanged brief is never paid for twice.** A generation lands at
//! `generated/<asset-id>-<hash of the brief>.mp4`, and that file existing is
//! the answer to *has this been paid for*, whatever the document claims about
//! the asset's state. The hash covers every field of the request **and the
//! bytes of every still** — see [`Brief::digest`].
//!
//! **The provider is a trait, so no test ever spends a cent.** [`VideoProvider`]
//! is what the state machine talks to; [`VeoProvider`] is one implementation
//! and the tests' mock is another. The repo rule against real provider calls in
//! tests is satisfied by construction here rather than by anyone remembering.
//!
//! # What is an error, and what is an outcome
//!
//! An [`Outcome`] is what happened to **one asset** — cached, queued, still
//! waiting, generated, refused by the provider. A [`GenerationError`] stops the
//! **whole run**: no key, over budget, a still that will not read. The split is
//! load-bearing: a run of twenty shots where one prompt is rejected must leave
//! nineteen generated and say which one was not, because the alternative is
//! paying for nineteen again.

mod brief;
mod error;
mod patience;
mod provider;
mod run;
mod veo;

use scorsese_core::{ProjectPath, Timestamp};

pub use brief::{Brief, Still};
pub use error::GenerationError;
pub use patience::{POLL_EVERY, Run, WAIT_FOR, generate_waiting};
pub use provider::{Progress, ProviderError, Ready, Ticket, VideoProvider};
pub use run::{Plan, RETENTION_DAYS, collect, generate, pending, plan};
pub use veo::VeoProvider;

/// What happened to one asset in a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The brief's file was already there. Nothing was sent and nothing spent.
    Cached {
        /// Where it is, project-relative.
        path: ProjectPath,
    },
    /// Handed to the provider on this run. **This is the outcome that spent
    /// money**, and the estimate is what it is expected to have cost.
    Queued {
        /// The provider's name for the work.
        operation: String,
        /// What it was calculated to cost, in US cents. Our arithmetic over a
        /// published rate table, never a bill — see [`crate::prices`].
        estimated_cost_cents: u64,
    },
    /// Handed over on an earlier run and still in flight.
    Waiting {
        /// The provider's name for the work.
        operation: String,
        /// When it was handed over.
        queued_at: Option<Timestamp>,
    },
    /// In flight past the window the provider keeps a finished video in.
    ///
    /// Its own outcome rather than a longer wait, because the two call for
    /// opposite things: waiting means come back later, and this means the money
    /// is gone and the shot has to be asked for again.
    Expired {
        /// The provider's name for the work.
        operation: String,
        /// When it was handed over.
        queued_at: Option<Timestamp>,
    },
    /// Collected on this run and written into `generated/`.
    Generated {
        /// Where it landed, project-relative.
        path: ProjectPath,
        /// How big it is.
        bytes: usize,
        /// What it was calculated to cost, in US cents.
        estimated_cost_cents: u64,
    },
    /// The provider took the brief and then refused it. The asset goes back to
    /// `sketch` so the prompt can be edited and tried again.
    Failed {
        /// The provider's own sentence, which is the part worth reading.
        message: String,
    },
}

impl Outcome {
    /// Whether anything is still owed on this asset — a poll to make, a video
    /// to collect. What a caller loops on.
    pub fn is_in_flight(&self) -> bool {
        matches!(self, Self::Waiting { .. } | Self::Queued { .. })
    }

    /// What this outcome is expected to have cost, in US cents.
    ///
    /// Zero for everything that spent nothing, which includes a cache hit — the
    /// point of the cache — and a poll of work paid for on an earlier run,
    /// which would otherwise be counted again every time somebody asked after
    /// it.
    pub fn spent_cents(&self) -> u64 {
        match self {
            Self::Queued {
                estimated_cost_cents,
                ..
            } => *estimated_cost_cents,
            _ => 0,
        }
    }
}
