//! Gemini as a chat model: one streamed `generateContent` call.
//!
//! The whole of scorsese's dealings with Google's chat wire (#705): what a
//! request looks like ([`request`]) and the chunks a streamed reply arrives
//! as ([`response`]). Nothing here knows about a turn, a tool, a user or a
//! price — those are [`crate::chat::gemini`]'s and the server's.
//!
//! **`streamGenerateContent`, not Interactions.** The image client beside this
//! one ([`super`]) uses the newer Interactions endpoint, which Google's chat
//! guides now lead with too. Read on 2026-10-03, its reference does not yet
//! spell out a stateless multi-turn history, function results or the stream's
//! delta payloads; `generateContent` documents all three, is stable, carries no
//! deprecation notice, and reports usage as `usageMetadata` with the cached
//! count (`cachedContentTokenCount`) and thinking (`thoughtsTokenCount`) apart
//! — every figure the charge needs. Sources: <https://ai.google.dev/api/generate-content>,
//! <https://ai.google.dev/gemini-api/docs/function-calling> and
//! <https://ai.google.dev/gemini-api/docs/thinking>.
//!
//! The shapes were written from that reference, not captured; the replies
//! the assembler is tested against are fixtures written the same way.

pub mod request;
pub mod response;

use std::io::BufRead;

use crate::api::http::{Caller, HttpError};
use crate::api::sse::{self, StreamError};
use crate::credentials::Secret;

/// Where a model's streamed `generateContent` lives; `{model}` is its id.
const STREAM: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent?alt=sse";

/// The header Google reads the key from — the same key Veo takes.
const KEY_HEADER: &str = "x-goog-api-key";

/// A caller that can reach `generateContent` with one key.
#[derive(Debug, Clone)]
pub struct Generate {
    caller: Caller,
}

impl Generate {
    /// A client sending `key` on every call.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key),
        }
    }

    /// The same client, copying every reply into `tap` — see
    /// [`crate::api::tap`].
    pub fn tapped(mut self, tap: &crate::api::tap::Tap) -> Self {
        self.caller = self.caller.tapped(tap);
        self
    }

    /// Send `body` to `model` and hand back its reply as it streams: the
    /// chunks, in order.
    pub fn stream(
        &self,
        model: &str,
        body: &request::Generate,
    ) -> Result<impl Iterator<Item = Result<response::Chunk, StreamError>>, HttpError> {
        let reader = self
            .caller
            .post_stream(&STREAM.replace("{model}", model), body)?;
        Ok(chunks(reader))
    }
}

/// The chunks `reader` carries, in order, until it ends.
pub fn chunks<R: BufRead>(reader: R) -> impl Iterator<Item = Result<response::Chunk, StreamError>> {
    sse::events(reader)
}
