//! The text-to-speech endpoint: what it takes, and what comes back.
//!
//! The sibling of [`voices`](super::voices), and the only call here that costs
//! anything. Declared field for field as the vendor names it, so this file and
//! the vendor's REST page can be read side by side and checked off against each
//! other — the only practical way to keep a hand-written client honest about
//! somebody else's API.
//!
//! **The `with-timestamps` variant, always** (#811). The plain endpoint answers
//! with an MP3; this one with JSON holding the same MP3 in base64 and, beside
//! it, when each character is said — at the same price, verified on all three
//! models on 2026-10-07. So every line scorsese pays for comes back with its
//! word timings. A refusal is JSON either way, which is why this still goes
//! through `post_bytes` and reads the body itself: what is made of a refusal is
//! [`refusal`](super::refusal)'s business.

use base64::Engine;
use serde::{Deserialize, Serialize};

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

/// The most a spoken line's reply is allowed to be, in bytes.
///
/// Forty thousand characters at this bitrate is well under a hundred megabytes
/// of MP3, a third more as base64, plus the timings; this is above that, so it
/// bounds a redirect somewhere unexpected rather than any real narration.
const MAX_REPLY_BYTES: u64 = 256 * 1024 * 1024;

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

/// What a spoken line comes back as: the audio, and when each character of it
/// is said.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Timed {
    /// The MP3, base64.
    pub audio_base64: String,
    /// When each character of the text as sent is said. Optional because the
    /// vendor documents it as such; a line without it is a line without word
    /// timings, not a failure.
    #[serde(default)]
    pub alignment: Option<Alignment>,
}

/// When each character is said, as three lists of one length.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Alignment {
    /// The characters, one string each.
    pub characters: Vec<String>,
    /// When each begins, in seconds of the audio.
    pub character_start_times_seconds: Vec<f64>,
    /// When each ends.
    pub character_end_times_seconds: Vec<f64>,
}

impl Timed {
    /// The MP3, decoded — `None` if what arrived is not base64.
    pub fn audio(&self) -> Option<Vec<u8>> {
        base64::engine::general_purpose::STANDARD
            .decode(&self.audio_base64)
            .ok()
    }
}

impl Alignment {
    /// Each character with when it starts and ends — `None` if the three lists
    /// disagree about how many characters there are.
    pub fn characters(&self) -> Option<Vec<(char, f64, f64)>> {
        let n = self.characters.len();
        if self.character_start_times_seconds.len() != n
            || self.character_end_times_seconds.len() != n
        {
            return None;
        }
        let timed = self
            .characters
            .iter()
            .zip(&self.character_start_times_seconds)
            .zip(&self.character_end_times_seconds)
            .flat_map(|((text, start), end)| text.chars().map(move |c| (c, *start, *end)))
            .collect();
        Some(timed)
    }
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

    /// The same client, copying every reply into `tap` — for the live
    /// provider check ([`crate::live`]); see [`crate::api::tap`].
    pub fn tapped(mut self, tap: &crate::api::tap::Tap) -> Self {
        self.caller = self.caller.tapped(tap);
        self
    }

    /// Speaks a line, and hands back the MP3 with its timings.
    ///
    /// The voice is a path segment rather than a field, which is the vendor's
    /// shape and worth noticing for one reason: a voice that has been withdrawn
    /// answers `404` on the URL rather than a validation error about a field.
    /// See [`Refusal`](super::refusal::Refusal) for what is made of that.
    pub fn speak(&self, voice_id: &str, body: &Speak) -> Result<Timed, HttpError> {
        let url = format!(
            "{BASE}/text-to-speech/{voice_id}/with-timestamps?output_format={OUTPUT_FORMAT}"
        );
        let bytes = self.caller.post_bytes(&url, body, MAX_REPLY_BYTES)?;
        serde_json::from_slice(&bytes).map_err(|error| HttpError::Unreadable {
            url,
            message: error.to_string(),
        })
    }
}
