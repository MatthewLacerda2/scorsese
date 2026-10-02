//! What an image provider is, from scorsese's side of the line.
//!
//! One call, for [`SpeechProvider`](crate::speech::SpeechProvider)'s reason:
//! the picture comes back on the connection that asked for it, so a ticket
//! would name work that was never in flight. A trait, so the lifecycle in
//! [`super`] is driven by the tests without a network or a cent.

use super::Brief;

pub use crate::video::ProviderError;

/// Somewhere a brief can be turned into a picture.
pub trait ImageProvider {
    /// Draws the brief and hands back the picture's bytes.
    ///
    /// **This is the call that spends the money**, and there is no other.
    fn draw(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError>;

    /// What this provider is called, for a message somebody reads.
    fn name(&self) -> &'static str;
}
