//! Laying a shape's line: all of it or the trimmed part, solid or dashed.
//!
//! tiny-skia strokes a path and dashes one, and has no notion of drawing only
//! part of it — so a trim is built here, by measuring the outline
//! ([`super::measure`]) and handing the stroker only the stretch that is kept.
//!
//! **An untrimmed line is the path exactly as it always was.** Nothing is
//! measured or rebuilt unless a trim asks for it, so a shape that never
//! mentions one draws the same pixels it drew before trims existed — and a
//! closed outline keeps the join where it meets itself, which a rebuilt open
//! copy of it would not have.

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
    /// Part of it, and the measured outline that part was cut from.
    Part(Measured),
}

/// Strokes the part of `path` that `stroking` keeps.
pub(super) fn lay(frame: &mut Frame, path: &Path, border: Border, stroking: &Stroking) -> Laid {
    let Some((start, end)) = stroking.span() else {
        return Laid::Nothing;
    };
    if start <= 0.0 && end >= 1.0 {
        stroke(frame, path, border, stroking.dash.as_ref(), 0.0);
        return Laid::Whole;
    }
    let Some(measured) = Measured::of_path(path) else {
        return Laid::Nothing;
    };
    let Some(part) = measured.between(start, end) else {
        return Laid::Nothing;
    };
    stroke(
        frame,
        &part,
        border,
        stroking.dash.as_ref(),
        measured.distance(start),
    );
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
