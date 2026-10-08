//! What an image provider is, from scorsese's side of the line.
//!
//! One call to draw now, for [`SpeechProvider`](crate::speech::SpeechProvider)'s
//! reason: the picture comes back on the connection that asked for it, so a
//! ticket would name work that was never in flight. And two for a **batch**
//! (#894) — order, then ask — because a batch is the one way of drawing that
//! *is* in flight, for up to a day. A trait, so the lifecycle in [`super`] is
//! driven by the tests without a network or a cent.

use super::Brief;

pub use crate::video::ProviderError;

/// Somewhere a brief can be turned into a picture.
pub trait ImageProvider {
    /// Draws the brief and hands back the picture's bytes.
    ///
    /// **This is a call that spends the money** — the other is
    /// [`ImageProvider::order`].
    fn draw(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError>;

    /// What this provider is called, for a message somebody reads.
    fn name(&self) -> &'static str;

    /// Orders every brief as one half-price batch job and answers with its
    /// name — the ticket. The briefs share one model; the caller groups them.
    ///
    /// **This spends the money**, half of [`ImageProvider::draw`]'s, whenever
    /// the vendor gets to the work. A provider that takes no batches says so,
    /// which is the default.
    fn order(&self, briefs: &[&Brief]) -> Result<String, ProviderError> {
        let _ = briefs;
        Err(ProviderError::new(
            self.name(),
            "this provider takes no batches",
        ))
    }

    /// Asks after the batch `operation`. Free.
    fn ask(&self, operation: &str) -> Result<Batch, ProviderError> {
        let _ = operation;
        Err(ProviderError::new(
            self.name(),
            "this provider takes no batches",
        ))
    }
}

/// Where a batch job is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Batch {
    /// Waiting or drawing.
    Running,
    /// Every answer is in: each key the job was sent with, and its picture or
    /// why there is none.
    Finished(Vec<(String, Result<Vec<u8>, String>)>),
    /// Stopped without answers — failed, cancelled, or expired unfinished —
    /// in the vendor's words. Nothing was drawn and nothing is billed.
    Stopped(String),
}
