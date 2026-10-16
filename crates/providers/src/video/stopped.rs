//! A run of shots that stopped part-way, and what it had already done.
//!
//! The carrier is [`crate::Stopped`], shared with stills and narration
//! (#1153); what is the shots' own is the sentence. A shot paid for on this
//! run is one that was *sent* — its ticket is in the document — and a run can
//! also stop *after* every submission, while waiting, which neither a still
//! nor a line can.

use super::{GenerationError, Run};
use crate::generated::{Counted, tell};

/// Why a run of shots stopped, with everything it did before it did (#1057).
pub type Stopped = crate::Stopped<Run, GenerationError>;

/// How shots are counted when a run stops.
const SHOTS: Counted = Counted {
    one: "shot",
    many: "shots",
    made: "sent",
    then: "Run generate again once the shots in flight finish",
};

impl Stopped {
    /// The run in one sentence: what stopped it, how many shots went and how
    /// many did not.
    pub fn says(&self) -> String {
        if self.unsent.is_empty() {
            return format!(
                "The run stopped while waiting on the shots: {} — every shot it was to send \
                 had been sent, and their tickets are in project.json",
                self.error
            );
        }
        let sent = self
            .run
            .outcomes
            .iter()
            .filter(|(_, outcome)| outcome.spent_cents() > 0)
            .count();
        tell(&self.error, sent, &self.unsent, &SHOTS)
    }
}
