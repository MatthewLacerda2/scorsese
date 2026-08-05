//! Waiting a while before detaching.
//!
//! **Detach after a few minutes, rather than detach by default.** Most
//! generations finish inside five, and a command that returns a ticket for work
//! that would have been done before anybody read the reply makes the caller
//! write the polling loop instead — badly, and differently each time. But
//! waiting *indefinitely* would put a video's whole generation on the lifetime
//! of a process, which is the thing the ticket-in-the-document exists to
//! prevent.
//!
//! So: wait, then stop waiting. Nothing is lost when the wait runs out — the
//! ticket is in `project.json` and [`collect`](super::collect) picks it up
//! whenever anybody comes back.

use std::path::Path;
use std::time::{Duration, Instant};

use scorsese_core::{AssetId, Project};

use crate::credentials::Budget;

use super::{GenerationError, Outcome, VideoProvider, collect, generate};

/// How long to leave between polls.
///
/// Seconds, not milliseconds: the work being waited on takes minutes, and a
/// tighter loop would be a request per second at a vendor to learn the same
/// thing.
pub const POLL_EVERY: Duration = Duration::from_secs(10);

/// How long a caller who says nothing waits before detaching.
pub const WAIT_FOR: Duration = Duration::from_secs(300);

/// One run of [`generate_waiting`]: what happened, and what it cost.
///
/// The spend is carried here rather than read back off the outcomes, and that
/// is not tidiness — it is the only place it survives. A shot submitted on this
/// run is [`Outcome::Queued`], which is the outcome that knows it was paid for;
/// when the wait then collects it, that outcome is replaced by
/// [`Outcome::Generated`], which cannot know whether the money went today or
/// last week. Totalling as we go is what keeps a successful run from reporting
/// that it spent nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// What happened to each asset, in the order the assets appear.
    pub outcomes: Vec<(AssetId, Outcome)>,
    /// What this run is estimated to have spent, in US cents. Our arithmetic
    /// over a published rate table, never a bill.
    pub spent_cents: u64,
}

impl Run {
    /// How many shots are still going.
    pub fn in_flight(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|(_, outcome)| outcome.is_in_flight())
            .count()
    }
}

/// Submits everything, then waits up to `patience` for it to finish.
///
/// Returns as soon as nothing is in flight, which for a run that was entirely
/// cache hits is immediately. Whatever is still going when the patience runs
/// out is reported as [`Outcome::Waiting`] and is nobody's emergency: it is in
/// the document.
pub fn generate_waiting(
    project: &mut Project,
    root: &Path,
    provider: &dyn VideoProvider,
    budget: Budget,
    patience: Duration,
    mut rest: impl FnMut(Duration),
) -> Result<Run, GenerationError> {
    let started = Instant::now();
    let mut done = generate(project, root, provider, budget)?;
    // Read now, while the shots submitted on this run still say so.
    let spent_cents = done.iter().map(|(_, outcome)| outcome.spent_cents()).sum();

    while done.iter().any(|(_, outcome)| outcome.is_in_flight()) {
        if started.elapsed() >= patience {
            break;
        }
        rest(POLL_EVERY);
        // A sweep rather than another `generate`: everything that could be
        // submitted already has been, and a second submitting pass inside a
        // waiting loop is how one bad condition becomes twenty generations.
        let swept = collect(project, root, provider)?;
        for (id, outcome) in swept {
            match done.iter_mut().find(|(known, _)| *known == id) {
                Some(slot) => slot.1 = outcome,
                None => done.push((id, outcome)),
            }
        }
    }
    Ok(Run {
        outcomes: done,
        spent_cents,
    })
}
