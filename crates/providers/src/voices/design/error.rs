//! Why a voice was not designed, or not kept.
//!
//! Every variant here exists because its **advice differs** — the same test the
//! listing errors are held to next door. Two failures a person fixes the same
//! way are one failure with two spellings; two fixed in different places must
//! never share a message.
//!
//! What is *not* here is anything a listing already names. A key that is
//! absent, a key that is valid and unscoped, a provider that cannot be reached:
//! those arrive as [`VoiceError`] and are carried through whole, because the
//! sentence that tells somebody to look in the ElevenLabs dashboard rather than
//! in `.env` should not be rewritten twice.
//!
//! The variants that *are* here are the ones that only exist because this
//! spends money and leaves something behind: a request the vendor's own limits
//! refuse, a ceiling that would be crossed, a plan that does not include the
//! feature, and a candidate that has gone.

use crate::api::elevenlabs::design::{PASSAGE, PROMPT};
use crate::credentials::OverBudget;

use super::super::VoiceError;

/// Why a design did not happen.
#[derive(Debug, thiserror::Error)]
pub enum DesignError {
    /// Something a listing already names: no key, an unscoped key, a provider
    /// that would not answer.
    #[error(transparent)]
    Voice(#[from] VoiceError),

    /// The run would cross the spending ceiling.
    ///
    /// **Not negotiable by a flag**, exactly as generating a shot is not: the
    /// ceiling exists for the run nobody is watching, and a limit a flag can
    /// wave through is not a limit.
    #[error(transparent)]
    OverBudget(#[from] OverBudget),

    /// The description is outside the length the vendor accepts.
    ///
    /// Refused here rather than remotely, because a `422` costs a round trip to
    /// say less than this does — and because the lower bound is a real piece of
    /// advice: twenty characters is about as short as a description can be and
    /// still describe anything.
    #[error(
        "a voice description has to be between {} and {} characters, and this one is {characters}. \
         Say who is speaking and how — age, accent, pace, warmth — rather than naming a person.",
        PROMPT.start(),
        PROMPT.end()
    )]
    PromptLength {
        /// How long it actually was.
        characters: usize,
    },

    /// The passage the previews read is outside the length the vendor accepts.
    #[error(
        "the passage a preview reads has to be between {} and {} characters, and this one is \
         {characters}. It is also the only thing this call is billed for, so a long one is a \
         better audition and a dearer one.",
        PASSAGE.start(),
        PASSAGE.end()
    )]
    PassageLength {
        /// How long it actually was.
        characters: usize,
    },

    /// Voice Design is not sold to this account through the API.
    ///
    /// Its own variant because the advice is specific and nothing else gives
    /// it: the built-in voices and — on a paid plan — the Voice Library still
    /// answer, and one of those is what to use. No subscription tier is
    /// detected to produce this; it is only ever what the vendor said when
    /// asked, which needs no account introspection at all.
    #[error(
        "ElevenLabs would not design a voice for this account: {said}\n\
         Voice Design is not available through the API on a free plan. `scorsese voices` lists \
         the built-in voices, and one from there narrates exactly the same way."
    )]
    NotOnThisPlan {
        /// What the vendor said, verbatim.
        said: String,
    },

    /// A candidate id that no design in this project produced.
    ///
    /// The likeliest way to reach it is a typo; the next likeliest is a
    /// candidate from a design whose samples have been cleared out. Either way
    /// nothing has been created and nothing spent.
    #[error(
        "no design in this project offered {chosen}. Run the design again — the candidates from \
         one are listed with it, and a candidate id is only ever one of those."
    )]
    NoSuchCandidate {
        /// The id that was asked for.
        chosen: String,
    },

    /// The vendor answered a design with nothing to listen to.
    ///
    /// Named rather than reported as an empty success, because an empty success
    /// is a surface printing "designed 0 candidates" over a call that was
    /// billed.
    #[error(
        "{provider} answered the design with no candidates at all. Nothing was created, but the \
         passage may still have been billed — try again before rewriting the description."
    )]
    NoCandidates {
        /// Who was asked.
        provider: &'static str,
    },

    /// A sample, or a record of a design, could not be written.
    ///
    /// Fatal rather than ignored, and that is the difference between this and a
    /// voice listing's cache: a listing re-reads for free, while these are
    /// bytes somebody paid for and a record without which a voice cannot be
    /// regenerated.
    #[error("{doing}: {source}")]
    Write {
        /// What was being written when it failed.
        doing: String,
        /// What the filesystem said.
        source: std::io::Error,
    },
}
