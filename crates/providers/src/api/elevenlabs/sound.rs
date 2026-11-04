//! The sound-effects endpoint: what it takes, and what comes back (#1265).
//!
//! `POST /v1/sound-generation`, declared field for field as the vendor's API
//! reference names it (read 2026-11-04), so the two can be read side by side.
//!
//! **What is not sent, and why.** `prompt_influence` is left at the vendor's
//! default (0.3): how literally a prompt is followed is a professional's dial,
//! and the default is what the vendor tunes for. `enable_logging` stays on,
//! since turning it off is an enterprise feature. Each is absent rather than
//! defaulted, so the request is the vendor's defaults but for what is written
//! here.
//!
//! The reply is the audio itself, as the music endpoint answers, and a
//! refusal is JSON, which is why this reads bytes and leaves the refusal to
//! [`refusal`](super::refusal).

use std::time::Duration;

use serde::Serialize;

use super::{BASE, KEY_HEADER, OUTPUT_FORMAT};
use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// The one sound model scorsese makes effects with: the vendor's newest. Its
/// sound-effects page calls `eleven_text_to_sound_v2` "deprecated, and will be
/// removed on March 1, 2027", and says v3 follows multi-part prompts more
/// closely (read 2026-11-04); both cost the same, so there is no choice worth
/// offering.
///
/// A line of every sound's brief fingerprint, read from here: moving to a
/// newer model makes every sound again the next time it is generated, rather
/// than serving the old model's files as the new one's.
pub const MODEL_ID: &str = "eleven_text_to_sound_v3";

/// How long a sound may take to come back, connection open.
///
/// The vendor documents no bound, and makes at most thirty seconds of audio
/// in one request; three minutes is a ceiling on a request that hangs rather
/// than an expectation.
pub const WAIT: Duration = Duration::from_secs(3 * 60);

/// The most a sound's reply may be, in bytes: thirty seconds at 128 kbps is
/// under half a megabyte, so this bounds a redirect somewhere unexpected
/// rather than any real sound.
const MAX_REPLY_BYTES: u64 = 16 * 1024 * 1024;

/// The whole POST body of a sound-effects request.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Effect {
    /// The description of the sound.
    pub text: String,
    /// How long, in seconds — 0.5 to 30.
    pub duration_seconds: f64,
    /// Whether it loops seamlessly.
    #[serde(rename = "loop")]
    pub looping: bool,
    /// Which model makes it, as the vendor's own id.
    pub model_id: String,
}

/// The sound-effects endpoint, reachable.
#[derive(Debug, Clone)]
pub struct Sounds {
    caller: Caller,
}

impl Sounds {
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

    /// Makes a sound, and hands back the MP3 — in [`OUTPUT_FORMAT`], which
    /// every plan may ask for.
    pub fn make(&self, body: &Effect) -> Result<Vec<u8>, HttpError> {
        let url = format!("{BASE}/sound-generation?output_format={OUTPUT_FORMAT}");
        self.caller
            .post_bytes_within(&url, body, WAIT, MAX_REPLY_BYTES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The body is the vendor's names, and nothing else.
    #[test]
    fn a_request_is_written_as_the_vendor_names_it() {
        let body = Effect {
            text: "a creaking door".to_owned(),
            duration_seconds: 2.5,
            looping: true,
            model_id: MODEL_ID.to_owned(),
        };
        assert_eq!(
            serde_json::to_value(&body).expect("written"),
            serde_json::json!({
                "text": "a creaking door",
                "duration_seconds": 2.5,
                "loop": true,
                "model_id": "eleven_text_to_sound_v3"
            })
        );
    }
}
