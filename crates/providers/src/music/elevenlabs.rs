//! ElevenLabs Music behind the trait: the translation between scorsese's
//! brief and the vendor's request, and nothing else. The wire shape lives in
//! [`api::elevenlabs::music`](crate::api::elevenlabs::music).

use crate::api::elevenlabs::music::{Compose, MODEL_ID, Music};
use crate::api::elevenlabs::refusal::Refusal;
use crate::api::http::HttpError;
use crate::credentials::Secret;

use super::{Brief, MusicProvider, ProviderError};

/// What this provider is called where a message names it.
const NAME: &str = "ElevenLabs";

/// ElevenLabs, as a [`MusicProvider`].
#[derive(Debug, Clone)]
pub struct ElevenLabsComposer {
    api: Music,
}

impl ElevenLabsComposer {
    /// A provider that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            api: Music::new(key),
        }
    }
}

impl MusicProvider for ElevenLabsComposer {
    fn compose(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.api.compose(&body_of(brief)).map_err(explain)
    }

    fn name(&self) -> &'static str {
        NAME
    }
}

/// The vendor's request, out of scorsese's brief.
fn body_of(brief: &Brief) -> Compose {
    Compose {
        prompt: brief.prompt.clone(),
        music_length_ms: brief.seconds.saturating_mul(1000),
        model_id: MODEL_ID.to_owned(),
        force_instrumental: brief.instrumental,
    }
}

/// A transport failure, turned into the most useful sentence available —
/// sorted by [`Refusal`] like every ElevenLabs refusal, except that a `404`
/// here names no voice: there is none in a music request.
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

    use super::body_of;
    use crate::music::Brief;

    #[test]
    fn a_brief_becomes_the_vendors_request() {
        let body = body_of(&Brief {
            id: AssetId::new("bed"),
            prompt: "calm lo-fi".to_owned(),
            seconds: 40,
            instrumental: false,
        });
        assert_eq!(body.music_length_ms, 40_000);
        assert_eq!(body.model_id, "music_v2_5");
        assert!(!body.force_instrumental);
    }
}
