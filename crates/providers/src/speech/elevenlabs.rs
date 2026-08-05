//! ElevenLabs behind the trait.
//!
//! The whole of the join between scorsese's idea of a brief and the vendor's
//! idea of a request. It is deliberately thin: the wire shapes live in
//! [`api::elevenlabs`](crate::api::elevenlabs) and the lifecycle lives in
//! [`super::run`], so what is left here is the translation between them and
//! nothing else.

use crate::api::elevenlabs::refusal::Refusal;
use crate::api::elevenlabs::speech::{Speak, Speech};
use crate::api::http::HttpError;
use crate::credentials::Secret;

use super::{Brief, ProviderError, SpeechProvider};

/// What this provider is called where a message names it.
const NAME: &str = "ElevenLabs";

/// ElevenLabs, as a [`SpeechProvider`].
#[derive(Debug, Clone)]
pub struct ElevenLabsProvider {
    api: Speech,
}

impl ElevenLabsProvider {
    /// A provider that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            api: Speech::new(key),
        }
    }
}

impl SpeechProvider for ElevenLabsProvider {
    fn speak(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError> {
        let body = body_of(brief);
        self.api
            .speak(&brief.voice_id, &body)
            .map_err(|error| explain(&brief.voice_id, error))
    }

    fn name(&self) -> &'static str {
        NAME
    }
}

/// The vendor's request, out of scorsese's brief.
///
/// A function of its own rather than a few lines inside `speak`, so the
/// translation is somewhere a person can read it beside the vendor's page —
/// which is the rule the whole `api` directory is built on.
///
/// `language_code` is sent only where the model honours it. The document
/// already refuses the other combination, so this is belt and braces; what it
/// buys is that the request built here is never one the vendor would silently
/// ignore a field of, whatever a future caller does.
fn body_of(brief: &Brief) -> Speak {
    let request = &brief.request;
    Speak {
        text: brief.text.clone(),
        model_id: request.model.model_id().to_owned(),
        language_code: request
            .language
            .clone()
            .filter(|_| request.model.takes_language()),
        seed: request.seed,
    }
}

/// A transport failure, turned into the most useful sentence available.
///
/// A refusal carries the vendor's own body, and [`Refusal`] is what sorts it
/// into advice — *the key is wrong*, *the key is right and lacks a scope*,
/// *that voice needs a paid plan*, *that voice is gone*. Discarding that and
/// reporting the status code would throw away the only part worth reading, and
/// on a permissions failure it is the only part that names the fix.
fn explain(voice_id: &str, error: HttpError) -> ProviderError {
    match error {
        HttpError::Refused { status, body, .. } => {
            ProviderError::new(NAME, Refusal::of(status, voice_id, &body))
        }
        other => ProviderError::new(NAME, other),
    }
}
