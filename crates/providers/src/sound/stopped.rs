//! A run of sound effects that stopped part-way, and what it had already
//! done — the shared carrier, [`crate::Stopped`] (#1153), counted in sounds.

use scorsese_core::AssetId;

use super::{Outcome, SoundError};
use crate::generated::{Counted, tell_billed};

/// Why a run of sound effects stopped, with every sound it reached before it
/// did.
pub type Stopped = crate::Stopped<Vec<(AssetId, Outcome)>, SoundError>;

/// How sounds are counted when a run stops.
const SOUNDS: Counted = Counted {
    one: "sound",
    many: "sounds",
    made: "made",
    then: "What was made is kept, so generating again pays only for the rest",
};

impl Stopped {
    /// The run in one sentence: what stopped it, how many sounds were made
    /// and which were not — and any refused that the vendor billed anyway.
    pub fn says(&self) -> String {
        let made = self
            .run
            .iter()
            .filter(|(_, outcome)| matches!(outcome, Outcome::Generated { .. }))
            .count();
        let billed: Vec<&AssetId> = self
            .run
            .iter()
            .filter(|(_, outcome)| matches!(outcome, Outcome::Failed { .. }))
            .filter(|(_, outcome)| outcome.spent_cents() > 0)
            .map(|(id, _)| id)
            .collect();
        tell_billed(&self.error, made, &self.unsent, &billed, &SOUNDS)
    }
}
