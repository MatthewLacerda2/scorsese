//! Gemini's image model, as one call: a prompt and pictures in, a picture out.
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
pub mod request;
pub mod response;

use std::time::Duration;

use crate::api::google::{self, BASE, ModelInfo};
use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

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

/// The model every picture is drawn by, Nano Banana 2.1, as the API names it
/// — the only one scorsese uses (#1308).
pub const MODEL: &str = "gemini-nano-banana-2.1";

/// Gemini's image model, reachable.
#[derive(Debug, Clone)]
pub struct Gemini {
    caller: Caller,
}

impl Gemini {
    /// A client that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: google::caller(key),
        }
    }

    /// The same client, copying every reply into `tap` — for the live
    /// provider check ([`crate::live`]); see [`crate::api::tap`].
    pub fn tapped(mut self, tap: &crate::api::tap::Tap) -> Self {
        self.caller = self.caller.tapped(tap);
        self
    }

    /// What the API says about [`MODEL`]: its full name, and the methods it
    /// can be called with. Free — nothing is drawn.
    ///
    /// The same lookup as [`Veo::model`](crate::api::veo::Veo::model), and
    /// for the same reader: the live provider check ([`crate::live`]), which
    /// confirms without paying for a picture that the id is still served.
    pub fn model(&self) -> Result<ModelInfo, HttpError> {
        google::model(&self.caller, MODEL)
    }

    /// Draws one picture. **This is the call that spends the money.**
    pub fn create(&self, body: &request::Create) -> Result<response::Interaction, HttpError> {
        self.caller
            .post_large(&format!("{BASE}/interactions"), body, WAIT, MAX_REPLY_BYTES)
    }
}
