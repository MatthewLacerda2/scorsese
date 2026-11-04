//! ElevenLabs Sound Effects behind the trait: the translation between
//! scorsese's brief and the vendor's request, and nothing else. The wire shape
//! lives in [`api::elevenlabs::sound`](crate::api::elevenlabs::sound).

use crate::api::elevenlabs::refusal::Refusal;
use crate::api::elevenlabs::sound::{Effect, MODEL_ID, Sounds};
use crate::api::http::HttpError;
use crate::credentials::Secret;

use super::{Brief, ProviderError, SoundProvider};

/// What this provider is called where a message names it.
const NAME: &str = "ElevenLabs";

/// ElevenLabs, as a [`SoundProvider`].
#[derive(Debug, Clone)]
pub struct ElevenLabsSounds {
    api: Sounds,
}

impl ElevenLabsSounds {
    /// A provider that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            api: Sounds::new(key),
        }
    }
}

impl SoundProvider for ElevenLabsSounds {
    fn make(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError> {
        let bytes = self.api.make(&body_of(brief)).map_err(explain)?;
        answered(bytes)
    }

    fn name(&self) -> &'static str {
        NAME
    }
}

/// The vendor's request, out of scorsese's brief.
fn body_of(brief: &Brief) -> Effect {
    Effect {
        text: brief.prompt.clone(),
        duration_seconds: f64::from(brief.millis) / 1000.0,
        looping: brief.looping,
        model_id: MODEL_ID.to_owned(),
    }
}

/// A reply the vendor answered with: the sound, or — when it came back
/// empty — a call that was completed and billed with nothing to keep (#1695).
fn answered(bytes: Vec<u8>) -> Result<Vec<u8>, ProviderError> {
    if bytes.is_empty() {
        return Err(ProviderError::billed(
            NAME,
            "the vendor answered with no audio",
        ));
    }
    Ok(bytes)
}

/// A transport failure, turned into the most useful sentence available —
/// sorted by [`Refusal`] like every ElevenLabs refusal, except that a `404`
/// here names no voice: there is none in a sound request.
fn explain(error: HttpError) -> ProviderError {
    match error {
        HttpError::Refused { status, body, .. } => match Refusal::of(status, "", &body) {
            Refusal::VoiceGone { .. } => ProviderError::new(
                NAME,
                Refusal::Other {
                    status,
                    message: body,
                },
            ),
            refusal => ProviderError::new(NAME, refusal),
        },
        other => ProviderError::new(NAME, other),
    }
}

#[cfg(test)]
mod tests {
    use scorsese_core::AssetId;

    use super::{answered, body_of};
    use crate::sound::Brief;

    #[test]
    fn a_brief_becomes_the_vendors_request() {
        let body = body_of(&Brief {
            id: AssetId::new("door"),
            prompt: "a creaking door".to_owned(),
            millis: 2_500,
            looping: true,
        });
        assert_eq!(body.text, "a creaking door");
        assert!((body.duration_seconds - 2.5).abs() < f64::EPSILON);
        assert!(body.looping);
        assert_eq!(body.model_id, "eleven_text_to_sound_v3");
    }

    #[test]
    fn only_an_empty_answer_is_a_billed_refusal() {
        let refused = answered(Vec::new()).expect_err("nothing came back");
        assert!(refused.billed, "{refused:?}");
        assert_eq!(answered(b"MP3".to_vec()).expect("a sound"), b"MP3");
    }
}
