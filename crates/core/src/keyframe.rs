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
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

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
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// Holds this value until the next keyframe, then jumps.
    Hold,
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
    pub value: f64,
    /// How the value travels from here to the next keyframe.
    #[serde(default)]
    pub easing: Easing,
}

/// A property animated over the life of a clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyframeTrack {
    pub property: PropertyPath,
    /// Ordered by `t`, ascending, with no duplicated times.
    pub keyframes: Vec<Keyframe>,
}

impl KeyframeTrack {
    pub fn new(property: PropertyPath, keyframes: Vec<Keyframe>) -> Self {
        Self {
            property,
            keyframes,
        }
    }

    /// True when times ascend strictly. Checked by validation; the evaluator
    /// this unblocks is entitled to assume it.
    pub fn is_sorted(&self) -> bool {
        self.keyframes.windows(2).all(|pair| pair[0].t < pair[1].t)
    }
}
