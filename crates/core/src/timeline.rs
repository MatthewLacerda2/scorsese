//! Tracks and clips: where assets sit in time.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::asset::AssetId;
use crate::keyframe::KeyframeTrack;
use crate::time::Frames;

/// Identifies a track within one project.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrackId(String);

/// Identifies a clip within one project.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClipId(String);

macro_rules! string_id {
    ($ty:ty) => {
        impl $ty {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                // `pad` rather than `write_str`, so `{:<20}` in a table actually aligns.
                f.pad(&self.0)
            }
        }
    };
}

string_id!(TrackId);
string_id!(ClipId);

/// Whether a track carries picture or sound. Audio is first-class: an audio
/// track is a peer of a video track, not an attachment to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

/// A lane of clips. Video tracks composite in array order, first at the
/// bottom; audio tracks all mix together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub id: TrackId,
    pub kind: TrackKind,
    /// What a human calls this track. Cosmetic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(id: TrackId, kind: TrackKind) -> Self {
        Self {
            id,
            kind,
            name: None,
            clips: Vec::new(),
        }
    }
}

/// How a clip's source is fitted into the render's raster.
///
/// The raster is a render setting, and the project is not supposed to care what
/// it is. So this says what the author *meant* — the whole thing with bars
/// allowed, cover it and crop the overflow, or leave it alone — and lets the
/// render work out the pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// Scale to fit inside the raster, keeping proportions. What is left over
    /// is **transparent**, so the tracks below show through it.
    #[default]
    Fit,
    /// Scale to cover the raster, keeping proportions, cropping the overflow
    /// off the edges. What a background plate that must not have bars wants.
    Fill,
    /// No scaling at all: the source arrives at its own pixel size, resting
    /// centred, and `transform.position.*` offsets it from there.
    ///
    /// This is how something is placed at a size it was authored at. Scaling a
    /// 64×64 logo to fit makes its on-screen size a function of the render's
    /// resolution, so the factor that shrinks it back means nothing to a reader
    /// and stops meaning it the moment the render changes size.
    Native,
}

impl Fit {
    /// True for [`Fit::Fit`] — what a clip that says nothing means. Keeps the
    /// field out of documents that do not set it.
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Fit)
    }
}

/// One placement of an asset on a track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub id: ClipId,
    /// The asset this clip shows — **by id, never by path**.
    pub asset: AssetId,
    /// Where the clip begins on the timeline, in frames on the project's
    /// [`crate::Fps`] grid.
    pub start: Frames,
    /// How many frames of the timeline the clip occupies.
    pub duration: Frames,
    /// Offset into the source media where playback begins, in frames of the
    /// **timeline** grid — a source shot at another rate is conformed by
    /// [`crate::Fps::conform`]. Zero by default; meaningless for `text` and
    /// still images, which have no timeline of their own.
    #[serde(default)]
    pub source_in: Frames,
    /// How the source is fitted into the render's raster. [`Fit::Fit`] when
    /// absent, which is what every clip did before there was a choice.
    ///
    /// Picture only: an audio clip has no raster, and this says nothing about
    /// one.
    #[serde(default, skip_serializing_if = "Fit::is_default")]
    pub fit: Fit,
    /// Properties animated over this clip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<KeyframeTrack>,
}

impl Clip {
    pub fn new(id: ClipId, asset: AssetId, start: Frames, duration: Frames) -> Self {
        Self {
            id,
            asset,
            start,
            duration,
            source_in: Frames::ZERO,
            fit: Fit::default(),
            keyframes: Vec::new(),
        }
    }

    /// The frame just past this clip's last one — so a clip at frame 0 running
    /// 240 frames ends at 240, and the clip starting at 240 owns that frame.
    pub fn end(&self) -> Frames {
        self.start + self.duration
    }

    /// True when the two clips share any frame. Clips that merely touch (one
    /// ending exactly where the next starts) do not overlap — with integer
    /// frames that is a fact rather than a tolerance.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end() && other.start < self.end()
    }
}
