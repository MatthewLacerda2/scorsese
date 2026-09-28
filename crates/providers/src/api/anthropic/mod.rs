//! Anthropic's Messages API, as one streamed call.
//!
//! The whole of scorsese's dealings with Claude's wire (#540): what a request
//! looks like ([`request`]), the blocks a conversation is made of
//! ([`content`]), and the server-sent events a streamed reply arrives as
//! ([`stream`]). Nothing here knows about a turn, a tool, a user or a price —
//! those are scorsese's business ([`crate::claude`], and the hosted server). A
//! reader checking a payload against Anthropic's reference should be able to
//! do it from this directory alone.
//!
//! There is no official Anthropic SDK for Rust, so this is plain HTTP, the way
//! Veo and ElevenLabs are. The shapes were written from Anthropic's API
//! reference as the `claude-api` skill carried it on 2026-09-28 — the raw-HTTP
//! request and streaming examples, the prompt-caching and tool-use pages, and
//! the Claude Opus 5.5 migration notes — and the replies it parses are pinned
//! by recorded fixtures under `fixtures/anthropic/`, which #567's live check
//! verifies against a real call.
//!
//! **Always streamed.** A turn of Claude Opus 5.5 thinks, writes and calls
//! tools for minutes; an unstreamed request that long meets every timeout
//! between here and Anthropic, and a streamed one is also what lets the web
//! app show the reply as it is written.

pub mod content;
pub mod request;
pub mod stream;

use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// The one endpoint.
const MESSAGES: &str = "https://api.anthropic.com/v1/messages";

/// The header Anthropic reads the key from.
const KEY_HEADER: &str = "x-api-key";

/// The API version every request names, as the reference requires.
const VERSION: &str = "2023-06-01";

/// The beta features every request opts into, comma-separated.
///
/// `thinking-display-updates-2026-08-18` is what makes `thinking.display:
/// "updates"` legal: on Claude Opus 5.5 the notes the model writes between
/// tool calls come back as `thinking` blocks, empty unless asked for this way,
/// and those notes are the assistant's progress lines.
pub const BETAS: &str = "thinking-display-updates-2026-08-18";

/// A caller that can reach the Messages API with one key.
#[derive(Debug, Clone)]
pub struct Messages {
    caller: Caller,
}

impl Messages {
    /// A client sending `key`, the API version and [`BETAS`] on every call.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key)
                .with("anthropic-version", VERSION)
                .with("anthropic-beta", BETAS),
        }
    }

    /// Send `request` and hand back its reply as it streams: the events, in
    /// order, as [`stream::events`] reads them.
    pub fn stream(
        &self,
        request: &request::Request,
    ) -> Result<impl Iterator<Item = Result<stream::Event, stream::StreamError>>, HttpError> {
        let reader = self.caller.post_stream(MESSAGES, request)?;
        Ok(stream::events(reader))
    }
}
