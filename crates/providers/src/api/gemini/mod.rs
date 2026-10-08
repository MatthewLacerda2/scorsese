//! Gemini's image models, as one call: a prompt and pictures in, a picture out —
//! and, in [`chat`], Gemini as the assistant's chat model (#705).
//!
//! The whole of scorsese's dealings with Google's image API, through the
//! **Interactions** endpoint, which is the one Google's image-generation page
//! documents (read 2026-10-02:
//! <https://ai.google.dev/gemini-api/docs/image-generation>). Nothing here
//! knows about the sketch lifecycle, the cache or a price.
//!
//! One call where Veo has three, because the work is seconds rather than
//! minutes: the picture comes back on the connection that asked for it, inside
//! the JSON, base64. There is no ticket to keep, so none is invented — except
//! for a still somebody chose to wait a day for at half price, which is a
//! [`batch`] job with a name, polled like a shot.

pub mod batch;
pub mod chat;
pub mod request;
pub mod response;

use std::time::Duration;

use crate::api::http::{Caller, HttpError};
use crate::api::veo::response::ModelInfo;
use crate::credentials::Secret;

/// The API's root, which every endpoint here hangs off.
const BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// The endpoint every interaction is created at.
const INTERACTIONS: &str = "https://generativelanguage.googleapis.com/v1beta/interactions";

/// The header Google reads the key from — the same key Veo takes.
const KEY_HEADER: &str = "x-goog-api-key";

/// How long one drawing may take before it is given up on.
///
/// Five minutes. A 1K still is seconds; a 4K one with several references is
/// longer, and the bound is there for a request that has genuinely hung, not
/// for a slow one.
const WAIT: Duration = Duration::from_secs(300);

/// The most a reply carrying one picture is read to, in bytes.
///
/// A 4K picture is tens of megabytes and base64 adds a third; this is well above
/// that and bounds a reply that is something else entirely.
const MAX_REPLY_BYTES: u64 = 256 * 1024 * 1024;

/// Which model draws a picture: the ids the API answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// Gemini Nano Banana 2.1.
    NanoBanana21,
    /// Gemini 3.1 Flash Image.
    Flash,
    /// Gemini 3.1 Flash Lite Image.
    Lite,
    /// Gemini 3 Pro Image.
    Pro,
}

impl Model {
    /// Every model scorsese offers.
    pub const ALL: [Self; 4] = [Self::NanoBanana21, Self::Flash, Self::Lite, Self::Pro];

    /// The id the request names.
    pub const fn id(self) -> &'static str {
        match self {
            Self::NanoBanana21 => "gemini-nano-banana-2.1",
            Self::Flash => "gemini-3.1-flash-image",
            Self::Lite => "gemini-3.1-flash-lite-image",
            Self::Pro => "gemini-3-pro-image",
        }
    }
}

/// Gemini's image models, reachable.
#[derive(Debug, Clone)]
pub struct Gemini {
    caller: Caller,
}

impl Gemini {
    /// A client that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key),
        }
    }

    /// The same client, copying every reply into `tap` — for the live
    /// provider check ([`crate::live`]); see [`crate::api::tap`].
    pub fn tapped(mut self, tap: &crate::api::tap::Tap) -> Self {
        self.caller = self.caller.tapped(tap);
        self
    }

    /// What the API says about a model: its full name, and the methods it
    /// can be called with. Free — nothing is drawn.
    ///
    /// The same lookup as [`Veo::model`](crate::api::veo::Veo::model), and
    /// for the same reader: the live provider check ([`crate::live`]), which
    /// confirms without paying for a picture that each id is still served.
    pub fn model(&self, model: Model) -> Result<ModelInfo, HttpError> {
        self.caller.get(&format!("{BASE}/models/{}", model.id()))
    }

    /// Draws one picture. **This is the call that spends the money.**
    pub fn create(&self, body: &request::Create) -> Result<response::Interaction, HttpError> {
        self.caller
            .post_large(INTERACTIONS, body, WAIT, MAX_REPLY_BYTES)
    }
}
