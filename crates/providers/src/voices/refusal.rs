//! An ElevenLabs refusal, as a fact about a voice.
//!
//! Nothing here reads a refusal body. [`Refusal`] does that, once, for the
//! whole of scorsese — and this module **projects** its answer onto the
//! narrower question this one asks: *may this voice be used*. That is the
//! entire job, and the reason it is worth a file of its own is what happens
//! when it is not one.
//!
//! Sorting the body a second time here would be a second opinion about it. The
//! two would agree on the day they were written and drift on the day the vendor
//! changes something — and the drift has a date already on it, because **every
//! ElevenLabs default voice expires on 2026-12-31** and the default set is
//! being replaced before then. If the shape of that refusal shifts, one reading
//! gets fixed and the other keeps its old opinion, and the two disagree about
//! whether a voice is gone. Which is exactly the question somebody is asking
//! when they hit it.
//!
//! What each caller still decides for itself is what a refusal means **on its
//! own endpoint**: a plan limit is a Voice Library that is not sold on a
//! listing, an unusable voice on a lookup, and no candidates on a design. That
//! is a fact about which call was made rather than about the body, so it stays
//! at the call.

use crate::api::elevenlabs::refusal::Refusal;
use crate::api::http::HttpError;
use crate::video::ProviderError;

use super::catalogue::Unusable;
use super::error::VoiceError;

/// The voice id to sort a refusal with when the call was not about a voice.
///
/// A listing has none to name, and a `404` from one is not a voice going
/// missing — it is an endpoint that moved. [`unusable`] is the only thing that
/// reads [`Refusal::VoiceGone`], and it is only ever consulted where a voice
/// was genuinely asked about, so this never reaches a message.
pub(super) const NO_VOICE: &str = "";

/// What the vendor's refusal means, or `None` where there was nothing to read.
///
/// `None` for anything that never got an answer at all — no network, DNS, TLS,
/// a timeout — because those have no status and no body to interpret and must
/// never be mistaken for one. A flaky connection that read as a refusal is how
/// somebody's chosen narrator gets quietly deleted.
pub(super) fn refused(voice_id: &str, error: &HttpError) -> Option<Refusal> {
    match error {
        HttpError::Refused { status, body, .. } => Some(Refusal::of(*status, voice_id, body)),
        _ => None,
    }
}

/// The vendor's sentence, where this refusal is about the account's plan rather
/// than about the request.
///
/// The sentence rather than a `bool`, because it is the part that says *which*
/// limit was hit, and every message built from this carries it whole.
pub(super) fn plan_limited(refusal: &Refusal) -> Option<&str> {
    match refusal {
        Refusal::PlanRequired { message } => Some(message),
        _ => None,
    }
}

/// The refusal as a reason a voice cannot be used, where it is one.
///
/// `None` for every refusal that says nothing about the voice — a key with no
/// scope, a spent quota, a busy vendor. Those are failures to *ask*, and the
/// caller turns them into one.
pub(super) fn unusable(refusal: &Refusal) -> Option<Unusable> {
    match refusal {
        Refusal::VoiceGone { .. } => Some(Unusable::Gone),
        other => plan_limited(other).map(|said| Unusable::NotOnThisPlan {
            said: said.to_owned(),
        }),
    }
}

