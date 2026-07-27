//! What an asset *is*, and — for the ones that do not exist yet — how far
//! along it is.

use serde::{Deserialize, Serialize};

/// What kind of media an asset is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// A moving-picture file, which may carry sound of its own.
    Video,
    /// A still. It has no duration — how long it is on screen is the clip's
    /// business, not the file's.
    Image,
    /// A sound file, and the only imported kind that belongs on an audio track.
    Audio,
    /// A string rendered as picture. The one kind with no file behind it: the
    /// content lives inline in `project.json`.
    Text,
    /// A Veo prompt: video that does not exist until it is generated.
    GeneratedVideo,
    /// An ElevenLabs TTS prompt: audio that does not exist until generated.
    GeneratedAudio,
    /// A synthesis recipe: audio computed from a document the project carries,
    /// rather than asked for in words. Free, offline, and the same bytes every
    /// time — see [`AssetKind::is_synthesized`].
    SynthAudio,
}

impl AssetKind {
    /// True for the kinds that do not exist until something makes them: they
    /// carry a [`GenerationState`], their output lands in `generated/`, and
    /// they render as a stand-in until it does.
    ///
    /// This says nothing about what the brief *is* — see
    /// [`AssetKind::is_prompted`] and [`AssetKind::is_synthesized`] for that.
    pub fn is_generated(self) -> bool {
        matches!(
            self,
            Self::GeneratedVideo | Self::GeneratedAudio | Self::SynthAudio
        )
    }

    /// True when the brief is a sentence of natural language, which is also
    /// what makes realising it cost money and need a network.
    pub fn is_prompted(self) -> bool {
        matches!(self, Self::GeneratedVideo | Self::GeneratedAudio)
    }

    /// True when the brief is a *document* the project carries — a recipe —
    /// and realising it is a deterministic local computation.
    ///
    /// The distinction from [`AssetKind::is_prompted`] is not decoration: it
    /// decides which field holds the brief, whether GO has anything to charge
    /// for, and whether the result can be reproduced from the project alone.
    pub fn is_synthesized(self) -> bool {
        matches!(self, Self::SynthAudio)
    }

    /// True when this kind produces picture, and so belongs on a video track.
    pub fn is_visual(self) -> bool {
        matches!(
            self,
            Self::Video | Self::Image | Self::Text | Self::GeneratedVideo
        )
    }

    /// True when this kind produces sound, and so belongs on an audio track.
    pub fn is_audible(self) -> bool {
        matches!(self, Self::Audio | Self::GeneratedAudio | Self::SynthAudio)
    }

    /// True when a file on disk is what this kind ultimately refers to.
    /// `text` is the exception: it carries its content inline.
    pub fn is_file_backed(self) -> bool {
        !matches!(self, Self::Text)
    }
}

/// Where a generated asset sits in the sketch lifecycle.
///
/// `sketch → queued → generated`, and back to `stale` when the brief is
/// edited after generation. Sketch and stale clips render as slug cards, so a
/// full preview cut costs nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationState {
    /// A brief nobody has realised yet. Where every generated asset starts.
    Sketch,
    /// Handed to the provider and in flight. GO leaves it alone rather than
    /// paying for it twice.
    Queued,
    /// The media exists on disk. A cache hit for as long as the brief is
    /// unchanged, and never re-billed.
    Generated,
    /// Generated once, then the brief was edited — so the file on disk is no
    /// longer what the project asks for, and GO will redo it.
    Stale,
}

impl GenerationState {
    /// True for the states GO acts on. `generated` is a cache hit and is
    /// never regenerated; `queued` is already in flight.
    pub fn needs_generation(self) -> bool {
        matches!(self, Self::Sketch | Self::Stale)
    }

    /// True when this state implies a media file should exist on disk.
    pub fn has_media(self) -> bool {
        matches!(self, Self::Generated)
    }
}
