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
    /// Stills played in order, each held for a number of frames, once or on a
    /// loop — a directory of rendered frames, a timelapse, stop motion, a
    /// flickering sign. See [`crate::ImageSequence`].
    ///
    /// The opposite of [`AssetKind::Image`] in the one way that matters: a
    /// still has no time in it, and a sequence carries a timeline of its own,
    /// so a clip of it is a window onto that timeline rather than a hold. Like
    /// a `group` it has no file of its own — its stills are `image` assets,
    /// named by id, and the files are theirs.
    ImageSequence,
    /// A web page the project carries under `pages/`, played as a moving
    /// picture with alpha — a title, a lower third, an animated graphic built
    /// from HTML and CSS rather than from the compositor's own shapes.
    ///
    /// **Authored, not generated.** A page is a document somebody wrote, like
    /// a recipe, so deleting one loses work; but it has no brief, no
    /// [`GenerationState`] and nothing that costs money, so it is none of
    /// [`AssetKind::is_generated`]'s kinds. The frames drawn from it are
    /// rebuildable cache, never `generated/` output.
    ///
    /// Like [`AssetKind::Image`] it has no length of its own: the clip says how
    /// long it is on screen, and the page is told that length when it is
    /// drawn. It may reference other files inside the project — a picture, a
    /// font — and never anything outside it, so it still survives `scp -r`.
    Html,
    /// A Veo prompt: video that does not exist until it is generated.
    GeneratedVideo,
    /// A Gemini image prompt: a still that does not exist until it is
    /// generated. Once it is, it is a picture like any imported one — no length
    /// of its own, held for as long as its clip says.
    GeneratedImage,
    /// An ElevenLabs TTS prompt: audio that does not exist until generated.
    GeneratedAudio,
    /// A synthesis recipe: audio computed from a document the project carries,
    /// rather than asked for in words. Free, offline, and the same bytes every
    /// time — see [`AssetKind::is_synthesized`].
    SynthAudio,
    /// Several clips that render as **one layer**: tracks of its own, placed on
    /// the timeline by a clip like any other picture — see [`crate::Group`].
    ///
    /// The fifth kind with no file behind it. What it carries is placements of
    /// other assets, never media, so there is nothing to import, hash or probe.
    Group,
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
            Self::GeneratedVideo | Self::GeneratedImage | Self::GeneratedAudio | Self::SynthAudio
        )
    }

    /// True when the brief is a sentence of natural language, which is also
    /// what makes realising it cost money and need a network.
    pub fn is_prompted(self) -> bool {
        matches!(
            self,
            Self::GeneratedVideo | Self::GeneratedImage | Self::GeneratedAudio
        )
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
                | Self::ImageSequence
                | Self::Html
                | Self::GeneratedVideo
                | Self::GeneratedImage
                | Self::Group
        )
    }

    /// True when this kind is a single picture with no time in it — an
    /// imported still, or a generated one.
    ///
    /// What a brief may name as a reference image, and what a render holds
    /// for a clip's length rather than plays: both questions are about the
    /// file being one frame, and a generated still is one frame exactly as an
    /// imported photograph is.
    pub fn is_still(self) -> bool {
        matches!(self, Self::Image | Self::GeneratedImage)
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
    /// project and nothing that could be missing after a copy. So does a
    /// `group`: it holds placements of other assets, and whatever files those
    /// need are theirs. And an `image_sequence`, for the same reason: it names
    /// its stills, and the files are theirs.
    pub fn is_file_backed(self) -> bool {
        !matches!(
            self,
            Self::Text | Self::Color | Self::Shape | Self::Icon | Self::Group | Self::ImageSequence
        )
    }

    /// True when the file behind this kind is *media* — something ffprobe
    /// reads, and so something with a `media` block a probe fills in.
    ///
    /// Every file-backed kind but [`AssetKind::Html`]. A page is a document,
    /// not a stream: there is no frame rate or duration to read off it, and
    /// asking ffprobe would be a process spent to fail. So it is never probed,
    /// never reported as unprobed, and imported without a probe.
    pub fn is_media(self) -> bool {
        self.is_file_backed() && self != Self::Html
    }
}

/// True when `path` names a page: it ends in `.html`, in any case.
///
/// The extension is what tells a reader of the document — and the browser
/// that draws it — that the file is HTML, so a page is held to it rather than
/// sniffed. `.htm` is not admitted: one spelling is one thing to look for.
pub(crate) fn is_page_path(path: &str) -> bool {
    path.len() > ".html".len()
        && path
            .get(path.len() - ".html".len()..)
            .is_some_and(|extension| extension.eq_ignore_ascii_case(".html"))
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
