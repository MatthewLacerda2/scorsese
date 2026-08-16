//! Drawing a line between two points, and the head that says which way it goes.
//!
//! No anchor reaches here: an arrow's two endpoints already say where they are,
//! so there is nothing to measure from an edge. That is the whole difference
//! between this module and `closed`.
//!
//! **The head is aimed along the curve's tangent, not along the straight line
//! between the ends.** On a bowed arrow those differ by a visible angle, and a
//! head aimed at the wrong one is the classic bug in this feature — it looks
//! like a mistake in the diagram rather than in the renderer, and no test that
//! is not a picture will object to it.

use tiny_skia::PathBuilder;

use scorsese_core::Curve;

use crate::frame::Frame;
use crate::paint;

use super::{Arrow, Border};

/// How far the bow of an S reaches along the run, as a fraction of it.
///
/// A half puts each control point above or beside the midpoint, which is the
/// familiar connector shape. It is not a field for the same reason the bow
/// *axis* is not one: an author drawing a diagram has an opinion about which
/// two things are joined, not about the curvature of the join.
const BOW: f32 = 0.5;

/// How long a head is, as a multiple of the line's thickness.
const HEAD_LENGTH: f32 = 4.0;

/// How far a head's base spreads either side of the line, as a multiple of its
/// thickness. Kept below half the length so the head reads as a point rather
/// than as a wedge.
const HEAD_SPREAD: f32 = 1.6;

/// Draws `arrow` onto `frame` in `border`'s colour and thickness.
///
/// The line first, then the heads over it. They are the same colour, so the
/// order is invisible in the result and is chosen for a different reason: the
/// head is the part that must land exactly on the endpoint, and drawing it last
/// means nothing can be laid over its tip.
pub(super) fn draw(frame: &mut Frame, arrow: Arrow, border: Border) {
    if !border.width.is_finite() || border.width <= 0.0 {
        return;
    }
    let Some(run) = Run::of(arrow) else {
        return;
    };

    let mut line = PathBuilder::new();
    line.move_to(arrow.from.0, arrow.from.1);
    match run.control {
        Some((first, second)) => {
            line.cubic_to(first.0, first.1, second.0, second.1, arrow.to.0, arrow.to.1)
        }
        None => line.line_to(arrow.to.0, arrow.to.1),
    }
    if let Some(path) = line.finish() {
        paint::stroke(frame, &path, border.color, border.width);
    }

    if arrow.heads.at_end() {
        head(frame, arrow.to, run.at_end, border);
    }
    // The head at the start points back the way the line came, so its direction
    // is the arriving tangent reversed.
    if arrow.heads.at_start() {
        head(
            frame,
            arrow.from,
            (-run.at_start.0, -run.at_start.1),
            border,
        );
    }
}

/// An arrow's shape once its geometry has been worked out: the control points
/// if it is bowed, and the unit tangent at each end.
struct Run {
    /// The two Bézier control points, or `None` for a straight line.
    control: Option<((f32, f32), (f32, f32))>,
    /// Which way the line is travelling as it *arrives* at `to` — the
    /// direction the head there points.
    at_end: (f32, f32),
    /// Which way it is travelling as it *leaves* `from`. A head there points
    /// back along it.
    at_start: (f32, f32),
}

impl Run {
    /// `None` when the two ends are in the same place: there is no direction to
    /// draw along and none to aim a head down. Validation refuses that in a
    /// loaded project, so this covers a figure built in memory.
    fn of(arrow: Arrow) -> Option<Self> {
        let (dx, dy) = (arrow.to.0 - arrow.from.0, arrow.to.1 - arrow.from.1);
        if !dx.is_finite() || !dy.is_finite() {
            return None;
        }
        match arrow.curve {
            Curve::Straight => {
                let direction = unit(dx, dy)?;
                Some(Self {
                    control: None,
                    at_end: direction,
                    at_start: direction,
                })
            }
            Curve::S => {
                // The bow is along whichever axis the two ends are further
                // apart on, so two boxes side by side get a connector that
                // leaves rightward and arrives leftward, and two stacked get
                // one that leaves downward. Inferred rather than asked for:
                // nobody drawing a diagram has an opinion about it.
                let (first, second) = if dx.abs() >= dy.abs() {
                    (
                        (arrow.from.0 + dx * BOW, arrow.from.1),
                        (arrow.to.0 - dx * BOW, arrow.to.1),
                    )
                } else {
                    (
                        (arrow.from.0, arrow.from.1 + dy * BOW),
                        (arrow.to.0, arrow.to.1 - dy * BOW),
                    )
                };
                // A cubic's tangent at each end runs from the end to the
                // control point beside it. When the bow collapses — the two
                // ends level on the bowed axis — that vector is zero and the
                // straight line between the ends is the honest fallback.
                let straight = unit(dx, dy)?;
                Some(Self {
                    control: Some((first, second)),
                    at_end: unit(arrow.to.0 - second.0, arrow.to.1 - second.1).unwrap_or(straight),
                    at_start: unit(first.0 - arrow.from.0, first.1 - arrow.from.1)
                        .unwrap_or(straight),
                })
            }
        }
    }
}

/// Fills one triangular head with its tip at `tip`, pointing along `direction`.
fn head(frame: &mut Frame, tip: (f32, f32), direction: (f32, f32), border: Border) {
    let length = border.width * HEAD_LENGTH;
    let spread = border.width * HEAD_SPREAD;
    // Back along the line from the tip, and then out to each side of it.
    let base = (tip.0 - direction.0 * length, tip.1 - direction.1 * length);
    let side = (-direction.1 * spread, direction.0 * spread);

    let mut builder = PathBuilder::new();
    builder.move_to(tip.0, tip.1);
    builder.line_to(base.0 + side.0, base.1 + side.1);
    builder.line_to(base.0 - side.0, base.1 - side.1);
    builder.close();
    if let Some(path) = builder.finish() {
        paint::fill(frame, &path, border.color);
    }
}

/// A direction of length one, or `None` when there is no direction at all.
fn unit(dx: f32, dy: f32) -> Option<(f32, f32)> {
    let length = dx.hypot(dy);
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some((dx / length, dy / length))
}
