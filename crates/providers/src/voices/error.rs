//! Why a list, or a voice, did not come back.
//!
//! Every variant here exists because its **advice differs**. That is the whole
//! test for whether a case deserves its own name: two failures that a person
//! fixes the same way are one failure with two spellings, and two that are
//! fixed in different places must never share a message.
//!
//! The case that proves the rule is [`VoiceError::MissingPermission`]. It
//! arrives as a `401`, exactly like an absent key — and the fix is in the
//! vendor's dashboard rather than in `.env`. Collapsing the two, which reading
//! only the status code would do, sends somebody to check a credential that is
//! already correct.

use crate::api::elevenlabs::voices::MAX_PAGE_SIZE;
use crate::credentials::CredentialError;
use crate::video::ProviderError;

use super::catalogue::Unusable;

/// Why a listing did not happen.
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    /// No key for ElevenLabs anywhere the resolver looks.
    #[error(transparent)]
    Credential(#[from] CredentialError),

    /// The provider could not be reached, or refused in a way nothing below
    /// names — and there was nothing cached to fall back on.
    #[error(transparent)]
    Provider(#[from] ProviderError),

    /// The key is valid and was never granted the scope this call needs.
    ///
    /// **Not a credential problem**, however much the status code looks like
    /// one. The vendor's sentence names the exact permission — `voices_read` —
    /// and it is carried whole for that reason: summarising it would drop the
    /// only word that makes this fixable.
    #[error(
        "the ElevenLabs key is valid but is not allowed to do this: {said}\n\
         Add that permission to the key in the ElevenLabs dashboard, under Profile → API \
         Keys. There is nothing wrong with the key itself, so there is nothing to change \
         in .env or the settings file."
    )]
    MissingPermission {
        /// What the vendor said, verbatim, permission name included.
        said: String,
    },

    /// More voices were asked for than the vendor puts on a page. Refused here
    /// rather than remotely: a round trip to be told a number is too big costs
    /// a second and says less than this does.
    #[error("page_size {asked} is more than the {MAX_PAGE_SIZE} one page holds")]
    PageTooLarge {
        /// What was asked for.
        asked: u32,
    },

    /// The Voice Library is not sold through the API to this account.
    ///
    /// Its own variant because the advice is specific and nothing else here
    /// gives it: the built-in voices still work, and that is what to use. No
    /// subscription tier is detected to produce this — it is only ever what the
    /// vendor answered when asked, which needs no account introspection at all.
    #[error(
        "ElevenLabs would not serve the Voice Library to this account: {said}\n\
         The Voice Library is not available through the API on a free plan. The built-in \
         voices are — `scorsese voices` lists them, and a voice from there generates \
         exactly the same way."
    )]
    NoLibraryOnThisPlan {
        /// What the vendor said, verbatim.
        said: String,
    },

    /// A voice that cannot be generated with. **The recoverable one.**
    ///
    /// Nothing is changed and nothing is substituted: the asset still says what
    /// it said, and whoever wrote it picks again. Silently falling back to
    /// another voice would be the worst outcome available here — narration
    /// would be produced, billed, and read by somebody nobody chose, and it
    /// would sound perfectly fine.
    #[error(
        "{voice_id} cannot be used at {provider}: {why}. \
         Run `scorsese voices` and pick another — nothing has been changed."
    )]
    Unusable {
        /// The id that will not do.
        voice_id: String,
        /// Who was asked.
        provider: &'static str,
        /// Which of the reasons it was.
        why: Unusable,
    },
}
