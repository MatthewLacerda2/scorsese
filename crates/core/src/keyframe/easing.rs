//! How a value travels from one keyframe to the next.
//!
//! **Eased progress may leave `0..=1`, and that is the point of half of these.**
//! A title that lands by passing its mark and settling back is the cheapest
//! thing that makes motion feel alive, and it is *made of* the value going past
//! the keyframe that was written. So nothing here clamps what it returns — only
//! what it is given — and a property that cannot go past its ends clamps at the
//! property, where the meaning of the number is known: opacity in the
//! compositor, volume in the mixer. Scale, position and rotation are left to
//! overshoot, because for them overshooting is the animation.

use serde::{Deserialize, Serialize};

use super::bezier;

/// How a value approaches the next keyframe.
///
/// Written in a document as a bare word — `"ease_out"`, `"back_out"` — except
/// [`Easing::CubicBezier`], which carries its four numbers:
/// `{ "cubic_bezier": [0.25, 0.1, 0.25, 1] }`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
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
    /// Pulls back a little the other way first — about a tenth of the move —
    /// then accelerates into the next keyframe. A wind-up.
    BackIn,
    /// Arrives fast, passes the next keyframe by about a tenth of the move and
    /// settles back onto it. The "pop" of a title landing.
    BackOut,
    /// Both: winds up, crosses quickly, passes the mark and settles back.
    BackInOut,
    /// Springs onto the next keyframe: past it by about a sixth of the move,
    /// a slight swing back under, then still.
    Spring,
    /// The CSS `cubic-bezier(x1, y1, x2, y2)`, for when no preset is right:
    /// `[0.25, 0.1, 0.25, 1]` is CSS's `ease`. `x1` and `x2` must lie in
    /// `0..=1`, or the curve is not a function of time; `y1` and `y2` may be
    /// anything, and outside `0..=1` is how the curve overshoots.
    CubicBezier([f64; 4]),
}

/// How far the back family pulls past its ends: Robert Penner's constant, the
/// one CSS libraries, Keynote and Filmora's "back" presets all descend from.
/// It is chosen for what it does — ten percent of the move — rather than
/// derived from anything.
const BACK: f64 = 1.701_58;

/// The in-and-out curve does each half in half the time, so it pulls harder to
/// reach the same ten percent. Penner's again.
const BACK_BOTH: f64 = BACK * 1.525;

/// The spring's swing: one and a half turns across the segment, which is one
/// pass over the mark and one back under it before it is still.
const SPRING_TURNS: f64 = 3.0 * std::f64::consts::PI;

/// How fast the spring's swing dies away.
const SPRING_DAMPING: f64 = 5.0;

impl Easing {
    /// Reshapes linear progress through a segment, `0.0..=1.0`, into eased
    /// progress.
    ///
    /// The **input** is clamped; the **output** is not. Every curve starts at
    /// `0.0` and arrives at exactly `1.0` (except [`Easing::Hold`], which never
    /// leaves), but the back family, [`Easing::Spring`] and a cubic bezier with
    /// a `y` outside `0..=1` go past either end on the way — see the module doc
    /// for where that gets clamped and why it is not here.
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
            Self::BackIn => back(p, BACK),
            Self::BackOut => 1.0 - back(1.0 - p, BACK),
            Self::BackInOut if p < 0.5 => back(2.0 * p, BACK_BOTH) / 2.0,
            Self::BackInOut => 1.0 - back(2.0 - 2.0 * p, BACK_BOTH) / 2.0,
            // A damped swing whose envelope also closes linearly, so it lands
            // on the mark exactly at the keyframe rather than ringing on past
            // it into a value nobody wrote.
            Self::Spring => {
                1.0 - (SPRING_TURNS * p).cos() * (-SPRING_DAMPING * p).exp() * (1.0 - p)
            }
            Self::CubicBezier(points) => bezier::ease(points, p),
        }
    }

    /// False only for a [`Easing::CubicBezier`] that is not a curve over time:
    /// an `x` outside `0..=1` makes time run backwards somewhere along it, and
    /// a number that is not finite makes it no curve at all. Every preset is
    /// well formed.
    pub fn is_well_formed(&self) -> bool {
        match self {
            Self::CubicBezier([x1, y1, x2, y2]) => {
                (0.0..=1.0).contains(x1)
                    && (0.0..=1.0).contains(x2)
                    && y1.is_finite()
                    && y2.is_finite()
            }
            _ => true,
        }
    }
}

/// The back curve easing in: `p² · ((c + 1)·p − c)`, rearranged so both ends
/// are exact in floating point — `back(1)` is `1 · (c · 0 + 1)`, which is one
/// whatever `c` is, where the textbook order sums to one only nearly.
fn back(p: f64, pull: f64) -> f64 {
    p * p * (pull * (p - 1.0) + p)
}
