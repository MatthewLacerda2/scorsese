//! The assets table: what media a project knows about.
//!
//! Assets are entities; clips are references. A clip names an asset by
//! [`AssetId`] and never by path, so moving or regenerating a file is one
//! edit in one place.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::path::ProjectPath;

/// Identifies an asset within one project. Unique across the assets table.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(String);

impl AssetId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

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

/// What kind of media an asset is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Video,
    Image,
    Audio,
    Text,
    /// A Veo prompt: video that does not exist until it is generated.
    GeneratedVideo,
    /// An ElevenLabs TTS prompt: audio that does not exist until generated.
    GeneratedAudio,
}

impl AssetKind {
    /// True for the prompt-backed kinds, which carry a prompt and a
    /// [`GenerationState`] and cost money to realise.
    pub fn is_generated(self) -> bool {
        matches!(self, Self::GeneratedVideo | Self::GeneratedAudio)
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
        matches!(self, Self::Audio | Self::GeneratedAudio)
    }

    /// True when a file on disk is what this kind ultimately refers to.
    /// `text` is the exception: it carries its content inline.
    pub fn is_file_backed(self) -> bool {
        !matches!(self, Self::Text)
    }
}

/// Where a prompt-backed asset sits in the sketch lifecycle.
///
/// `sketch → queued → generated`, and back to `stale` when the prompt is
/// edited after generation. Sketch and stale clips render as slug cards, so a
/// full preview cut costs nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationState {
    Sketch,
    Queued,
    Generated,
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

/// One row of the assets table.
///
/// Which fields are required depends on `kind` — an imported video needs a
/// `path`, a Veo prompt needs a `prompt` and a `state`, a text asset needs
/// `text`. Those rules are checked by [`crate::validate`] so that one pass
/// reports every problem at once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub id: AssetId,
    pub kind: AssetKind,
    /// Path to the media, relative to the project root. Imported media lives
    /// under `assets/`, provider output under `generated/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<ProjectPath>,
    /// Lowercase hex SHA-256 of the file at `path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// What ffprobe found. Absent until the asset has been probed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<MediaMetadata>,
    /// The generation prompt. Required for the `generated_*` kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Lifecycle state. Required for the `generated_*` kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<GenerationState>,
    /// Inline content for the `text` kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
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

    fn bare(id: AssetId, kind: AssetKind) -> Self {
        Self {
            id,
            kind,
            path: None,
            sha256: None,
            media: None,
            prompt: None,
            state: None,
            text: None,
        }
    }

    /// True when GO would spend money on this asset.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_channels: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
}
