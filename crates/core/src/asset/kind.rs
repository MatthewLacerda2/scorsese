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
    /// A string rendered as picture. Its content lives inline in
    /// `project.json` rather than in a file.
    Text,
    /// A solid colour filling the whole raster: a background, a colour card, a
    /// wash under a title. Like [`AssetKind::Text`] it has no file behind it,
    /// and it is simpler still — no content at all, only appearance.
    ///
    /// Resolution-independent by construction. It is whatever the render is,
    /// so nothing about it carries a raster the project should not know.
    Color,
    /// A rectangle or an ellipse, drawn by the render rather than imported as a
    /// picture of one. The third kind with no file behind it, and
    /// resolution-independent for [`AssetKind::Color`]'s reason: what it
    /// carries is fractions of the raster, so the drawing happens at whatever
    /// size the render turns out to be.
    Shape,
    /// A symbol from the set this build ships, named rather than imported —
    /// the fourth kind with no file behind it, and the only one whose content
    /// is a *reference* to something the binary carries.
    ///
    /// A name is portable in a way a path is not: `clapperboard` survives
    /// `scp -r` because the symbols travel with the binary, exactly as a
    /// `style`'s `sans` does. Which names exist is not this crate's business —
    /// see [`crate::Icon`].
    Icon,
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
            Self::Video
                | Self::Image
                | Self::Text
                | Self::Color
                | Self::Shape
                | Self::Icon
                | Self::GeneratedVideo
        )
    }

    /// True when this kind produces sound, and so belongs on an audio track.
    pub fn is_audible(self) -> bool {
        matches!(self, Self::Audio | Self::GeneratedAudio | Self::SynthAudio)
    }

    /// True when a file on disk is what this kind ultimately refers to.
    ///
    /// The **inline** kinds are the exception — `text`, `color`, `shape` and
    /// `icon` say what they are in the document itself. Most of what follows
    /// from that is asked here rather than of the kind directly: they cannot be
    /// imported, there is nothing to hash or probe, and `fit` has no source
    /// raster to reconcile against.
    ///
    /// An `icon` belongs with them even though something is read to draw it:
    /// what it names is compiled into the binary, so there is no path in the
    /// project and nothing that could be missing after a copy.
    pub fn is_file_backed(self) -> bool {
        !matches!(self, Self::Text | Self::Color | Self::Shape | Self::Icon)
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
