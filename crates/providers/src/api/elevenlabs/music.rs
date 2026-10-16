//! The music endpoint: what it takes, and what comes back (#1264).
//!
//! `POST /v1/music`, declared field for field as the vendor's API reference
//! names it (read 2026-10-16), so the two can be read side by side. Only the
//! prompt form is sent: the vendor also takes a `composition_plan` instead of
//! a prompt — sections, styles, lyrics line by line — and that is a
//! professional's tool this one deliberately leaves out.
//!
//! **What is not sent, and why.** `seed` is refused by the vendor beside a
//! prompt; `finetune_id`, `store_for_inpainting` and `sign_with_c2pa` are
//! features scorsese has no use for. Each is absent rather than defaulted, so
//! the request is the vendor's defaults but for what is written here.
//!
//! The reply is the audio itself, the way the plain text-to-speech endpoint
//! answers — no JSON around it — and a refusal is JSON, which is why this
//! reads bytes and leaves the refusal to [`refusal`](super::refusal).

use std::time::Duration;

use serde::Serialize;

use super::{BASE, KEY_HEADER, OUTPUT_FORMAT};
use crate::api::http::{Caller, HttpError};
use crate::credentials::Secret;

/// The one music model scorsese composes with: the vendor's newest, which its
/// models page calls its most advanced (read 2026-10-16). `music_v1` is
/// "outclassed" in the vendor's own word, and every model costs the same
/// (`docs/prices.md`, *Music*), so there is no choice worth offering.
///
/// A line of every track's brief fingerprint, read from here: moving to a
/// newer model composes every track again the next time it is generated,
/// rather than serving the old model's files as the new one's.
pub const MODEL_ID: &str = "music_v2_5";

/// How long a track may take to come back, connection open.
///
/// The vendor documents no bound, and composes up to ten minutes of music in
/// one request; ten minutes of waiting is room for that, and a ceiling on a
/// request that hangs rather than an expectation.
pub const WAIT: Duration = Duration::from_secs(10 * 60);

/// The most a track's reply may be, in bytes: ten minutes at 128 kbps is
/// under ten megabytes, so this bounds a redirect somewhere unexpected rather
/// than any real track.
const MAX_REPLY_BYTES: u64 = 64 * 1024 * 1024;

/// The whole POST body of a music request, in the prompt form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Compose {
    /// The description of the music.
    pub prompt: String,
    /// How long, in milliseconds — 3,000 to 600,000.
    pub music_length_ms: u32,
    /// Which model composes it, as the vendor's own id.
    pub model_id: String,
    /// Whether it is guaranteed to have no singing.
    pub force_instrumental: bool,
}

/// The music endpoint, reachable.
#[derive(Debug, Clone)]
pub struct Music {
    caller: Caller,
}

impl Music {
    /// A client that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key),
        }
    }

    /// Composes a track, and hands back the MP3.
    ///
    /// The format is asked for rather than left at the vendor's `auto`, which
    /// on the v2 models is 192 kbps — a format the vendor gates behind its
    /// Creator plan. [`OUTPUT_FORMAT`] is open to every plan.
    pub fn compose(&self, body: &Compose) -> Result<Vec<u8>, HttpError> {
        let url = format!("{BASE}/music?output_format={OUTPUT_FORMAT}");
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
        let body = Compose {
            prompt: "calm lo-fi".to_owned(),
            music_length_ms: 40_000,
            model_id: MODEL_ID.to_owned(),
            force_instrumental: true,
        };
        assert_eq!(
            serde_json::to_value(&body).expect("written"),
            serde_json::json!({
                "prompt": "calm lo-fi",
                "music_length_ms": 40_000,
                "model_id": "music_v2_5",
                "force_instrumental": true
            })
        );
    }
}
