//! Instant Voice Cloning: a voice from somebody's own recordings, in one call.
//!
//! **`POST /v1/voices/add`** takes a name and one or more audio files as
//! `multipart/form-data`, and answers with the `voice_id` of a voice now in
//! the account — usable exactly like one from a listing, and taking one of the
//! account's custom voice slots, as a designed voice does. The undoing of it
//! is the same **`DELETE /v1/voices/{voice_id}`** a designed voice's is
//! ([`Design::delete`](super::design::Design::delete)).
//!
//! What the vendor's pages say, read 2026-10-16:
//!
//! - **The request** (elevenlabs.io/docs/api-reference/voices/ivc/create):
//!   `name` and `files` required; `remove_background_noise`, `description`
//!   and `labels` optional. The page lists no length or size limit for any of
//!   them. It answers `voice_id` and `requires_verification`.
//! - **The recordings** (elevenlabs.io/docs/eleven-creative/voices/
//!   voice-cloning/instant-voice-cloning): "Approximately 1-2 minutes of clear
//!   audio without any reverb, artifacts, or background noise of any kind is
//!   recommended", and "Avoid recording more than 3 minutes, this will yield
//!   little improvement" — [`RECOMMENDED_SECONDS`], [`ENOUGH_SECONDS`].
//! - **Consent**: the vendor's own app asks a person to "confirm that you
//!   have the right and consent to clone the voice" before saving one; its
//!   terms are [`TERMS`] and [`USE_POLICY`]. The API takes no such field, so
//!   scorsese asks it itself (`voices::clone`).
//! - **The plan** (elevenlabs.io/pricing): Instant Voice Cloning is listed
//!   from Starter up; the free plan has none.
//! - **The price**: the pricing page's list of what credits are spent on —
//!   speech, transcription, music, sound effects, voice changer and isolator,
//!   dubbing — has no line for cloning, so creating a clone is not charged;
//!   narrating with it is charged as any line is.

use serde::Deserialize;

use super::{BASE, KEY_HEADER};
use crate::api::http::{Caller, Form, HttpError};
use crate::credentials::Secret;

/// How much recording the vendor recommends, in seconds: one to two minutes.
pub const RECOMMENDED_SECONDS: std::ops::RangeInclusive<f64> = 60.0..=120.0;

/// Past this many seconds of recording, the vendor says, more "will yield
/// little improvement".
pub const ENOUGH_SECONDS: f64 = 180.0;

/// ElevenLabs' terms of use, which cloning a voice is bound by.
pub const TERMS: &str = "https://elevenlabs.io/terms-of-use";

/// ElevenLabs' prohibited use policy: what a cloned voice may never be used
/// for.
pub const USE_POLICY: &str = "https://elevenlabs.io/use-policy";

/// One recording, as it goes up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recording {
    /// The file name sent with it — the vendor shows it as the sample's name.
    pub file_name: String,
    /// What the bytes are, e.g. `audio/mpeg`.
    pub media_type: String,
    /// The audio itself.
    pub bytes: Vec<u8>,
}

/// What a clone asks for.
#[derive(Debug, Clone, PartialEq)]
pub struct AddRequest {
    /// What the voice will be called in the account.
    pub name: String,
    /// What the voice is, kept with it at the vendor. Omitted when absent.
    pub description: Option<String>,
    /// The recordings to clone from — at least one.
    pub recordings: Vec<Recording>,
    /// Ask the vendor to clean background noise out of the recordings first.
    /// Its page warns this "can lower quality" on recordings with none, so it
    /// is only sent when asked for.
    pub remove_background_noise: bool,
}

impl AddRequest {
    /// The request, as the form the vendor takes.
    pub fn form(&self) -> Form {
        let mut form = Form::new().text("name", self.name.clone());
        if let Some(description) = &self.description {
            form = form.text("description", description.clone());
        }
        if self.remove_background_noise {
            form = form.text("remove_background_noise", "true");
        }
        for recording in &self.recordings {
            form = form.file(
                "files",
                &recording.file_name,
                &recording.media_type,
                recording.bytes.clone(),
            );
        }
        form
    }
}

/// The voice that now exists in the account.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct AddedVoice {
    /// The id a narration names.
    #[serde(default)]
    pub voice_id: String,
    /// Whether the vendor wants the voice verified before it can be used.
    #[serde(default)]
    pub requires_verification: bool,
}

/// Instant Voice Cloning, reachable.
///
/// Its own client, beside [`Design`](super::design::Design), because it is
/// its own endpoint taking its own shape of body; both make a voice in the
/// account, which is why one [`Studio`](crate::voices::design::Studio) holds
/// both.
#[derive(Debug, Clone)]
pub struct Cloning {
    caller: Caller,
}

impl Cloning {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            caller: Caller::new(KEY_HEADER, key),
        }
    }

    /// Clones a voice from `request`'s recordings into the account.
    pub fn add(&self, request: &AddRequest) -> Result<AddedVoice, HttpError> {
        self.caller
            .post_multipart(&format!("{BASE}/voices/add"), &request.form())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(remove_background_noise: bool, description: Option<&str>) -> AddRequest {
        AddRequest {
            name: String::from("Me"),
            description: description.map(ToOwned::to_owned),
            recordings: vec![Recording {
                file_name: String::from("take.mp3"),
                media_type: String::from("audio/mpeg"),
                bytes: b"ID3".to_vec(),
            }],
            remove_background_noise,
        }
    }

    fn body(request: &AddRequest) -> String {
        String::from_utf8(request.form().encode().1).unwrap()
    }

    #[test]
    fn an_optional_field_is_sent_only_when_asked_for() {
        let plain = body(&request(false, None));
        assert!(plain.contains("name=\"name\"\r\n\r\nMe\r\n"), "{plain}");
        assert!(plain.contains("filename=\"take.mp3\""), "{plain}");
        assert!(!plain.contains("remove_background_noise"), "{plain}");
        assert!(!plain.contains("description"), "{plain}");

        let asked = body(&request(true, Some("warm")));
        assert!(
            asked.contains("remove_background_noise\"\r\n\r\ntrue"),
            "{asked}"
        );
        assert!(asked.contains("description\"\r\n\r\nwarm"), "{asked}");
    }

    #[test]
    fn the_reply_reads_the_id_and_the_verification_flag() {
        let reply: AddedVoice =
            serde_json::from_str(r#"{"voice_id":"c38k","requires_verification":true}"#).unwrap();
        assert_eq!(reply.voice_id, "c38k");
        assert!(reply.requires_verification);
    }
}
