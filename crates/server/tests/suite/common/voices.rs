//! The voice catalogue every test server asks where a voice comes from
//! (#1655), by the id alone: `custom-…` and `voice-of-…` (what the studio
//! mock names a kept voice) are custom voices in the account, `library-…` a
//! Voice Library voice, `gone-…` nothing at all, and any other a built-in;
//! and one that cannot be reached at all ([`unreachable`]).

use std::sync::Arc;

use scorsese_providers::video::ProviderError;
use scorsese_providers::voices::{
    Availability, Catalogue, Filters, Origin, Page, Voice, VoiceError,
};
use scorsese_server::tools::Voices;

/// The catalogue, as a server takes one.
pub(crate) fn voices() -> Voices {
    Arc::new(ById)
}

/// A catalogue that cannot be reached, as ElevenLabs down or with no key.
pub(crate) fn unreachable() -> Voices {
    Arc::new(Unreachable)
}

/// Answers [`Catalogue::origin`] from the id's prefix, and nothing else.
struct ById;

/// Answers nothing at all.
struct Unreachable;

impl Catalogue for Unreachable {
    fn builtin(&self) -> Result<Vec<Voice>, VoiceError> {
        Err(down())
    }

    fn library(&self, _: &Filters) -> Result<Page, VoiceError> {
        Err(down())
    }

    fn one(&self, _: &str) -> Result<Availability, VoiceError> {
        Err(down())
    }

    fn origin(&self, _: &str) -> Result<Option<Origin>, VoiceError> {
        Err(down())
    }

    fn name(&self) -> &'static str {
        "unreachable voices"
    }
}

/// What a catalogue that cannot be reached answers.
fn down() -> VoiceError {
    VoiceError::Provider(ProviderError::new("test voices", "no route to host"))
}

impl Catalogue for ById {
    fn builtin(&self) -> Result<Vec<Voice>, VoiceError> {
        Err(unscripted())
    }

    fn library(&self, _: &Filters) -> Result<Page, VoiceError> {
        Err(unscripted())
    }

    fn one(&self, _: &str) -> Result<Availability, VoiceError> {
        Err(unscripted())
    }

    fn origin(&self, voice_id: &str) -> Result<Option<Origin>, VoiceError> {
        Ok(if voice_id.starts_with("gone-") {
            None
        } else if voice_id.starts_with("custom-") || voice_id.starts_with("voice-of-") {
            Some(Origin::Custom)
        } else if voice_id.starts_with("library-") {
            Some(Origin::Library)
        } else {
            Some(Origin::Builtin)
        })
    }

    fn name(&self) -> &'static str {
        "test voices"
    }
}

/// What every listing answers: the tests ask only where a voice comes from.
fn unscripted() -> VoiceError {
    VoiceError::Provider(ProviderError::new("test voices", "only origin is scripted"))
}
