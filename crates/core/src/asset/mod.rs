//! The assets table: what media a project knows about.
//!
//! Assets are entities; clips are references. A clip names an asset by
//! [`AssetId`] and never by path, so moving or regenerating a file is one
//! edit in one place.

pub mod kind;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::path::ProjectPath;
use crate::text::TextStyle;
use crate::time::Fps;

pub use kind::{AssetKind, GenerationState};

/// Identifies an asset within one project. Unique across the assets table.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(String);

impl AssetId {
    /// Wraps a string as an id. Uniqueness is a property of the table, not of
    /// the id, so it is [`crate::validate`] that catches a repeat.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id as written in `project.json`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `pad` rather than `write_str`, so `{:<20}` in a table actually aligns.
        f.pad(&self.0)
    }
}

/// One row of the assets table.
///
/// Which fields are required depends on `kind` — an imported video needs a
/// `path`, a Veo prompt needs a `prompt` and a `state`, a synthesis asset
/// needs a `recipe` and a `state`, a text asset needs `text`. Those rules are
/// checked by [`crate::validate`] so that one pass reports every problem at
/// once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    /// Unique within the project, and the only handle a clip has on this
    /// asset — so renaming one here is a rename everywhere it is used.
    pub id: AssetId,
    /// Decides which of the fields below are required, and which track the
    /// asset may sit on: [`AssetKind::is_visual`] against the track's kind.
    pub kind: AssetKind,
    /// Path to the media, relative to the project root. Imported media lives
    /// under `assets/`, generated and synthesised output under `generated/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<ProjectPath>,
    /// Lowercase hex SHA-256 of the file at `path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// What ffprobe found. Absent until the asset has been probed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<MediaMetadata>,
    /// The generation prompt. Required for the prompt-backed kinds, and
    /// refused on every other — including `synth_audio`, whose brief is a
    /// document rather than a sentence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Path to the synthesis recipe, relative to the project root — by
    /// convention under `recipes/`. Required for the synthesised kinds.
    ///
    /// A file of its own rather than inline JSON because a recipe is long: a
    /// song is tracks, patterns and an arrangement, and inlining one would
    /// bury the timeline under note lists in the document an agent reads to
    /// understand the edit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<ProjectPath>,
    /// Lifecycle state. Required for every generated kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<GenerationState>,
    /// Inline content for the `text` kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// How that content looks: font, size, colour, alignment. Only the `text`
    /// kind has one, and an absent style means [`TextStyle::default`] —
    /// white, centred, sans — rather than nothing to draw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<TextStyle>,
}

impl Asset {
    /// A file-backed asset that already exists on disk.
    pub fn imported(id: AssetId, kind: AssetKind, path: ProjectPath) -> Self {
        Self {
            path: Some(path),
            ..Self::bare(id, kind)
        }
    }

    /// A prompt-backed asset in its initial `sketch` state.
    pub fn sketch(id: AssetId, kind: AssetKind, prompt: impl Into<String>) -> Self {
        Self {
            prompt: Some(prompt.into()),
            state: Some(GenerationState::Sketch),
            ..Self::bare(id, kind)
        }
    }

    /// A synthesis asset in its initial `sketch` state, pointing at the recipe
    /// that describes it. The sibling of [`Asset::sketch`] for the kinds whose
    /// brief is a document rather than a sentence.
    pub fn synth(id: AssetId, kind: AssetKind, recipe: ProjectPath) -> Self {
        Self {
            recipe: Some(recipe),
            state: Some(GenerationState::Sketch),
            ..Self::bare(id, kind)
        }
    }

    fn bare(id: AssetId, kind: AssetKind) -> Self {
        Self {
            id,
            kind,
            path: None,
            sha256: None,
            media: None,
            prompt: None,
            recipe: None,
            state: None,
            text: None,
            style: None,
        }
    }

    /// A text asset carrying its content inline, styled by whatever the
    /// document says — or by [`TextStyle::default`] when it says nothing.
    pub fn text(id: AssetId, content: impl Into<String>) -> Self {
        Self {
            text: Some(content.into()),
            ..Self::bare(id, AssetKind::Text)
        }
    }

    /// The style this asset's text is drawn in, defaults included. Not the
    /// stored field: an absent style is every default, not an absence, so a
    /// caller never has to decide what a missing font means.
    pub fn text_style(&self) -> TextStyle {
        self.style.clone().unwrap_or_default()
    }

    /// True when GO would realise this asset. Not the same as "would spend
    /// money on it": a synthesised asset needs generation and costs nothing.
    pub fn needs_generation(&self) -> bool {
        self.state.is_some_and(GenerationState::needs_generation)
    }

    /// True when this asset has something to render right now.
    ///
    /// A sketch or stale asset does not: it renders as a slug card instead,
    /// which is what makes a full preview cut cost nothing.
    pub fn has_renderable_media(&self) -> bool {
        match self.state {
            Some(state) => state.has_media() && self.path.is_some(),
            None => self.path.is_some() || self.kind == AssetKind::Text,
        }
    }
}

/// What ffprobe reported about a media file. Typed on purpose — a raw probe
/// blob would make every consumer re-parse ffprobe's output shape.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaMetadata {
    /// The source's own length in wall-clock seconds, which is what a probe
    /// measures. Not a timeline value: it is on the source's grid, not the
    /// project's, until a clip conforms it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    /// Picture width in pixels. Its absence is how import tells that a file
    /// claiming to be video has no video stream in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Picture height in pixels. With `width`, the shape a clip's
    /// [`crate::Fit`] has to reconcile against the render's raster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// The source's framerate, kept rational because ffprobe reports one and
    /// because conforming a 30000/1001 source off a rounded 29.97 is exactly
    /// the drift the timeline grid exists to avoid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<Fps>,
    /// How many audio channels the file carries. This is what says whether a
    /// clip on a *video* track has sound of its own to mix; absent means
    /// nobody has looked, not that the file is silent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_channels: Option<u16>,
    /// The source's sample rate in Hz. Informational here — the mix works in
    /// one rate and resamples everything on the way in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
}
