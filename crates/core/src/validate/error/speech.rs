//! What a spoken line's request can get wrong.
//!
//! The sibling of [`super::VideoProblem`], and it exists for the same reason:
//! these are not the usual missing-or-stray finding. The fields are present and
//! individually legal, and what is wrong is either a *combination* the vendor
//! will not honour or a size it will not accept.
//!
//! Both are answerable from the document, which is the whole point of asking
//! here. One of them — the language the model ignores — is not even an error at
//! the vendor: the request succeeds, the field is dropped, and the narration
//! comes back read in whatever language the model guessed. That failure has no
//! symptom except somebody listening, so the document is the only place it can
//! be caught at all.

use crate::asset::AssetId;

/// One thing wrong with what a spoken line is asking for.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SpeechProblem {
    /// A language pinned on the one model that silently drops it.
    ///
    /// **Refused rather than ignored, and that asymmetry is deliberate.** The
    /// vendor accepts this request and charges for it; the field simply does
    /// nothing. So the choice is between refusing a document that would work
    /// and shipping narration in a language nobody chose — and only one of
    /// those tells anyone what happened.
    #[error(
        "asset `{asset}` pins the language to `{language}`, \
         but the `{model}` model ignores that field — \
         use `expressive` or `fast`, or drop the language"
    )]
    LanguageIgnored {
        /// The asset asking for it.
        asset: AssetId,
        /// The model that would drop it.
        model: &'static str,
        /// The language code that would go unheeded.
        language: String,
    },

    /// More characters than the vendor speaks in one request.
    ///
    /// Named with both numbers because the fix is a decision about where to
    /// cut, and "too long" gives nobody a place to make it.
    #[error(
        "asset `{asset}` asks for {found} characters of speech, and at most {max} are accepted — \
         split the narration across two assets"
    )]
    TooLong {
        /// The asset asking for it.
        asset: AssetId,
        /// How many characters the prompt holds.
        found: usize,
        /// How many the vendor takes in one request.
        max: usize,
    },

    /// A voice named by an empty string.
    ///
    /// Its own problem rather than being folded into *no voice chosen*, because
    /// the two are different mistakes. An absent voice is a sketch nobody has
    /// finished, which is a legitimate document and is refused only at the
    /// point of spending. An empty one is a field somebody cleared or a
    /// template that never got filled in — it looks chosen and is not.
    #[error("asset `{asset}` has an empty `voice_id`: leave it out, or name a voice")]
    BlankVoice {
        /// The asset carrying it.
        asset: AssetId,
    },
}
