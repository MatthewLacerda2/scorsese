//! What a generated music track asks for, beyond its description.
//!
//! The second block a `generated_audio` asset can carry, beside
//! [`super::speech`]: the prompt is a description of the music — *calm
//! lo-fi, soft piano* — rather than words to be said, and what else the
//! vendor needs is a length and whether it may sing (#1264).
//!
//! **Exactly one block per asset**, and which one it carries is what it is:
//! a `music` block makes the asset a track, and no block at all keeps it a
//! line of narration, as every `generated_audio` asset was before music
//! arrived. [`AudioBlock`] is that answer, asked once.
//!
//! Smaller than the speech block on purpose. ElevenLabs' music endpoint takes
//! a model, a length, `force_instrumental` and a seed that it refuses beside a
//! prompt (its API reference, read 2026-10-16), so there is no seed here; and
//! one model is offered, the vendor's newest, so there is no model either —
//! both are written into the brief's fingerprint so that choosing one later
//! does not re-bill the tracks already made.

use serde::{Deserialize, Serialize};

/// The shortest track the vendor composes, in seconds: `music_length_ms` is
/// at least 3,000 (its API reference, read 2026-10-16).
pub const MIN_MUSIC_SECONDS: u32 = 3;

/// The longest track the vendor composes in one request, in seconds:
/// `music_length_ms` is at most 600,000 — ten minutes (its API reference,
/// read 2026-10-16).
pub const MAX_MUSIC_SECONDS: u32 = 600;

/// Everything a generated track asks for beyond its description.
///
/// Every field has a default or is optional, so `"music": {}` is a legal
/// sketch, and it is the length alone that a track needs before it can be
/// paid for — see [`MusicRequest::seconds`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MusicRequest {
    /// How long the track is, in whole seconds — between
    /// [`MIN_MUSIC_SECONDS`] and [`MAX_MUSIC_SECONDS`].
    ///
    /// Optional in the document and required at spending, the way a line's
    /// voice is: the vendor would pick a length itself, but the price is per
    /// minute, and a quote with no length is not a quote. Usually the length
    /// of the cut the track sits under; a track is cropped like any audio
    /// clip, so a little longer is never wrong.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u32>,
    /// Whether the track is guaranteed to have no singing.
    ///
    /// **On by default, where the vendor's own default is off.** A generated
    /// track in an edit is almost always a bed under narration or a
    /// montage, and words sung under a voice fight it; Filmora's music
    /// library, the reference for taste, is mostly instrumental too. A song
    /// with lyrics is one `false` away, with the lyrics described in the
    /// prompt.
    pub instrumental: bool,
}

impl Default for MusicRequest {
    fn default() -> Self {
        Self {
            seconds: None,
            instrumental: true,
        }
    }
}

/// Which block a `generated_audio` asset is realised from.
///
/// One question with one answer, asked by everything that has to tell a
/// track from a line — the passes, the quote, the jobs — so none of them
/// decides it its own way. A new block (sound effects, #1265) is a new
/// variant here, and every `match` over it says where it has to be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioBlock {
    /// A line of narration, read by a voice — the `speech` block, or no
    /// block at all.
    Speech,
    /// A track of music, composed from a description — the `music` block.
    Music,
}
