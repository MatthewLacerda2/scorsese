//! Turning a shape asset into a layer's worth of pixels.
//!
//! One thing happens here that cannot happen in `scorsese-compositor`, and
//! nothing else does: **fractions become pixels**. A project stores a shape's
//! size as a fraction of the frame so that one document reads the same at 720p
//! and at 4K, and the raster it is a fraction *of* is a render setting, known
//! here. The same split [`crate::text`] is on the other side of, for the same
//! reason.
//!
//! The drawing itself is entirely the compositor's, and what comes out is an
//! ordinary layer. There is no shape path through the renderer beyond this
//! file: a box is composited, transformed and faded by exactly the code a video
//! clip goes through.

use scorsese_compositor::shape::{Border, Figure, Outline};
use scorsese_compositor::{Frame, Resolution};
use scorsese_core::{Anchor, Geometry, Shape};

/// Draws `shape` onto `frame`, sized against the frame's own raster.
///
/// The frame is cleared to transparent first, exactly as a title's is: a shape
/// layer is the shape and nothing else, and everywhere it is not — including
/// the middle of a hollow one — the tracks below it have to show through. A
/// layer left opaque black would paint over them, and over a black canvas that
/// mistake is invisible until there is something underneath.
pub(crate) fn paint(frame: &mut Frame, shape: &Shape, anchor: Anchor) {
    let figure = figure(shape, frame.resolution(), anchor);
    frame.fill_transparent();
    scorsese_compositor::shape::draw(frame, &figure);
}

/// The shape as the compositor takes it: the same outline, in pixels.
fn figure(shape: &Shape, resolution: Resolution, anchor: Anchor) -> Figure {
    let (width, height) = (resolution.width() as f64, resolution.height() as f64);
    // Width against the raster's width and height against its height, which is
    // what `transform.position` already does — so a shape and the offset that
    // moves it are read in the same units, and an ellipse on a 16:9 frame is
    // circular only when its numbers say so.
    let size = (
        (shape.geometry.width() * width) as f32,
        (shape.geometry.height() * height) as f32,
    );
    Figure {
        size,
        outline: outline(shape.geometry, size),
        anchor,
        fill: shape.fill,
        border: shape.stroke.map(|color| Border {
            color,
            // A thickness has no axis, so it takes the one a text `size`
            // takes: the raster's height.
            width: (shape.stroke_width * height) as f32,
        }),
    }
}

/// Which outline, and — for a rectangle — how rounded in pixels.
///
/// The radius is a fraction of the shape's own **shorter side**, so it becomes
/// pixels against the shape rather than against the frame. That is what keeps a
/// corner circular on a 16:9 raster: one number turning into one distance,
/// where a fraction of the frame would turn into two different ones and round
/// the corner into an ellipse.
fn outline(geometry: Geometry, size: (f32, f32)) -> Outline {
    match geometry {
        Geometry::Ellipse { .. } => Outline::Ellipse,
        Geometry::Rectangle { radius, .. } => Outline::Rectangle {
            radius: radius as f32 * size.0.min(size.1),
        },
    }
}
