//! What a speech provider is, from scorsese's side of the line.
//!
//! **One call, and that is the decision worth reading.**
//! [`VideoProvider`](crate::video::VideoProvider) is three — submit, poll,
//! fetch — because a Veo generation outlives the process that asked for it, and
//! its module doc anticipated this one fitting the same shape by answering
//! `Ready` on the first poll.
//!
//! Building it settled the question the other way. Speech returns the audio on
//! the same connection, so fitting it to three calls means inventing a ticket
//! for work that was never in flight, writing an `operation` into
//! `project.json` that is meaningless a microsecond later, and inheriting a
//! retention window for a file that arrived before anything could expire. Every
//! one of those is a state a reader has to rule out, and none of them can ever
//! happen.
//!
//! So the shape of the awkward provider turned out not to be the general one.
//! What the two genuinely share — [`ProviderError`], the brief-hash cache, the
//! budget, the price arithmetic — is shared; the part only long-running
//! generation needs is not.
//!
//! **Nothing here is a client.** A trait is what the lifecycle in [`super`]
//! talks to, which is what lets the tests drive it without a network or a cent.
//! The repo rule against real provider calls in tests is satisfied by
//! construction rather than by anyone remembering.

use scorsese_core::words::Words;

use super::Brief;

pub use crate::video::ProviderError;

/// A line, spoken: the audio, and when each word of it is said if the provider
/// said so (#811).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spoken {
    /// The MP3.
    pub audio: Vec<u8>,
    /// The word timings, kept beside the audio — `None` for a provider or a
    /// reply that had none, which leaves the line without them.
    pub words: Option<Words>,
}

impl From<Vec<u8>> for Spoken {
    /// Audio with no word timings: a provider that has none.
    fn from(audio: Vec<u8>) -> Self {
        Self { audio, words: None }
    }
}

/// Somewhere a line can be turned into audio.
pub trait SpeechProvider {
    /// Speaks a line and hands back the audio, with its word timings when the
    /// provider gives them.
    ///
    /// **This is the call that spends the money**, and there is no other — no
    /// ticket to collect against later, nothing to resume. What it returns is
    /// the whole result.
    fn speak(&self, brief: &Brief) -> Result<Spoken, ProviderError>;

    /// What this provider is called, for a message somebody reads.
    fn name(&self) -> &'static str;
}
