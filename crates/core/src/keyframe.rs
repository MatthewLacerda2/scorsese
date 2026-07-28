//! Generic keyframe animation.
//!
//! A keyframe track is `(property_path, [(t, value, easing)])` over any
//! numeric property. **The mechanism must not know which property it
//! animates** — that is the generality rule: core defines property *types*,
//! never property *values*.
//!
//! So [`PropertyPath`] is a string, not an enum of known properties. That is
//! a deliberate trade (a typo in a property name is not caught here) and the
//! reason it holds: position, scale, opacity, and volume are all just numbers
//! over time, and the next animatable property should cost nothing to add.
//! Resolving a path against a clip belongs to the compositor, not here.

use serde::{Deserialize, Serialize};

use crate::time::Frames;

/// Names the property a keyframe track animates, e.g. `opacity`,
/// `transform.position.x`, `volume`. Dotted segments, no empty segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PropertyPath(String);

impl PropertyPath {
    /// Wraps a string as a property path. Infallible, and it does not check
    /// that anything animates the property — see [`PropertyPath::is_well_formed`].
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// The whole dotted path as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Segments of the dotted path, in order.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('.')
    }

    /// True when the path is syntactically usable: non-empty, and no empty
    /// segment (`a..b`, `.a`, `a.`). Says nothing about whether the property
    /// exists — by design.
    pub fn is_well_formed(&self) -> bool {
        !self.0.is_empty() && !self.segments().any(str::is_empty)
    }
}

impl std::fmt::Display for PropertyPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `pad` rather than `write_str`, so `{:<20}` in a table actually aligns.
        f.pad(&self.0)
    }
}

/// How a value approaches the next keyframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    /// Constant rate throughout, and what a keyframe that says nothing means.
    #[default]
    Linear,
    /// Starts slow and accelerates into the next keyframe.
    EaseIn,
    /// Starts at full rate and settles as it arrives.
    EaseOut,
    /// Slow at both ends, quickest in the middle — the one that reads as
    /// deliberate rather than mechanical.
    EaseInOut,
    /// Holds this value until the next keyframe, then jumps.
    Hold,
}

impl Easing {
    /// Reshapes linear progress through a segment, `0.0..=1.0`, into eased
    /// progress.
    ///
    /// The classic quadratic family, which is what "ease in" means to anyone
    /// who has used an editor. Nothing here is tuneable yet on purpose: a
    /// custom bezier is a property of a keyframe someone can ask for, and
    /// inventing the knob before the need is how a format grows fields nobody
    /// sets.
    pub fn apply(self, progress: f64) -> f64 {
        let p = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => p,
            Self::EaseIn => p * p,
            Self::EaseOut => 1.0 - (1.0 - p) * (1.0 - p),
            // Smoothstep: symmetric, and flat at both ends.
            Self::EaseInOut => p * p * (3.0 - 2.0 * p),
            // The value does not travel at all; it jumps at the next keyframe.
            Self::Hold => 0.0,
        }
    }
}

/// One `(t, value, easing)` point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    /// Time of this keyframe, in frames **relative to the start of its clip**
    /// — so moving a clip along the timeline never rewrites its keyframes.
    ///
    /// Keyframes are *control points*; the value travels continuously between
    /// them. Putting the control points on the frame grid does not make a
    /// ramp steppy, it only quantises where the ramp's corners sit, which is
    /// why even an audio fade wants frames and not something finer.
    pub t: Frames,
    /// What the property reads at this instant. Its meaning is the property's,
    /// never this crate's — `1.0` is fully opaque to opacity and as-recorded
    /// to volume, and core knows neither. Must be finite.
    pub value: f64,
    /// How the value travels from here to the next keyframe.
    #[serde(default)]
    pub easing: Easing,
}

/// A property animated over the life of a clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyframeTrack {
    /// Which property these points animate. A path nothing animates is
    /// ignored and warned about, never an error — a project authored against
    /// a newer scorsese still has to render on an older one.
    pub property: PropertyPath,
    /// Which tool wrote this track, if a tool did. Absent means a person or an
    /// agent placed these points by hand, which is what every keyframe meant
    /// before this field existed.
    ///
    /// It buys exactly one thing, and it is not provenance for its own sake: a
    /// tool that generates keyframes can find its own previous output and
    /// replace it, without touching anything anyone set by hand. Re-running
    /// auto-ducking should redo the ducking and leave your fades alone.
    ///
    /// Nothing else reads it. It does not reach [`KeyframeTrack::value_at`]
    /// and it changes no pixel and no sample.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Ordered by `t`, ascending, with no duplicated times.
    pub keyframes: Vec<Keyframe>,
}

impl KeyframeTrack {
    /// Takes the points as given, unsigned. Sorting is not imposed here —
    /// validation reports an out-of-order track rather than silently rewriting
    /// what an author wrote.
    pub fn new(property: PropertyPath, keyframes: Vec<Keyframe>) -> Self {
        Self {
            property,
            by: None,
            keyframes,
        }
    }

    /// The same track, signed by the tool that generated it.
    ///
    /// A tool signs what it writes so it can recognise its own work later.
    /// Everything unsigned belongs to whoever wrote it by hand and is never a
    /// tool's to replace.
    pub fn generated_by(self, tool: impl Into<String>) -> Self {
        Self {
            by: Some(tool.into()),
            ..self
        }
    }

    /// True when `tool` wrote this track, and so may replace it.
    pub fn is_generated_by(&self, tool: &str) -> bool {
        self.by.as_deref() == Some(tool)
    }

    /// True when times ascend strictly. Checked by validation; the evaluator
    /// this unblocks is entitled to assume it.
    pub fn is_sorted(&self) -> bool {
        self.keyframes.windows(2).all(|pair| pair[0].t < pair[1].t)
    }

    /// The value of this property at `t`, in frames from the start of the clip.
    ///
    /// `None` only when the track holds no keyframes — a property with nothing
    /// to say leaves whatever reads it on its own default, rather than
    /// asserting a zero nobody wrote.
    ///
    /// Outside the keyframed span the value **holds**: before the first
    /// keyframe it is the first value, after the last it is the last. The
    /// alternative — extrapolating — would have a two-keyframe fade keep
    /// darkening past the end of the clip, which nobody means by two
    /// keyframes.
    ///
    /// This is the whole of the mechanism, and it does not know which property
    /// it is animating. That is the generality rule holding: the same function
    /// evaluates an opacity ramp, a position move, and an audio fade.
    pub fn value_at(&self, t: Frames) -> Option<f64> {
        let first = self.keyframes.first()?;
        let last = self.keyframes.last()?;
        if t <= first.t {
            return Some(first.value);
        }
        if t >= last.t {
            return Some(last.value);
        }
        let pair = self
            .keyframes
            .windows(2)
            .find(|pair| pair[0].t <= t && t < pair[1].t)?;
        let (from, to) = (&pair[0], &pair[1]);
        let span = to.t.get().saturating_sub(from.t.get());
        if span == 0 {
            return Some(from.value);
        }
        let progress = (t.get() - from.t.get()) as f64 / span as f64;
        Some(from.value + (to.value - from.value) * from.easing.apply(progress))
    }
}
