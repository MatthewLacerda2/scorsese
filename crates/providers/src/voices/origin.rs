//! Where an ElevenLabs voice comes from (#1655): built-in, Voice Library, or
//! custom to the account — [`Origin`], read from the vendor's own shapes.

use crate::api::elevenlabs::voices::{self, Listing};
use crate::api::http::HttpError;

use super::catalogue::Origin;
use super::elevenlabs::NAME;
use super::error::VoiceError;
use super::refusal::{failure, refused, unusable};

/// Where `voice_id` comes from, from what the account lookup answered and the
/// Voice Library searched for it — passed in, as
/// `ElevenLabsVoices::one`'s are, so the decision is tested on the vendor's shapes without a network.
///
/// **`premade` is the one category that settles it alone**: a built-in voice
/// is in every account and is nobody's. Any other voice the account holds is
/// a Library voice only when the Library publishes that exact id, and
/// otherwise was designed or cloned in the account. A voice the account does
/// not hold (or may not use) is a Library one or nothing at all.
pub(super) fn origin(
    voice_id: &str,
    held: Result<voices::Voice, HttpError>,
    library: impl FnOnce() -> Result<Listing, HttpError>,
) -> Result<Option<Origin>, VoiceError> {
    let held = match held {
        Ok(found) if found.category.as_deref() == Some("premade") => {
            return Ok(Some(Origin::Builtin));
        }
        Ok(_) => true,
        Err(error) => match refused(voice_id, &error).as_ref().and_then(unusable) {
            Some(_) => false,
            None => return Err(failure(NAME, error)),
        },
    };
    let listing = library().map_err(|error| failure(NAME, error))?;
    let shared = listing
        .voices
        .iter()
        .any(|found| found.voice_id == voice_id);
    Ok(match (shared, held) {
        (true, _) => Some(Origin::Library),
        (false, true) => Some(Origin::Custom),
        (false, false) => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `GET /v1/voices/{id}` answered on 2026-10-16 for a Library voice
    /// the account had not saved (#1424).
    const NOT_HELD: &str = concat!(
        r#"{"detail":{"type":"not_found","code":"voice_not_found","#,
        r#""message":"A voice with ID 'x' was not found."}}"#
    );

    /// A refusal as the transport hands one over.
    fn answered(status: u16, body: &str) -> HttpError {
        HttpError::Refused {
            url: String::from("https://api.elevenlabs.io/v1/voices/x"),
            status,
            body: String::from(body),
        }
    }

    /// The Library search answering with these voices.
    fn searched(ids: &[&str]) -> Result<Listing, HttpError> {
        Ok(Listing {
            voices: ids
                .iter()
                .map(|id| voices::Voice {
                    voice_id: (*id).to_owned(),
                    ..voices::Voice::default()
                })
                .collect(),
            ..Listing::default()
        })
    }

    /// A voice the account holds, in `category`.
    fn held(category: &str) -> Result<voices::Voice, HttpError> {
        Ok(voices::Voice {
            voice_id: String::from("x"),
            category: Some(category.to_owned()),
            ..voices::Voice::default()
        })
    }

    /// A built-in voice is settled by its category, with no search.
    #[test]
    fn a_premade_voice_is_built_in() {
        let answer = origin("x", held("premade"), || panic!("the Library was searched"));
        assert_eq!(answer.unwrap(), Some(Origin::Builtin));
    }

    /// Held and published: a Library voice the account saved. Held and not
    /// published: one designed or cloned in the account (#1655).
    #[test]
    fn a_held_voice_is_custom_unless_the_library_publishes_it() {
        for category in ["generated", "cloned", "professional"] {
            let answer = origin("x", held(category), || searched(&["x"]));
            assert_eq!(answer.unwrap(), Some(Origin::Library), "{category}");
            let answer = origin("x", held(category), || searched(&["xy"]));
            assert_eq!(answer.unwrap(), Some(Origin::Custom), "{category}");
        }
    }

    /// Not held: the Library's, or nothing's.
    #[test]
    fn a_voice_the_account_lacks_is_the_librarys_or_nothing() {
        let answer = origin("x", Err(answered(400, NOT_HELD)), || searched(&["x"]));
        assert_eq!(answer.unwrap(), Some(Origin::Library));
        let answer = origin("x", Err(answered(400, NOT_HELD)), || searched(&["y"]));
        assert_eq!(answer.unwrap(), None);
    }

    /// A lookup or a search that got no answer decides nothing.
    #[test]
    fn an_origin_that_could_not_be_asked_is_a_failure() {
        let unreachable = || HttpError::Unreachable {
            url: String::from("https://api.elevenlabs.io/v1/voices/x"),
            message: String::from("no route to host"),
        };
        let answer = origin("x", Err(unreachable()), || panic!("searched"));
        assert!(matches!(answer, Err(VoiceError::Provider(_))), "{answer:?}");
        let answer = origin("x", held("generated"), || Err(unreachable()));
        assert!(matches!(answer, Err(VoiceError::Provider(_))), "{answer:?}");
    }
}