/// A failed call as the error it actually is.
///
/// The one distinction worth making here is the one a status code cannot make:
/// a `401` because the key is absent or wrong, against a `401` because the key
/// is perfectly good and was never granted the permission this call needs. They
/// are fixed in different places — one in `.env`, one in the vendor's dashboard
/// — so reporting them the same way sends half of the people who hit this to
/// look at something that is not wrong. The vendor's own sentence names the
/// permission, so it is carried whole rather than summarised.
pub(super) fn failure(provider: &'static str, error: HttpError) -> VoiceError {
    if let Some(Refusal::MissingPermission { message }) = refused(NO_VOICE, &error) {
        return VoiceError::MissingPermission { said: message };
    }
    VoiceError::Provider(ProviderError::new(provider, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the live API answered when a free account asked for a Voice Library
    /// voice. Note where the vendor put its word for it: `paid_plan_required`
    /// is the `code`, and `status` says something else entirely.
    const FREE_PLAN: &str = concat!(
        r#"{"detail":{"type":"payment_required","code":"paid_plan_required","#,
        r#""message":"Free users cannot use library voices via the API.","#,
        r#""status":"payment_required"}}"#
    );

    /// What it answered when the key was valid and lacked `voices_read`.
    const MISSING_SCOPE: &str = concat!(
        r#"{"detail":{"type":"authentication_error","code":"unauthorized","#,
        r#""message":"The API key you used is missing the permission voices_read "#,
        r#"to execute this operation.","status":"missing_permissions"}}"#
    );

    /// A refusal as the transport hands one over.
    fn answered(status: u16, body: &str) -> HttpError {
        HttpError::Refused {
            url: String::from("https://api.elevenlabs.io/v1/voices/v1"),
            status,
            body: String::from(body),
        }
    }

    /// **The test this module exists for.** One body, one verdict: what the
    /// speaking path calls a withdrawn voice is what the voices path calls an
    /// unusable one, because the second is the first projected rather than
    /// worked out again.
    #[test]
    fn a_withdrawn_voice_is_one_verdict_reached_once() {
        let sorted = refused("v1", &answered(404, "{}")).expect("the vendor answered");

        assert_eq!(
            sorted,
            Refusal::VoiceGone {
                voice_id: String::from("v1")
            }
        );
        assert_eq!(unusable(&sorted), Some(Unusable::Gone));
    }

    /// The plan limit, and the sentence has to survive intact on both sides: it
    /// is the part that says which limit was hit.
    #[test]
    fn a_plan_limit_carries_the_same_sentence_to_both_paths() {
        let sorted = refused("v1", &answered(402, FREE_PLAN)).expect("the vendor answered");

        let Refusal::PlanRequired { message } = &sorted else {
            panic!("a free plan is a plan limit, not {sorted:?}")
        };
        assert!(message.contains("library voices"), "{message}");
        assert_eq!(plan_limited(&sorted), Some(message.as_str()));
        assert_eq!(
            unusable(&sorted),
            Some(Unusable::NotOnThisPlan {
                said: message.clone()
            })
        );
    }

    /// The vendor spells the same limit two ways, and a bare `403` is the other
    /// one. Both paths have to read it as the plan rather than as a mystery.
    #[test]
    fn a_bare_403_is_the_same_limit_as_a_402() {
        let sorted = refused("v1", &answered(403, "{}")).expect("the vendor answered");

        assert!(plan_limited(&sorted).is_some(), "{sorted:?}");
        assert!(
            matches!(unusable(&sorted), Some(Unusable::NotOnThisPlan { .. })),
            "{sorted:?}"
        );
    }

    /// A key with no scope says nothing whatever about the voice. It is a
    /// failure to *ask*, and reading it as an unusable voice would send
    /// somebody hunting for a replacement they do not need.
    #[test]
    fn a_key_missing_a_scope_is_never_a_voice_being_unusable() {
        let error = answered(401, MISSING_SCOPE);
        let sorted = refused("v1", &error).expect("the vendor answered");

        assert!(
            matches!(sorted, Refusal::MissingPermission { .. }),
            "{sorted:?}"
        );
        assert_eq!(unusable(&sorted), None);
        match failure("ElevenLabs", error) {
            VoiceError::MissingPermission { said } => {
                assert!(said.contains("voices_read"), "{said}");
            }
            other => panic!("a valid key with no scope is not {other:?}"),
        }
    }

    /// **The one that must never be confused.** A timeout has no status and no
    /// body, so there is no refusal to project at all.
    #[test]
    fn a_provider_that_never_answered_is_no_refusal_to_read() {
        let error = HttpError::Unreachable {
            url: String::from("https://api.elevenlabs.io/v1/voices/v1"),
            message: String::from("no route to host"),
        };

        assert_eq!(refused("v1", &error), None);
        assert!(matches!(
            failure("ElevenLabs", error),
            VoiceError::Provider(_)
        ));
    }
}
