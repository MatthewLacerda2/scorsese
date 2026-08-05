//! Reading an ElevenLabs refusal, once, for every call in this module.
//!
//! Listing voices and designing one are different calls to the same vendor, and
//! they are refused in the same vocabulary: a `401` that is a wrong key or a
//! valid key missing a scope, a `402`/`403` that is the account's plan rather
//! than the request, and a body carrying the sentence that says which. Reading
//! that vocabulary twice would be two places for the two spellings of a `401`
//! to drift apart — and the whole reason the body is read at all is that
//! collapsing them sends somebody to fix a credential that is already correct.
//!
//! So the interpretation lives here and the *meaning* stays with each call.
//! What a `404` means is a fact about the endpoint that answered it — a
//! withdrawn voice on a lookup, something else entirely elsewhere — and this
//! module deliberately has no opinion about it.

use crate::api::elevenlabs::refusal::{self, Detail, MISSING_PERMISSIONS, PAID_PLAN_REQUIRED};
use crate::api::http::HttpError;
use crate::video::ProviderError;

use super::error::VoiceError;

/// A refusal's status and whatever the vendor said inside it.
///
/// `None` for anything that never got an answer at all — no network, DNS, TLS,
/// a timeout — because those have no status to interpret and must not be
/// mistaken for one.
pub(super) fn refused(error: &HttpError) -> Option<(u16, Detail)> {
    match error {
        HttpError::Refused { status, body, .. } => Some((*status, refusal::read(body))),
        _ => None,
    }
}

/// Whether this refusal is the account's plan rather than its request.
///
/// Both spellings the vendor uses: `402 paid_plan_required` on something a free
/// account may not have, and a bare `403`. The status word is checked first
/// where there is one, because it is the vendor being explicit and the number
/// is only ever an inference.
pub(super) fn plan_limited(status: u16, detail: &Detail) -> bool {
    detail.is(PAID_PLAN_REQUIRED) || matches!(status, 402 | 403)
}

/// The best sentence available about a refusal.
pub(super) fn said<'a>(error: &'a HttpError, detail: &'a Detail) -> &'a str {
    match error {
        HttpError::Refused { body, .. } => detail.says(body),
        _ => "",
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
    if let Some((_, detail)) = refused(&error)
        && detail.is(MISSING_PERMISSIONS)
    {
        return VoiceError::MissingPermission {
            said: said(&error, &detail).to_owned(),
        };
    }
    VoiceError::Provider(ProviderError::new(provider, error))
}
