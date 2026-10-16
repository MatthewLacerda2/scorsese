//! What a music provider is, from scorsese's side of the line: one call that
//! spends, as [`SpeechProvider`](crate::speech::SpeechProvider) is, and a
//! trait so the lifecycle is driven in tests without a network or a cent.

use super::Brief;

pub use crate::video::ProviderError;

/// Somewhere a description can be turned into music.
pub trait MusicProvider {
    /// Composes a track and hands back the MP3.
    ///
    /// **This is the call that spends the money**, and there is no other.
    fn compose(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError>;

    /// What this provider is called, for a message somebody reads.
    fn name(&self) -> &'static str;
}
