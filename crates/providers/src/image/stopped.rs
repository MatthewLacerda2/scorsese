//! A run of stills that stopped part-way, and what it had already done.
//!
//! The carrier is [`crate::Stopped`], shared with shots and narration
//! (#1153). The realistic stop is the ceiling reached at the fourth still:
//! the three before it were drawn and paid for, the document records them,
//! and the reply has to say so rather than answer with the ceiling alone.

use std::path::Path;

use scorsese_core::{AssetId, Project};

use super::run::Stills;
use super::{ImageError, Outcome};
use crate::generated::{self, Counted, tell};

/// Why a run of stills stopped, with every still it reached before it did.
pub type Stopped = crate::Stopped<Vec<(AssetId, Outcome)>, ImageError>;

/// How stills are counted when a run stops.
const STILLS: Counted = Counted {
    one: "still",
    many: "stills",
    made: "paid for",
    then: "What was drawn or ordered is kept, so generating again pays only for the rest",
};

impl Stopped {
    /// The run in one sentence: what stopped it, how many stills were paid
    /// for — drawn now or ordered in a batch — and which were not.
    pub fn says(&self) -> String {
        let paid = self
            .run
            .iter()
            .filter(|(_, outcome)| outcome.spent_cents() > 0)
            .count();
        tell(&self.error, paid, &self.unsent, &STILLS)
    }
}

/// Ends a run on `error`, carrying `done` and every still it would still
/// have sent — never one that was not ready when it began, which it would
/// not have sent anyway (#1058).
pub(super) fn stopped(
    project: &Project,
    root: &Path,
    done: Vec<(AssetId, Outcome)>,
    unready: &[(AssetId, String)],
    error: ImageError,
) -> Stopped {
    let left: Vec<AssetId> = generated::ids::<Stills>(project)
        .into_iter()
        .filter(|id| !done.iter().any(|(reached, _)| reached == id))
        .filter(|id| !unready.iter().any(|(waiting, _)| waiting == id))
        .collect();
    Stopped {
        unsent: generated::unsent::<Stills>(project, root, &left),
        run: done,
        error,
    }
}
