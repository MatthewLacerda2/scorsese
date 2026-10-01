//! Laying a shape's line: all of it or the trimmed part, solid or dashed.
//!
//! tiny-skia strokes a path and dashes one, and has no notion of drawing only
//! part of it — so a trim is built here, by measuring the outline
//! ([`super::measure`]) and handing the stroker only the stretch that is kept.
//!
//! **An untrimmed line is the path exactly as it always was.** Nothing is
//! measured or rebuilt unless a trim asks for it — or an arrow's head needs the
//! line stopped short of its tip (`Pullback`) — so a box that never mentions a
//! trim draws the same pixels it drew before trims existed, and a closed
//! outline keeps the join where it meets itself, which a rebuilt open copy of
//! it would not have.

use tiny_skia::{Path, StrokeDash};

use crate::frame::Frame;
use crate::paint;

use super::Border;
use super::measure::Measured;
use super::trace::{Dash, Stroking};

/// What reached the frame, which is what an arrow's heads are placed by.
pub(super) enum Laid {
    /// Nothing: the trim kept none of the line.
    Nothing,
    /// The whole outline, exactly as its path runs.
    Whole,
    /// Part of it, and the measured outline that part was cut from. Also what
    /// a line pulled back from its ends reports, even when the stretch between
    /// the pulled-back ends was empty and nothing was stroked: the trim kept
    /// something, so the heads on its ends are still drawn.
    Part(Measured),
}

/// How far short of each end of the kept stretch the stroke stops, in pixels.
///
/// An arrow's head covers the last stretch of the line itself, so the line
/// stops inside the head rather than under its tip (#608). Zero for anything
/// without heads, which keeps the untrimmed fast path.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Pullback {
    /// Back from the kept stretch's start, toward its end.
    pub(super) start: f32,
    /// Back from the kept stretch's end, toward its start.
    pub(super) end: f32,
}

/// Strokes the part of `path` that `stroking` keeps, stopping `pullback`
/// short of either end of it.
///
/// **Pulled back by distance along the outline**, not along a straight line, so
/// a bowed arrow's line follows its own curve all the way into the head.
pub(super) fn lay(
    frame: &mut Frame,
    path: &Path,
    border: Border,
    stroking: &Stroking,
    pullback: Pullback,
) -> Laid {
    let Some((start, end)) = stroking.span() else {
        return Laid::Nothing;
    };
    if start <= 0.0 && end >= 1.0 && pullback == Pullback::default() {
        stroke(frame, path, border, stroking.dash.as_ref(), 0.0);
        return Laid::Whole;
    }
    let Some(measured) = Measured::of_path(path) else {
        return Laid::Nothing;
    };
    let length = measured.length();
    let from = measured.distance(start) + pullback.start.max(0.0);
    let to = measured.distance(end) - pullback.end.max(0.0);
    // Pulled back past one another, the kept stretch is all head and no line:
    // nothing to stroke, but the trim still kept something to put heads on.
    if to > from
        && let Some(part) = measured.between(from / length, to / length)
    {
        stroke(frame, &part, border, stroking.dash.as_ref(), from);
    }
    Laid::Part(measured)
}

/// Strokes `path`, which begins `from` pixels along the whole outline.
///
/// **The dashes belong to the outline, not to the part of it drawn.** A trimmed
/// line starts part way along, and the pattern is shifted by exactly that much
/// — so a dashed arrow drawing itself on lays each dash down where it will stay,
/// rather than sliding the whole pattern along as the line grows.
fn stroke(frame: &mut Frame, path: &Path, border: Border, dash: Option<&Dash>, from: f32) {
    let dash = dash.and_then(|dash| StrokeDash::new(dash.pattern()?, from - dash.offset));
    paint::stroke(frame, path, border.color, border.width, dash);
}
