//! Tracks and clips: where assets sit in time.
//!
//! How fast a clip runs against the timeline is `speed`, below. Where its
//! picture sits in *space* — `fit`, `crop`, `anchor`, `origin` — is
//! [`placement`].

use std::fmt;

use serde::{Deserialize, Serialize};

pub(crate) mod placement;

use crate::asset::AssetId;
use crate::grade::Grade;
use crate::keyframe::KeyframeTrack;
use crate::time::{Frames, Speed};

pub use placement::{Anchor, AnchorX, AnchorY, Crop, Fit, Origin, OriginX, OriginY};

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
            /// document, so a repeat is [`crate::Project::validate`]'s to find.
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
    /// Why this track is the way it is, for whoever reads the project next.
    ///
    /// **Never rendered.** Not a card, not a caption, not under any setting —
    /// that is an invariant of the format rather than a default someone could
    /// turn off, and it is worth saying out loud because the format does have
    /// precedent for text in a document reaching the screen: a `text` asset is
    /// drawn, and a sketch asset's `prompt` goes on a slug card. A note is
    /// categorically unlike both. Text meant to be seen is a `text` asset.
    ///
    /// This is not [`Track::name`]. A name is what the lane is *called*, and it
    /// belongs in a lane header; a note is why the lane is here — why the music
    /// sits under the narration rather than over it, why this is the one that
    /// gets ducked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
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
            note: None,
            clips: Vec::new(),
        }
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
    /// How fast the source runs against the timeline. [`Speed::NORMAL`] when
    /// absent, which is one source frame per timeline frame — what every clip
    /// did before there was a rate to choose.
    ///
    /// This is what breaks the one-to-one relationship `source_in` and
    /// `duration` used to have: a clip of 60 timeline frames at `2.0` consumes
    /// 120 frames of its source, and one at `0.5` consumes 30. `duration` is
    /// still a length of **timeline**, so speeding a clip up without also
    /// shortening it shows more of the source in the same slot — see
    /// [`Clip::retime`] for the other reading, the one an editor's 2× button
    /// performs.
    ///
    /// Applies to the clip's sound as well as its picture, at the same rate and
    /// with the pitch going with it. A clip has one speed: separating the two
    /// is a compositing-suite answer to a question nobody editing a video asks.
    #[serde(default, skip_serializing_if = "Speed::is_normal")]
    pub speed: Speed,
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
    /// Which edges `transform.position` is measured from. Absent means centred
    /// on both axes, which is what every layer did before the field existed.
    ///
    /// A **field and not an animatable property**, deliberately. An anchor says
    /// how a coordinate is to be read, and animating the interpretation of a
    /// coordinate — sliding a layer by changing what its number means — is a
    /// way to produce jumps nobody asked for. `transform.position` stays the
    /// animated part.
    #[serde(default, skip_serializing_if = "Anchor::is_default")]
    pub anchor: Anchor,
    /// Which point of the layer's own box `transform.scale`,
    /// `transform.rotation` and `transform.flip` turn about. Absent means its
    /// centre, which is what every layer did before the field existed.
    ///
    /// **This is what makes a bar that fills from its left edge one keyframe
    /// track.** Centre-anchored, that same bar is a scale *and* a position
    /// track cancelling half of it out — two properties carrying one
    /// intention, agreeing only while the scale happens to be linear in time.
    ///
    /// A **field and not an animatable property**, for the reason [`Anchor`]
    /// is one: it says how a transform is to be *read*, and animating it would
    /// move a layer by changing what its numbers mean.
    ///
    /// `transform.position` is applied after the pivot and is unaffected.
    #[serde(default, skip_serializing_if = "Origin::is_default")]
    pub origin: Origin,
    /// How this clip's pixels are graded before they are composited. Absent
    /// means untouched, which is what every clip did before the field existed.
    ///
    /// A field **and** five animatable properties, which is a combination no
    /// other clip property has. The field is the clip's baseline and a
    /// `grade.*` keyframe track takes that one property over, because both
    /// readings are ordinary: most of the time a shot has *a* look, said once
    /// and left alone; sometimes the look arrives over three seconds, and that
    /// is a ramp between two numbers like any other.
    ///
    /// Picture only. An audio clip has no pixels, and this says nothing about
    /// one.
    #[serde(default, skip_serializing_if = "Grade::is_neutral")]
    pub grade: Grade,
    /// How far this clip's pixels are softened before they are composited, as
    /// a fraction of the layer's own **height**. `0.0` — and an absent field —
    /// is the picture exactly as sharp as it arrived; more is blurrier.
    ///
    /// A fraction rather than a count of pixels, for the reason a `text` size
    /// is one: the same number has to mean the same softness whether the
    /// source is 1080p or 4K, and at whatever resolution the next render is
    /// delivered. It is resolved against the layer's own pixels, so
    /// `transform.scale` multiplies the apparent blur — a clip drawn at 200%
    /// looks twice as soft as the same number at 100%.
    ///
    /// **Beside [`Grade`] and deliberately not inside it.** A grade is the
    /// closed set of *colour* properties, and every one of them reads one
    /// pixel and writes one pixel. A blur reads a neighbourhood, so filing it
    /// under a struct that says colour would make that doc untrue about what
    /// it holds.
    ///
    /// Picture only. An audio clip has no pixels, and this says nothing about
    /// one.
    #[serde(default, skip_serializing_if = "is_sharp")]
    pub blur: f64,
    /// Why this clip is the way it is. Never rendered — see [`super::Track::note`].
    ///
    /// The commonest place a note belongs, because most decisions are decisions
    /// about a shot: why it runs this long, why it was moved out of the
    /// full-bleed plate, which corner the push-in must not drift toward. It
    /// travels with the clip, so retiming the cut cannot desynchronise it and
    /// deleting the clip takes the reasoning for a clip that no longer exists
    /// with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Properties animated over this clip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<KeyframeTrack>,
}

