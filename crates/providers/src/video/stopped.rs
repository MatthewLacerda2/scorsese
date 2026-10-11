//! A run that stopped part-way, and what it had already done.
//!
//! **An error after spending is still a report of the spending.** A run of
//! five shots that is refused at the fourth — Veo at its limit of concurrent
//! generations, the network gone, the ceiling reached — has paid for three, and
//! their tickets are in the document. Answering with the error alone tells
//! whoever is paying that nothing happened, and an agent told that retries
//! (#1057). So the error carries the run it ended: what each shot did, what was
//! committed, and which shots were never sent.

use scorsese_core::AssetId;

use super::{GenerationError, Run};

/// Why a run stopped, with everything it did before it did.
///
/// Reads as the error alone, so a caller that only wants the sentence — the
/// desktop app's status line, a test asserting the ceiling — sees exactly what
/// it saw before this type existed.
#[derive(Debug, thiserror::Error)]
#[error("{error}")]
pub struct Stopped {
    /// What the run had done: each shot it reached, and what was committed.
    pub run: Run,
    /// The shots that would have been sent and were not — the one the run
    /// stopped on included. Empty when the run stopped while waiting, after
    /// every submission had been made.
    pub unsent: Vec<AssetId>,
    /// What stopped it.
    #[source]
    pub error: GenerationError,
}

impl Stopped {
    /// The run in one sentence: what stopped it, how many shots went and how
    /// many did not.
    pub fn says(&self) -> String {
        let sent = self
            .run
            .outcomes
            .iter()
            .filter(|(_, outcome)| outcome.spent_cents() > 0)
            .count();
        let unsent = self.unsent.len();
        if unsent == 0 {
            return format!(
                "The run stopped while waiting on the shots: {} — every shot it was to send \
                 had been sent, and their tickets are in project.json",
                self.error
            );
        }
        format!(
            "The run stopped: {} — {sent} {} sent, {unsent} {} not ({}); nothing after it \
             was attempted. Run generate again once the shots in flight finish",
            self.error,
            shots(sent),
            shots(unsent),
            self.unsent
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// "shot was" or "shots were".
fn shots(count: usize) -> &'static str {
    if count == 1 { "shot was" } else { "shots were" }
}
