//! What a spoken line asks for, beyond the words.
//!
//! The sibling of [`super::video`], and deliberately smaller. A generated shot
//! is a request with couplings in it — a raster fixes a length, a tier
//! withdraws a capability — and most of that module is those couplings. Speech
//! has one: a language pinned on a model that ignores it. Everything else here
//! is a plain choice.
//!
//! The size difference is the point rather than an accident of maturity. A
//! narration is a sentence, a voice and a price, and the reason to keep the
//! block this small is that every field added to it is a field hashed into the
//! brief for ever after — see [`MAX_CHARACTERS`] for the one limit that is the
//! vendor's, and the crate that hashes these for why the set is written once.

use serde::{Deserialize, Serialize};

/// The most characters one request may speak.
///
/// The vendor's limit, and checked here rather than discovered there: an
/// oversized block refused locally names the asset and costs nothing, where the
/// same refusal from the API costs a round trip to say less. At the dearer rate
/// this is also a ceiling on one call — forty thousand characters is $4.00.
pub const MAX_CHARACTERS: usize = 40_000;

/// Which text-to-speech model a line is spoken by.
///
/// Three, and no more. The vendor also publishes Turbo variants and its own
/// documentation recommends Flash over Turbo in every case, so offering both
/// would be offering a choice with a right answer.
///
/// Named for what the choice is *about* rather than for the vendor's version
/// strings, the way [`VideoModel`](super::VideoModel) is: a person picks
/// between expression and price, not between `v3` and `v2_5`. Each variant's
/// documentation carries the id it becomes on the wire, and so does
/// `docs/project-format.md`, so the mapping is one lookup away in either
/// direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechModel {
    /// The most expressive of the three — `eleven_v3`.
    ///
    /// Same price as [`SpeechModel::Standard`], so the choice between them is
    /// about the reading rather than the money.
    Expressive,
    /// The default, and the vendor's own — `eleven_multilingual_v2`.
    ///
    /// Default here because it is the model the API uses when a request names
    /// none, which means an asset that says nothing about its model and a
    /// request that says nothing about it produce the same audio. The one
    /// model that **ignores** [`SpeechRequest::language`] — see
    /// [`SpeechModel::takes_language`].
    #[default]
    Standard,
    /// Low latency, and half the price — `eleven_flash_v2_5`.
    ///
    /// For narration where the reading matters less than the money, which is
    /// the same judgement [`VideoModel::Lite`](super::VideoModel::Lite) exists
    /// for on the picture side.
    Fast,
}

impl SpeechModel {
    /// Every model, in the order a person should consider them.
    pub const ALL: [Self; 3] = [Self::Expressive, Self::Standard, Self::Fast];

    /// The model's name as `project.json` spells it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Expressive => "expressive",
            Self::Standard => "standard",
            Self::Fast => "fast",
        }
    }

    /// The vendor's id for this model, which is what reaches the wire.
    ///
    /// The only place the two vocabularies meet. A request body built anywhere
    /// else would be a second mapping, and the first person to notice the two
    /// disagreed would be the one who had paid for a reading in the wrong
    /// model.
    pub const fn model_id(self) -> &'static str {
        match self {
            Self::Expressive => "eleven_v3",
            Self::Standard => "eleven_multilingual_v2",
            Self::Fast => "eleven_flash_v2_5",
        }
    }

    /// True when this model honours [`SpeechRequest::language`].
    ///
    /// A question rather than a comparison, for the reason
    /// [`VideoModel::supports_reference_images`](super::VideoModel::supports_reference_images)
    /// is one: the interesting part is not which model is excluded but that the
    /// vendor **silently ignores** the field on it. A setting that is accepted
    /// and does nothing is indistinguishable from a bug, and the person
    /// wondering why their narration came out in English has no way in. So the
    /// combination is refused in the document instead.
    pub const fn takes_language(self) -> bool {
        matches!(self, Self::Expressive | Self::Fast)
    }
}

/// Everything a `generated_audio` asset asks for beyond its prompt.
///
/// Every field has a default or is optional, so an asset that says nothing here
/// is still a complete request but for one thing: there is no default voice,
/// and there cannot be — see [`SpeechRequest::voice_id`]. Absent from the
/// document entirely means the same as present and empty, which is why
/// [`crate::Asset::speech_request`] exists rather than callers reading the
/// option.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SpeechRequest {
    /// Which model speaks the line.
    pub model: SpeechModel,
    /// The vendor's id for the voice that speaks it.
    ///
    /// **No default is possible.** Every one of the vendor's Default voices
    /// expires on 2026-12-31 and the set is being replaced before then, so a
    /// fallback written here would be a guaranteed outage with a date on it.
    /// The list resolves at runtime instead.
    ///
    /// Optional in the document but not at generation time: a sketch written
    /// before anyone has chosen a voice is a legitimate document, exactly as a
    /// shot with no prompt yet is, and both are refused at the point of
    /// spending rather than at the point of writing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,
    /// The language to pin the reading to, as an ISO 639-1 code — `en`, `pt`.
    ///
    /// Only meaningful on a model that takes one. Left out, the model decides
    /// from the words, which is usually right and occasionally confident and
    /// wrong about a proper noun.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// A seed, for a reading that comes back the same way twice.
    ///
    /// Best-effort at the vendor and documented as such — worth recording
    /// because a reading somebody liked is worth trying to get again, and not
    /// worth relying on. It is hashed into the brief like everything else, so
    /// changing it asks for a different take rather than the same one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
}

impl SpeechRequest {
    /// Why this request cannot be spoken as written, or `None` when it can.
    ///
    /// Only the couplings — the checks that need the *rest* of the asset, the
    /// prompt's length among them, live in validation where the asset is in
    /// hand. What is here is the one thing the request can get wrong on its
    /// own.
    pub fn conflict(&self) -> Option<LanguageIgnored> {
        let language = self.language.as_ref()?;
        if self.model.takes_language() {
            return None;
        }
        Some(LanguageIgnored {
            model: self.model.as_str(),
            language: language.clone(),
        })
    }
}

/// A language pinned on the one model that will not honour it.
///
/// Carried rather than inferred, for the reason
/// [`LengthLock`](super::LengthLock) is: *you cannot pin the language* is a
/// third of an explanation, and what the reader needs is which of their other
/// choices took the option away — since that is the one they might reconsider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageIgnored {
    /// The model that ignores it, as the document spells it.
    pub model: &'static str,
    /// The language code that would have gone unheeded.
    pub language: String,
}
