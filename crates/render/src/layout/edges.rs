//! Where a clip's content leaves the frame, and where a title's words were
//! cut to fit it (#1096).
//!
//! Both are facts the layout already holds — a rectangle in fractions of the
//! frame, and a block the compositor set — and both used to be found only by
//! rendering a still and looking at it. Here they are numbers, worked out once,
//! and worded once: `describe at` and the checkup print the same sentence.
//!
//! **Reported, never refused.** A clip half outside the frame is very often
//! meant — a slide-in, a pan across a wide picture — so this says where, and
//! the reader decides.

use std::fmt;

use scorsese_compositor::text::Overset;

use super::Region;

/// Less past an edge than this is rounding, not placement: a tenth of a
/// percent of the frame, two pixels across a 1080p one. The page check (#813)
/// ignores a pixel for the same reason.
const SLACK: f64 = 0.001;

/// How far a clip's content runs past each edge of the frame, as a fraction
/// of the frame's width (left, right) or height (top, bottom).
///
/// A plain value, so a caller other than the two printers — a re-fit after the
/// frame changes shape (#1278) — can weigh it for itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overhang {
    /// Past the left edge.
    pub left: f64,
    /// Past the right edge.
    pub right: f64,
    /// Past the top edge.
    pub top: f64,
    /// Past the bottom edge.
    pub bottom: f64,
}

impl Overhang {
    /// How far `region` leaves the frame, when it leaves it at all.
    ///
    /// Two arrangements are not a clip leaving the frame, and say nothing:
    ///
    /// - **Wholly outside.** Nothing of it is seen — it has exited, or is yet
    ///   to come in — which is the page check's reading too.
    /// - **Past both edges of one axis.** Content wider than the frame is
    ///   cropped by it, the way a `cover` picture or a slow zoom is, and is
    ///   not hanging off one side. Only the other axis is weighed.
    pub fn of(region: Region) -> Option<Self> {
        let outside = region.right() <= 0.0
            || region.left >= 1.0
            || region.bottom() <= 0.0
            || region.top >= 1.0;
        if outside {
            return None;
        }
        let (left, right) = one_side(-region.left, region.right() - 1.0);
        let (top, bottom) = one_side(-region.top, region.bottom() - 1.0);
        let overhang = Self {
            left,
            right,
            top,
            bottom,
        };
        (overhang.sides().next().is_some()).then_some(overhang)
    }

    /// Each edge it runs past, named as a sentence names it, with how far and
    /// which of the frame's dimensions that is a fraction of.
    fn sides(&self) -> impl Iterator<Item = (&'static str, f64, &'static str)> {
        [
            ("left", self.left, "width"),
            ("right", self.right, "width"),
            ("top", self.top, "height"),
            ("foot", self.bottom, "height"),
        ]
        .into_iter()
        .filter(|(_, by, _)| *by > 0.0)
    }
}

/// How far past each end of one axis, keeping only an overhang on one side
/// alone and past the slack.
fn one_side(before: f64, after: f64) -> (f64, f64) {
    let (before, after) = (before.max(0.0), after.max(0.0));
    if before > SLACK && after > SLACK {
        return (0.0, 0.0);
    }
    let kept = |by: f64| if by > SLACK { by } else { 0.0 };
    (kept(before), kept(after))
}

/// `runs off the right of the frame by 7.0% of its width` — the page check's
/// wording (#813), in the unit a clip is placed in.
impl fmt::Display for Overhang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "runs off the ")?;
        for (index, (edge, by, of)) in self.sides().enumerate() {
            let and = if index == 0 { "" } else { " and the " };
            write!(f, "{and}{edge}")?;
            if index == 0 {
                write!(f, " of the frame")?;
            }
            write!(f, " by {:.1}% of its {of}", by * 100.0)?;
        }
        Ok(())
    }
}

/// `is cut off: 6 lines set, 4 shown` — a block of words the band dropped the
/// tail of, as both printers say it.
pub(crate) fn cut_off(overset: &Overset) -> String {
    format!(
        "is cut off: {} lines set, {} shown",
        overset.set, overset.shown
    )
}
