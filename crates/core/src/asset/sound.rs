//! What a generated sound effect asks for, beyond its description.
//!
//! The third block a `generated_audio` asset can carry, beside
//! [`super::speech`] and [`super::music`]: the prompt describes a sound — *a
//! creaking door*, *a whoosh* — and what else the vendor needs is a length
//! and whether it loops (#1265).
//!
//! **A recipe comes first.** A sound a `synth_audio` recipe can make is made
//! there, free and reproducible; this block is for what a recipe cannot make
//! — a real door, a crowd, rain on a window.
//!
//! Smaller than the vendor's request on purpose. ElevenLabs' sound endpoint
//! also takes a model and a `prompt_influence` (its API reference, read
//! 2026-11-04); one model is offered, the vendor's newest, and the influence
//! stays at the vendor's default — both are written into the brief's
//! fingerprint so that choosing one later does not re-bill the sounds already
//! made.

use serde::{Deserialize, Serialize};

/// The shortest sound the vendor makes, in seconds: `duration_seconds` is at
/// least 0.5 (its API reference, read 2026-11-04).
pub const MIN_SOUND_SECONDS: f64 = 0.5;

/// The longest sound the vendor makes in one request, in seconds:
/// `duration_seconds` is at most 30 (its API reference, read 2026-11-04).
pub const MAX_SOUND_SECONDS: f64 = 30.0;

/// Everything a generated sound effect asks for beyond its description.
///
/// Every field has a default or is optional, so `"sound": {}` is a legal
/// sketch, and it is the length alone that a sound needs before it can be
/// paid for — see [`SoundRequest::seconds`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SoundRequest {
    /// How long the sound is, in seconds — fractions allowed, between
    /// [`MIN_SOUND_SECONDS`] and [`MAX_SOUND_SECONDS`].
    ///
    /// Optional in the document and required at spending, the way a track's
    /// length is: the vendor would infer a length from the prompt, but the
    /// price is per minute, and a quote with no length is not a quote.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<f64>,
    /// Whether the sound is made to loop seamlessly — a bed of rain or a
    /// room tone laid under a whole scene. Off by default: most effects
    /// happen once.
    #[serde(rename = "loop", skip_serializing_if = "std::ops::Not::not")]
    pub looping: bool,
}

impl SoundRequest {
    /// Whether `seconds` is a length the vendor makes.
    pub fn takes(seconds: f64) -> bool {
        (MIN_SOUND_SECONDS..=MAX_SOUND_SECONDS).contains(&seconds)
    }
}
