//! Drawing a box or an ellipse onto a frame.
//!
//! What comes out is an ordinary layer — pixels on a raster — which then goes
//! through the same compositing path a decoded video frame does, with the same
//! `opacity` and `transform.*` resolved from the same keyframe tracks. There is
//! no shape *path* through the compositor, which is the point of it: a box that
//! fades and slides costs nothing here.
//!
//! **Everything is pixels.** A [`Figure`] says 240 pixels wide, not "a quarter
//! of the frame"; the fraction a project actually stores is turned into one of
//! these where the render resolution is known, exactly as a [`crate::text`]
//! [`Style`](crate::text::Style) is.
//!
//! **The anchor reaches the drawing, not the compositor.** A shape is drawn
//! into a frame the size of the whole raster, and a raster-sized layer rests at
//! the origin whatever its anchor — so where the shape sits inside that raster
//! has to be decided here, the same way a block of text's is. `transform.*`
//! moves it from there.
//!
//! Not to be confused with `text::shape`, which is a verb: turning characters
//! into positioned glyphs. Nothing in this module has anything to do with it.

use tiny_skia::{PathBuilder, Rect};

use scorsese_core::{Anchor, AnchorX, AnchorY, Rgba};

use crate::frame::{Frame, Resolution};
use crate::paint;

/// The circle-to-cubic constant: how far along each tangent a Bézier control
/// point sits to approximate a quarter turn. Four of these is a circle to
/// within a fifth of a pixel at any size a frame has, which is why every
/// rasteriser uses it rather than an exact arc nobody can draw.
const KAPPA: f32 = 0.552_284_8;

/// One shape to draw, in pixels of the raster it goes on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Figure {
    /// How big the shape is, in pixels.
    pub size: (f32, f32),
    /// Which outline those dimensions describe.
    pub outline: Outline,
    /// Which edges of the frame the shape's own edges are measured from.
    pub anchor: Anchor,
    /// What the inside is painted, if anything.
    pub fill: Option<Rgba>,
    /// What the border is drawn in and how thick, if there is one.
    pub border: Option<Border>,
}

/// The outline a figure has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outline {
    /// Four corners, rounded by this many pixels — `0.0` for square ones.
    Rectangle {
        /// Corner radius in pixels, clamped on the way in to half the shorter
        /// side. Past that there is no straight edge left between two corners
        /// and the curves would cross.
        radius: f32,
    },
    /// The ellipse inscribed in the figure's box.
    Ellipse,
}

/// A border: what colour, and how thick in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    /// The colour the line is drawn in, alpha included.
    pub color: Rgba,
    /// How thick the line is, in pixels. It straddles the outline.
    pub width: f32,
}

/// Draws `figure` onto `frame`.
///
/// **Fill first, then border**, so a translucent interior does not wash over
/// the line that bounds it — a border is the sharper edge of the two and the
/// one a reader's eye follows, so it goes on last.
///
/// The frame is drawn onto as it arrives, cleared or not. A shape layer starts
/// transparent, so the tracks underneath show through everywhere the shape is
/// not — and through the middle of it too, when there is no fill.
pub fn draw(frame: &mut Frame, figure: &Figure) {
    let Some(bounds) = bounds(figure, frame.resolution()) else {
        return;
    };
    let Some(path) = path(figure.outline, bounds) else {
        return;
    };
    if let Some(fill) = figure.fill {
        paint::fill(frame, &path, fill);
    }
    if let Some(border) = figure.border {
        paint::stroke(frame, &path, border.color, border.width);
    }
}

/// Where the shape's box lands on the raster, given the anchor.
///
/// `None` for a size that could not describe a rectangle — zero, negative or
/// not a number. Validation refuses all three in a loaded project, so this is
/// the in-memory case, and drawing nothing is the honest answer to a shape with
/// no area.
fn bounds(figure: &Figure, resolution: Resolution) -> Option<Rect> {
    let (width, height) = figure.size;
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let (frame_width, frame_height) = (resolution.width() as f32, resolution.height() as f32);
    let left = match figure.anchor.x {
        AnchorX::Left => 0.0,
        AnchorX::Center => (frame_width - width) / 2.0,
        AnchorX::Right => frame_width - width,
    };
    let top = match figure.anchor.y {
        AnchorY::Top => 0.0,
        AnchorY::Center => (frame_height - height) / 2.0,
        AnchorY::Bottom => frame_height - height,
    };
    Rect::from_xywh(left, top, width, height)
}

/// The outline as one closed tiny-skia path.
fn path(outline: Outline, bounds: Rect) -> Option<tiny_skia::Path> {
    let mut builder = PathBuilder::new();
    match outline {
        Outline::Ellipse => builder.push_oval(bounds),
        Outline::Rectangle { radius } => {
            // Half the shorter side is where the two corners of that side meet.
            // Clamped rather than refused because this is the drawing step: the
            // document is held to a radius it has room for, and a figure built
            // in memory should still come out as the pill it was asking for.
            let limit = bounds.width().min(bounds.height()) / 2.0;
            let radius = if radius.is_finite() {
                radius.clamp(0.0, limit)
            } else {
                0.0
            };
            if radius <= 0.0 {
                builder.push_rect(bounds);
            } else {
                rounded(&mut builder, bounds, radius);
            }
        }
    }
    builder.finish()
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