/// Whether a clip's [`Clip::blur`] is the neutral one, so it can be left out
/// of the document entirely.
///
/// Exactly zero, rather than anything at or below it: a negative number is not
/// a blur and the compositor treats it as none, but dropping it on save would
/// be this crate silently editing somebody's document on its way past.
fn is_sharp(blur: &f64) -> bool {
    *blur == 0.0
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
            speed: Speed::NORMAL,
            fit: Fit::default(),
            crop: None,
            anchor: Anchor::default(),
            origin: Origin::default(),
            grade: Grade::NEUTRAL,
            blur: 0.0,
            keyframes: Vec::new(),
            note: None,
        }
    }

    /// The frame just past this clip's last one — so a clip at frame 0 running
    /// 240 frames ends at 240, and the clip starting at 240 owns that frame.
    pub fn end(&self) -> Frames {
        self.start + self.duration
    }

    /// The frame of the **source** just past the last one this clip plays —
    /// where it starts in the media, plus how much of the media it gets
    /// through.
    ///
    /// The number that has to land inside the source's own length, and the one
    /// place a clip's playback rate would enter that arithmetic: a clip running
    /// at anything other than one source frame per timeline frame gets through
    /// its media faster or slower, and nothing above this line changes to say
    /// so. Today the two are the same thing, so this is `source_in` plus
    /// `duration`.
    pub fn source_end(&self) -> Frames {
        self.source_in + self.duration
    }

    /// True when the two clips share any frame. Clips that merely touch (one
    /// ending exactly where the next starts) do not overlap — with integer
    /// frames that is a fact rather than a tolerance.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end() && other.start < self.end()
    }

    /// How much of the source this clip consumes, in frames of the timeline's
    /// grid — its `duration` stretched by its speed.
    ///
    /// The number that decides whether a clip runs off the end of its footage,
    /// and the reason speed and an over-long trim are the same problem: both
    /// ask for more source than there is, and both are answered the same way.
    pub fn source_frames(&self) -> f64 {
        self.speed.source_frames(self.duration)
    }

    /// Sets the speed and rescales `duration`, so the clip goes on covering the
    /// same stretch of its source at the new rate.
    ///
    /// This is the *operation* — what picking 2× in an editor does, and what a
    /// GUI or an assistant asked to speed a clip up should call. Writing
    /// `speed` into the document instead is a different and equally legitimate
    /// edit: the clip keeps its slot on the timeline and shows more or less of
    /// its source in it. The format records only the rate, which is what lets
    /// both exist without a field having to mean two things.
    ///
    /// The new length is rounded to a whole timeline frame and never to zero: a
    /// clip covering no frame is not a clip, and deleting one because a speed
    /// was extreme is a worse answer than a clip one frame long. A speed
    /// [`Speed::is_usable`] rejects changes nothing at all — there is no length
    /// to rescale to — and neither does one applied to a clip whose current
    /// speed is already unusable.
    pub fn retime(&mut self, speed: Speed) {
        if !speed.is_usable() || !self.speed.is_usable() {
            return;
        }
        let source = self.source_frames();
        self.speed = speed;
        self.duration = Frames((speed.timeline_frames(source).round() as u64).max(1));
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
