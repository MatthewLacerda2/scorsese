//! A run of narration that stopped part-way, and what it had already done.
//!
//! The carrier is [`crate::Stopped`], shared with shots and stills (#1153).
//! The realistic stop is the ceiling reached at the fifth line: the four
//! before it were spoken and paid for, the document records them, and the
//! reply has to say so rather than answer with the ceiling alone.

use scorsese_core::AssetId;

use super::{Outcome, SpeechError};
use crate::generated::{Counted, tell};

/// Why a run of narration stopped, with every line it reached before it did.
pub type Stopped = crate::Stopped<Vec<(AssetId, Outcome)>, SpeechError>;

/// How lines are counted when a run stops.
const LINES: Counted = Counted {
    one: "line",
    many: "lines",
    made: "spoken",
    then: "What was spoken is kept, so generating again pays only for the rest",
};

impl Stopped {
    /// The run in one sentence: what stopped it, how many lines were spoken
    /// and which were not.
    pub fn says(&self) -> String {
        let spoken = self
            .run
            .iter()
            .filter(|(_, outcome)| outcome.spent_cents() > 0)
            .count();
        tell(&self.error, spoken, &self.unsent, &LINES)
    }
}
