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
            /// Wraps a string as an id. Uniqueness is a property of the
            /// document, so a repeat is [`crate::validate`]'s to find.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The id as written in `project.json`.
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
    /// Carries picture. Where a track sits in the list is what "on top"
    /// means; the first is at the bottom.
    Video,
    /// Carries sound. Order means nothing here — everything playing at once
    /// is summed, and addition does not care what came first.
    Audio,
}

/// A lane of clips. Video tracks composite in array order, first at the
/// bottom; audio tracks all mix together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    /// Unique within the project. What validation names when it reports an
    /// overlap on this track.
    pub id: TrackId,
    /// Decides which assets may sit here: a visual asset on a video track, an
    /// audible one on an audio track.
    pub kind: TrackKind,
    /// What a human calls this track. Cosmetic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// What is on this track. Clips may touch but never overlap, and a hole
    /// between them contributes nothing rather than rendering black.
    #[serde(default)]
    pub clips: Vec<Clip>,
}

impl Track {
    /// An empty, unnamed track. Clips are pushed onto it afterwards.
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

/// A rectangle of the source, in fractions of it.
///
/// **The asset is never touched.** Cropping by cutting the file down is the one
/// place the format's premise — a document describing an edit over unmodified
/// assets — currently has to be broken to do ordinary work, and a project that
/// does it stops being a description of an edit and becomes a description of a
/// result: the original pixels are gone, nothing records that a sidebar was
/// removed or from where, and the recorded `sha256` describes a file no camera
/// and no capture ever produced.
///
/// **Fractions of the source, not source pixels**, and the reasoning matters
/// more than the choice. A fraction survives the asset being *replaced* by a
/// higher-resolution capture of the same thing — re-shoot the screenshot at 4K
/// and the crop still means the same region, where in pixels it would silently
/// mean a different one. That is exactly the change-your-mind-later case this
/// exists for, so the unit must not be the one that breaks it. A fraction also
/// validates from the document alone, where a pixel rectangle would need the
/// source's dimensions, which are only recorded if something probed the asset.
///
/// This is a different question from `transform.position`, which is a fraction
/// of the **output** raster. A crop is against the **source** raster, and the
/// two do not have to answer the same way.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crop {
    /// Left edge, as a fraction of the source's width.
    pub x: f64,
    /// Top edge, as a fraction of the source's height.
    pub y: f64,
    /// How much of the source's width is kept.
    pub width: f64,
    /// How much of the source's height is kept.
    pub height: f64,
}

impl Default for Crop {
    /// The whole source — what an absent `crop` means.
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }
}

impl Crop {
    /// Each edge, paired with the name `project.json` spells it — so a message
    /// about one names the field the author wrote rather than a description of
    /// it.
    pub fn edges(&self) -> [(&'static str, f64); 4] {
        [
            ("x", self.x),
            ("y", self.y),
            ("width", self.width),
            ("height", self.height),
        ]
    }

    /// True when the rectangle is inside the source and encloses some of it.
    ///
    /// Checkable from the document alone, which is the point of fractions: a
    /// pixel rectangle would need the source's dimensions, and those are only
    /// recorded if something probed the asset.
    pub fn is_within_source(&self) -> bool {
        self.edges().iter().all(|&(_, value)| value.is_finite())
            && self.width > 0.0
            && self.height > 0.0
            && self.x >= 0.0
            && self.y >= 0.0
            && self.x + self.width <= 1.0
            && self.y + self.height <= 1.0
    }
}

/// One placement of an asset on a track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    /// Unique across the whole project, not just this track — validation and
    /// every render report identify a clip by it alone.
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
    /// Which part of the source is shown. Absent means all of it, the way an
    /// absent [`Fit`] means the ordinary case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Crop>,
    /// Properties animated over this clip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<KeyframeTrack>,
}

impl Clip {
    /// A clip at its defaults: from the head of the source, fitted, and
    /// animating nothing.
    pub fn new(id: ClipId, asset: AssetId, start: Frames, duration: Frames) -> Self {
        Self {
            id,
            asset,
            start,
            duration,
            source_in: Frames::ZERO,
            fit: Fit::default(),
            crop: None,
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

    /// Replaces the tracks `tool` previously wrote for a property with
    /// `replacement`, leaving every other track alone.
    ///
    /// One call rather than "remove mine, then add mine", because those two
    /// steps have a moment between them where the clip is animated by nothing,
    /// and a run that fails there leaves an edit quietly missing a ramp.
    ///
    /// Passing an empty `replacement` is how a tool withdraws its work.
    /// Anything unsigned, or signed by someone else, is untouched whatever it
    /// animates — that is the whole point of the field, and the reason this
    /// takes a tool name rather than a property alone.
    pub fn replace_generated(&mut self, tool: &str, replacement: Vec<KeyframeTrack>) {
        self.keyframes.retain(|track| !track.is_generated_by(tool));
        self.keyframes.extend(replacement);
    }

    /// Every track this clip animates that no tool signed — what a person or
    /// an agent wrote by hand, and what nothing generated may overwrite.
    pub fn hand_written(&self) -> impl Iterator<Item = &KeyframeTrack> {
        self.keyframes.iter().filter(|track| track.by.is_none())
    }
}
