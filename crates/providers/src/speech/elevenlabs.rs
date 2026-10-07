//! ElevenLabs behind the trait.
//!
//! The whole of the join between scorsese's idea of a brief and the vendor's
//! idea of a request. It is deliberately thin: the wire shapes live in
//! [`api::elevenlabs`](crate::api::elevenlabs) and the lifecycle lives in
//! [`super::run`], so what is left here is the translation between them and
//! nothing else.

use scorsese_core::words::Words;

use crate::api::elevenlabs::refusal::Refusal;
use crate::api::elevenlabs::speech::{Speak, Speech, Timed};
use crate::api::http::HttpError;
use crate::credentials::Secret;

use super::{Brief, ProviderError, SpeechProvider, Spoken};

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
    fn speak(&self, brief: &Brief) -> Result<Spoken, ProviderError> {
        let body = body_of(brief);
        let timed = self
            .api
            .speak(&brief.voice_id, &body)
            .map_err(|error| explain(&brief.voice_id, error))?;
        spoken(&timed)
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

/// The line out of the vendor's reply: the MP3 decoded, and the characters'
/// timings folded into words. A reply whose timings do not add up still gives
/// its audio, without them — the line was paid for either way.
fn spoken(timed: &Timed) -> Result<Spoken, ProviderError> {
    let audio = timed
        .audio()
        .ok_or_else(|| ProviderError::new(NAME, "the audio came back as something not base64"))?;
    let words = timed
        .alignment
        .as_ref()
        .and_then(|alignment| alignment.characters())
        .map(Words::from_characters);
    Ok(Spoken { audio, words })
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
