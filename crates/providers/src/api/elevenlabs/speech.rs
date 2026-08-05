//! The text-to-speech endpoint: what it takes, and what comes back.
//!
//! The sibling of [`voices`](super::voices), and the only call here that costs
//! anything. Declared field for field as the vendor names it, so this file and
//! the vendor's REST page can be read side by side and checked off against each
//! other — the only practical way to keep a hand-written client honest about
//! somebody else's API.
//!
//! **A success has no JSON shape at all.** It is an MP3, and the failure is the
//! JSON — the reverse of every other endpoint scorsese calls, which is why this
//! goes through `post_bytes` and why the reply cannot be typed in advance. What
//! is made of a refusal is [`refusal`](super::refusal)'s business.

use serde::Serialize;

use super::{BASE, KEY_HEADER};
use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// The one output format scorsese asks for.
///
/// The API's own default, and not gated behind any subscription tier — which is
/// the whole reason it is a constant rather than a choice. 192kbps MP3 needs
/// Creator, PCM and WAV at 44.1kHz need Pro, and a picker whose options depend
/// on somebody's plan is a tier-detection feature wearing a dropdown.
const OUTPUT_FORMAT: &str = "mp3_44100_128";

/// The most a spoken line is allowed to be, in bytes.
///
/// Forty thousand characters at this bitrate is well under a hundred megabytes;
/// this is orders above that, so it bounds a redirect somewhere unexpected
/// rather than any real narration.
const MAX_AUDIO_BYTES: u64 = 128 * 1024 * 1024;

/// The whole POST body of a text-to-speech request.
///
/// Flat, because the vendor's is. Every optional field is skipped when absent
/// rather than sent as `null`, which matters more here than it looks: the API
/// treats an omitted `model_id` as `eleven_multilingual_v2`, so *absent* is a
/// meaningful value and not merely a shorter way of writing the default.
///
/// The voice is deliberately not here: the vendor puts it in the URL, and this
/// file mirrors the wire rather than tidying it.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Speak {
    /// The words to say.
    pub text: String,
    /// Which model says them, as the vendor's own id.
    pub model_id: String,
    /// The language to pin the reading to, as an ISO 639-1 code.
    ///
    /// Sent only where it will be honoured. `eleven_multilingual_v2` accepts
    /// this field and silently ignores it, so scorsese refuses that combination
    /// in the document rather than sending something that does nothing — see
    /// `SpeechModel::takes_language` in the core crate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    /// A seed, for a reading that comes back the same way twice.
    ///
    /// Best-effort at the vendor and documented as such, so it is worth sending
    /// and not worth relying on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
}

/// The text-to-speech endpoint, reachable.
#[derive(Debug, Clone)]
pub struct Speech {
    caller: Caller,
}

impl Speech {
    /// A client that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key),
        }
    }

    /// Speaks a line, and hands back the MP3.
    ///
    /// The voice is a path segment rather than a field, which is the vendor's
    /// shape and worth noticing for one reason: a voice that has been withdrawn
    /// answers `404` on the URL rather than a validation error about a field.
    /// See [`Refusal`](super::refusal::Refusal) for what is made of that.
    pub fn speak(&self, voice_id: &str, body: &Speak) -> Result<Vec<u8>, HttpError> {
        let url = format!("{BASE}/text-to-speech/{voice_id}?output_format={OUTPUT_FORMAT}");
        self.caller.post_bytes(&url, body, MAX_AUDIO_BYTES)
    }
}
