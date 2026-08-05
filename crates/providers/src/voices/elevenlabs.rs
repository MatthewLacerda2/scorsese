//! ElevenLabs behind the trait: two listings in, voices out, refusals read.
//!
//! The whole of this file is translation, in both directions. What a call looks
//! like on the wire is [`api::elevenlabs`](crate::api::elevenlabs)'s business
//! and what to do with an answer is [`super`]'s; this is the one place that
//! turns the vendor's shapes into scorsese's, and it is a separate file for
//! exactly that reason — a translation spread through the code that makes the
//! call is a translation nobody can check against the vendor's documentation.
//!
//! **Reading a refusal is the interesting half.** A status code alone cannot
//! tell a key that is wrong from a key that is right but unscoped, nor a
//! withdrawn voice from one this account may not use, and those four call for
//! four different sentences. So each call maps the status *and* the vendor's
//! own `detail` onto the state it actually means, and everything unrecognised
//! stays a plain provider failure rather than being guessed at.

use crate::api::elevenlabs::refusal::{self, Detail, MISSING_PERMISSIONS, PAID_PLAN_REQUIRED};
use crate::api::elevenlabs::voices::{self, Filters, Voices};
use crate::api::http::HttpError;
use crate::credentials::Secret;
use crate::video::ProviderError;

use super::catalogue::{Availability, Catalogue, Unusable, Voice};
use super::error::VoiceError;

/// What this catalogue is called wherever a message names it.
const NAME: &str = "ElevenLabs";

/// The voice listings, as a catalogue.
#[derive(Debug, Clone)]
pub struct ElevenLabsVoices {
    voices: Voices,
}

impl ElevenLabsVoices {
    /// One that authenticates with this key.
    pub fn new(key: &Secret) -> Self {
        Self {
            voices: Voices::new(key),
        }
    }
}

impl Catalogue for ElevenLabsVoices {
    fn name(&self) -> &'static str {
        NAME
    }

    fn builtin(&self) -> Result<Vec<Voice>, VoiceError> {
        let listing = self.voices.premade().map_err(listing_failure)?;
        Ok(listing.voices.iter().map(voice).collect())
    }

    fn library(&self, filters: &Filters) -> Result<Vec<Voice>, VoiceError> {
        let listing = self.voices.shared(filters).map_err(|error| {
            // The plan refusal only has a meaning on this endpoint: the Voice
            // Library is the part that is not sold to a free account, and the
            // built-in listing above is never refused for that reason.
            match refused(&error) {
                Some((status, detail)) if plan_limited(status, &detail) => {
                    VoiceError::NoLibraryOnThisPlan {
                        said: said(&error, &detail).to_owned(),
                    }
                }
                _ => listing_failure(error),
            }
        })?;
        Ok(listing.voices.iter().map(voice).collect())
    }

    fn one(&self, voice_id: &str) -> Result<Availability, VoiceError> {
        match self.voices.one(voice_id) {
            Ok(found) => Ok(Availability::Available(voice(&found))),
            Err(error) => match refused(&error) {
                // The case with a date on it. Nothing answers to the id, which
                // is what an expired default voice looks like from here.
                Some((404, _)) => Ok(Availability::Unusable(Unusable::Gone)),
                Some((status, detail)) if plan_limited(status, &detail) => {
                    Ok(Availability::Unusable(Unusable::NotOnThisPlan {
                        said: said(&error, &detail).to_owned(),
                    }))
                }
                // Everything else — a timeout, a 500, a key with no scope — is
                // a failure to *ask*, and says nothing about the voice. Turning
                // one of those into "unusable" is how a bad afternoon on the
                // network would look like a voice being withdrawn.
                _ => Err(listing_failure(error)),
            },
        }
    }
}

/// A refusal's status and whatever the vendor said inside it.
///
/// `None` for anything that never got an answer at all — no network, DNS, TLS,
/// a timeout — because those have no status to interpret and must not be
/// mistaken for one.
fn refused(error: &HttpError) -> Option<(u16, Detail)> {
    match error {
        HttpError::Refused { status, body, .. } => Some((*status, refusal::read(body))),
        _ => None,
    }
}

/// Whether this refusal is the account's plan rather than its request.
///
/// Both spellings the vendor uses: `402 paid_plan_required` on a voice a free
/// account may not generate with, and a bare `403` on the Voice Library. The
/// status word is checked first where there is one, because it is the vendor
/// being explicit and the number is only ever an inference.
fn plan_limited(status: u16, detail: &Detail) -> bool {
    detail.is(PAID_PLAN_REQUIRED) || matches!(status, 402 | 403)
}

/// The best sentence available about a refusal.
fn said<'a>(error: &'a HttpError, detail: &'a Detail) -> &'a str {
    match error {
        HttpError::Refused { body, .. } => detail.says(body),
        _ => "",
    }
}

/// A failed call as the error it actually is.
///
/// The one distinction worth making here is the one a status code cannot make:
/// a `401` because the key is absent or wrong, against a `401` because the key
/// is perfectly good and was never granted `voices_read`. They are fixed in
/// different places — one in `.env`, one in the vendor's dashboard — so
/// reporting them the same way sends half of the people who hit this to look at
/// something that is not wrong.
fn listing_failure(error: HttpError) -> VoiceError {
    if let Some((_, detail)) = refused(&error)
        && detail.is(MISSING_PERMISSIONS)
    {
        return VoiceError::MissingPermission {
            said: said(&error, &detail).to_owned(),
        };
    }
    VoiceError::Provider(ProviderError::new(NAME, error))
}

/// A vendor voice as scorsese carries one.
///
/// The two listings fill in different halves of the same facts — `/voices`
/// nests an accent and an age under `labels`, `/shared-voices` spreads them
/// across top-level fields — so both are read and whatever is present survives.
/// Flattening here rather than at each surface is what lets a built-in voice
/// and a Voice Library voice be printed in one list.
fn voice(found: &voices::Voice) -> Voice {
    let mut traits: Vec<String> = Vec::new();
    let mut add = |value: &Option<String>| {
        if let Some(value) = value.as_ref().filter(|value| !value.trim().is_empty()) {
            traits.push(value.trim().to_owned());
        }
    };
    add(&found.language);
    add(&found.gender);
    add(&found.age);
    add(&found.accent);
    add(&found.use_case);
    // The labels come last and in the vendor's own order, which is alphabetical
    // by key: they are the built-in listing's way of saying the same things,
    // and a voice that carried both would otherwise say each of them twice.
    // Every label, including the one called `description`: on a built-in voice
    // that is a one-word descriptor — *casual*, *expressive* — and not the
    // sentence the top-level field of that name holds.
    for value in found.labels.values() {
        let value = value.trim();
        if !value.is_empty() && !traits.iter().any(|had| had.eq_ignore_ascii_case(value)) {
            traits.push(value.to_owned());
        }
    }
    Voice {
        id: found.voice_id.clone(),
        name: found.name.clone(),
        traits,
        description: found.description.clone(),
        preview: found.preview_url.clone(),
    }
}
