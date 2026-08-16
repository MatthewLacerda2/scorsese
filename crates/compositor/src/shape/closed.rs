//! The shapes with an area: rectangles, rounded rectangles and ellipses.
//!
//! Where one sits inside the raster is decided here, from its anchor. A shape
//! layer is the size of the whole raster, and a raster-sized layer rests at the
//! origin whatever its anchor — so the anchor has to reach the *layout*, the
//! same way a block of text's does. `transform.*` moves it from there.

use tiny_skia::{Path, PathBuilder, Rect};

use scorsese_core::{AnchorX, AnchorY};

use crate::frame::{Frame, Resolution};
use crate::paint;

use super::{Boxed, Figure, Outline};

/// The circle-to-cubic constant: how far along each tangent a Bézier control
/// point sits to approximate a quarter turn. Four of these is a circle to
/// within a fifth of a pixel at any size a frame has, which is why every
/// rasteriser uses it rather than an exact arc nobody can draw.
const KAPPA: f32 = 0.552_284_8;

/// Draws a closed figure — anything but an arrow, which has no area and goes
/// through [`super::arrow`] instead.
pub(super) fn draw(frame: &mut Frame, figure: &Figure) {
    let Some(path) = path(figure.outline, frame.resolution()) else {
        return;
    };
    if let Some(fill) = figure.fill {
        paint::fill(frame, &path, fill);
    }
    if let Some(border) = figure.border {
        paint::stroke(frame, &path, border.color, border.width);
    }
}

/// The outline as one closed tiny-skia path, placed on the raster.
fn path(outline: Outline, resolution: Resolution) -> Option<Path> {
    let mut builder = PathBuilder::new();
    match outline {
        Outline::Ellipse(boxed) => builder.push_oval(bounds(boxed, resolution)?),
        Outline::Rectangle {
            bounds: boxed,
            radius,
        } => {
            let rect = bounds(boxed, resolution)?;
            // Half the shorter side is where the two corners of that side meet.
            // Clamped rather than refused because this is the drawing step: the
            // document is held to a radius it has room for, and a figure built
            // in memory should still come out as the pill it was asking for.
            let limit = rect.width().min(rect.height()) / 2.0;
            let radius = if radius.is_finite() {
                radius.clamp(0.0, limit)
            } else {
                0.0
            };
            if radius <= 0.0 {
                builder.push_rect(rect);
            } else {
                rounded(&mut builder, rect, radius);
            }
        }
        // An arrow never reaches here — `super::draw` sends it elsewhere — and
        // answering `None` rather than unwrapping keeps that true without a
        // panic if the two ever drift apart.
        Outline::Arrow(_) => return None,
    }
    builder.finish()
}

/// Where the shape's box lands on the raster, given the anchor.
///
/// `None` for a size that could not describe a rectangle — zero, negative or
/// not a number. Validation refuses all three in a loaded project, so this is
/// the in-memory case, and drawing nothing is the honest answer to a shape with
/// no area.
fn bounds(boxed: Boxed, resolution: Resolution) -> Option<Rect> {
    let (width, height) = boxed.size;
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let (frame_width, frame_height) = (resolution.width() as f32, resolution.height() as f32);
    let left = match boxed.anchor.x {
        AnchorX::Left => 0.0,
        AnchorX::Center => (frame_width - width) / 2.0,
        AnchorX::Right => frame_width - width,
    };
    let top = match boxed.anchor.y {
        AnchorY::Top => 0.0,
        AnchorY::Center => (frame_height - height) / 2.0,
        AnchorY::Bottom => frame_height - height,
    };
    Rect::from_xywh(left, top, width, height)
}

/// A rectangle whose corners are quarter-ellipses of `radius`, drawn clockwise
/// from the top edge.
///
/// Written out rather than reached for in tiny-skia, which has `push_rect` and
/// `push_oval` and nothing between them.
fn rounded(builder: &mut PathBuilder, bounds: Rect, radius: f32) {
    let (left, top) = (bounds.left(), bounds.top());
    let (right, bottom) = (bounds.right(), bounds.bottom());
    // How far back along each edge a corner's control point sits.
    let pull = radius * KAPPA;

    builder.move_to(left + radius, top);
    builder.line_to(right - radius, top);
    builder.cubic_to(
        right - radius + pull,
        top,
        right,
        top + radius - pull,
        right,
        top + radius,
    );
    builder.line_to(right, bottom - radius);
    builder.cubic_to(
        right,
        bottom - radius + pull,
        right - radius + pull,
        bottom,
        right - radius,
        bottom,
    );
    builder.line_to(left + radius, bottom);
    builder.cubic_to(
        left + radius - pull,
        bottom,
        left,
        bottom - radius + pull,
        left,
        bottom - radius,
    );
    builder.line_to(left, top + radius);
    builder.cubic_to(
        left,
        top + radius - pull,
        left + radius - pull,
        top,
        left + radius,
        top,
    );
    builder.close();
}
