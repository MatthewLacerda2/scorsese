//! A run that stopped part-way, and what it had already done — one carrier
//! for a shot, a still and a line (#1057, #1153).
//!
//! **An error after spending is still a report of the spending.** A run of
//! five that is refused at the fourth — Veo at its limit of shots in flight,
//! the network gone, the ceiling reached, a disk that will not take a file —
//! has paid for three, and the document records them. Answering with the
//! error alone tells whoever is paying that nothing happened, and an agent
//! told that retries. So the error carries the run it ended: what each asset
//! did, and which would have been sent and were not.
//!
//! The carrier is generic over what a run *is*: a shot's [`Run`](crate::video::Run)
//! keeps its own total, because a collected shot no longer says it was paid
//! for today; a still's or a line's is its outcomes. Each kind's `says`
//! counts what it made in its own words, and the sentence around the count is
//! [`tell`]'s, once.

use std::fmt::{self, Display};

use scorsese_core::{AssetId, Project};

use super::Plan;
use super::prompted::{Prompted, plan};

/// Why a run stopped, with everything it did before it did.
///
/// Reads as the error alone, so a caller that only wants the sentence — a
/// test asserting the ceiling, an old caller that maps it to a string — sees
/// exactly what it saw before this type existed. `says` is the whole story.
#[derive(Debug)]
pub struct Stopped<R, E> {
    /// What the run had done: each asset it reached, in document order.
    pub run: R,
    /// The assets that would have been sent and were not — the one the run
    /// stopped on included. Empty when nothing was left to send.
    pub unsent: Vec<AssetId>,
    /// What stopped it.
    pub error: E,
}

impl<R, E: Display> Display for Stopped<R, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<R: fmt::Debug, E: std::error::Error + 'static> std::error::Error for Stopped<R, E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// How one kind is counted in a stopped run's sentence — `shot` / `shots`,
/// made means `sent` — and what to do about it.
pub(crate) struct Counted {
    /// One of them.
    pub(crate) one: &'static str,
    /// More than one, or none.
    pub(crate) many: &'static str,
    /// What happened to one that was paid for on this run.
    pub(crate) made: &'static str,
    /// What to do next, said last.
    pub(crate) then: &'static str,
}

impl Counted {
    /// "3 stills were", "1 line was".
    fn count(&self, count: usize) -> String {
        if count == 1 {
            format!("1 {} was", self.one)
        } else {
            format!("{count} {} were", self.many)
        }
    }
}

/// The sentence a stopped run reads as: the reason, how many of `kind` went
/// and which did not, that nothing after the stop was attempted, and what to
/// do next.
pub(crate) fn tell(error: &dyn Display, made: usize, unsent: &[AssetId], kind: &Counted) -> String {
    let went = format!("{} {}", kind.count(made), kind.made);
    if unsent.is_empty() {
        return format!(
            "The run stopped: {error} — {went}; nothing after it was attempted. {}",
            kind.then
        );
    }
    let ids: Vec<String> = unsent.iter().map(ToString::to_string).collect();
    format!(
        "The run stopped: {error} — {went}, {} not ({}); nothing after it was attempted. {}",
        kind.count(unsent.len()),
        ids.join(", "),
        kind.then
    )
}

/// Which of `ids` a run would have sent — what a stopped run never reached.
pub(crate) fn unsent<K: Prompted>(
    project: &Project,
    root: &std::path::Path,
    ids: &[AssetId],
) -> Vec<AssetId> {
    ids.iter()
        .filter(|id| {
            project
                .asset(id)
                .is_some_and(|asset| plan::<K>(project, root, asset) == Plan::Submit)
        })
        .cloned()
        .collect()
}
