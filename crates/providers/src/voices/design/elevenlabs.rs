//! ElevenLabs behind the trait: a description in, three candidates out, one of
//! them kept.
//!
//! The whole of this file is translation, in both directions, exactly as its
//! sibling next door is. What a call looks like on the wire is
//! [`api::elevenlabs`](crate::api::elevenlabs)'s business and what to do with
//! an answer is [`super`]'s; this is the one place that turns the vendor's
//! shapes into scorsese's, so that it can be checked against the vendor's
//! reference page line by line.
//!
//! Two things here are not in the listing client.
//!
//! **The samples arrive as text.** A design reply carries three MP3s base64'd
//! inside the JSON, so decoding is part of reading the reply — and a sample
//! that will not decode is refused rather than written, because half an MP3 is
//! a file that exists, plays as noise, and looks like the vendor's fault.
//!
//! **A refusal here has one meaning the listings do not.** Voice Design is not
//! sold to a free account through the API, and that reads as the account's plan
//! rather than as anything wrong with the request — the same shape as the Voice
//! Library's refusal, and it points somewhere different because the remedy is
//! different: the built-in voices are still there.

use crate::api::base64;
use crate::api::elevenlabs::design::{CreateRequest, Design, PreviewSample};
use crate::api::http::HttpError;
use crate::credentials::Secret;
use crate::video::ProviderError;

use super::super::refusal::{NO_VOICE, failure, plan_limited, refused};
use super::super::{Voice, VoiceError};
use super::brief::Brief;
use super::error::DesignError;
use super::studio::{Candidate, Studio};

/// What this studio is called wherever a message names it.
const NAME: &str = "ElevenLabs";

/// Voice Design, as a studio.
#[derive(Debug, Clone)]
pub struct ElevenLabsStudio {
    design: Design,
}

impl ElevenLabsStudio {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            design: Design::new(key),
        }
    }
}

impl Studio for ElevenLabsStudio {
    fn name(&self) -> &'static str {
        NAME
    }

    fn candidates(&self, brief: &Brief) -> Result<Vec<Candidate>, DesignError> {
        let reply = self.design.preview(&brief.request()).map_err(refusal)?;
        reply.previews.iter().map(candidate).collect()
    }

    fn keep(
        &self,
        brief: &Brief,
        chosen: &str,
        name: &str,
        passed_over: &[String],
    ) -> Result<Voice, DesignError> {
        let created = self
            .design
            .create(&CreateRequest {
                voice_name: name.to_owned(),
                voice_description: brief.prompt.clone(),
                generated_voice_id: chosen.to_owned(),
                played_not_selected_voice_ids: passed_over.to_vec(),
            })
            .map_err(refusal)?;
        Ok(Voice {
            id: created.voice_id,
            // The vendor echoes the name back, and that echo is what to trust:
            // it has its own rules about what a voice may be called, so a
            // surface printing what was *asked for* would be printing a name
            // the account may not actually hold.
            name: if created.name.trim().is_empty() {
                name.to_owned()
            } else {
                created.name
            },
            // Designed rather than published, so nothing filled in a language
            // or an age for it. Said outright rather than left blank, because a
            // voice with no traits at all beside a listing's is easy to read as
            // a voice that failed to load.
            traits: vec![String::from("designed")],
            description: created.description.or_else(|| Some(brief.prompt.clone())),
            // No preview URL: the samples are in the project, which is a better
            // answer than a link — they are the exact three this voice was
            // chosen from.
            preview: None,
        })
    }
}

/// One candidate, decoded.
fn candidate(sample: &PreviewSample) -> Result<Candidate, DesignError> {
    let bytes = base64::decode(&sample.audio_base_64).ok_or_else(|| {
        DesignError::Voice(VoiceError::Provider(ProviderError::new(
            NAME,
            "a preview sample was not readable audio",
        )))
    })?;
    Ok(Candidate {
        generated_voice_id: sample.generated_voice_id.clone(),
        sample: bytes,
        seconds: sample.duration_secs,
    })
}

/// A failed design call as the error it actually is.
///
/// Three outcomes, and the split is the advice each one calls for. The
/// account's plan points at the built-in voices, which still work. A key
/// missing a scope points at the vendor's dashboard. Everything else — a
/// timeout, a `500`, a key that is simply wrong — is a failure to *ask*, and is
/// carried through as one rather than guessed at.
fn refusal(error: HttpError) -> DesignError {
    match refused(NO_VOICE, &error).as_ref().and_then(plan_limited) {
        Some(said) => DesignError::NotOnThisPlan {
            said: said.to_owned(),
        },
        None => DesignError::Voice(failure(NAME, error)),
    }
}
