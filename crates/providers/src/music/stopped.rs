//! A run of music that stopped part-way, and what it had already done — the
//! shared carrier, [`crate::Stopped`] (#1153), counted in tracks.

use scorsese_core::AssetId;

use super::{MusicError, Outcome};
use crate::generated::{Counted, tell};

/// Why a run of music stopped, with every track it reached before it did.
pub type Stopped = crate::Stopped<Vec<(AssetId, Outcome)>, MusicError>;

/// How tracks are counted when a run stops.
const TRACKS: Counted = Counted {
    one: "track",
    many: "tracks",
    made: "composed",
    then: "What was composed is kept, so generating again pays only for the rest",
};

impl Stopped {
    /// The run in one sentence: what stopped it, how many tracks were
    /// composed and which were not.
    pub fn says(&self) -> String {
        let composed = self
            .run
            .iter()
            .filter(|(_, outcome)| outcome.spent_cents() > 0)
            .count();
        tell(&self.error, composed, &self.unsent, &TRACKS)
    }
}
