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
