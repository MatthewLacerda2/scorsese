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

use scorsese_compositor::shape::{Arrow, Border, Boxed, Figure, Outline};
use scorsese_compositor::{Area, Frame, Resolution};
use scorsese_core::{Anchor, Geometry, Point, Shape};

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
    let height = f64::from(resolution.height());
    Figure {
        outline: outline(&shape.geometry, resolution, anchor),
        fill: shape.fill,
        border: shape.stroke.map(|color| Border {
            color,
            // A thickness has no axis, so it takes the one a text `size`
            // takes: the raster's height.
            width: (shape.stroke_width * height) as f32,
        }),
    }
}

/// Draws an arrow whose ends have already been worked out, in pixels.
///
/// The per-frame path: an attached arrow's ends move with the clips they
/// follow, so where they are is a fact about one instant rather than about the
/// document. Everything else about the arrow — its colours, its bow, its heads
/// — is the same for the whole segment and comes off the shape unchanged.
pub(crate) fn paint_arrow(frame: &mut Frame, shape: &Shape, from: (f32, f32), to: (f32, f32)) {
    let Geometry::Arrow { curve, heads, .. } = shape.geometry else {
        return;
    };
    let height = f64::from(frame.resolution().height());
    let figure = Figure {
        outline: Outline::Arrow(Arrow {
            from,
            to,
            curve,
            heads,
        }),
        fill: None,
        border: shape.stroke.map(|color| Border {
            color,
            width: (shape.stroke_width * height) as f32,
        }),
    };
    frame.fill_transparent();
    scorsese_compositor::shape::draw(frame, &figure);
}

/// Where this shape's own box lands on the raster, in pixels.
///
/// `None` for an arrow, which has no box. This is the rectangle an attached
/// arrow meets: what a reader actually sees, rather than the raster-sized layer
/// the shape happens to be drawn into.
pub(crate) fn area(shape: &Shape, resolution: Resolution, anchor: Anchor) -> Option<Area> {
    scorsese_compositor::shape::area_of(&outline(&shape.geometry, resolution, anchor), resolution)
}

/// Which outline, in pixels.
///
/// Width measures against the raster's width and height against its height —
/// what `transform.position` already does, so a shape and the offset that moves
/// it are read in the same units, and an ellipse on a 16:9 frame is circular
/// only when its numbers say so.
///
/// A rectangle's radius is the exception: it is a fraction of the shape's own
/// **shorter side**, so it becomes pixels against the shape rather than against
/// the frame. That is what keeps a corner circular, one number turning into one
/// distance, where a fraction of the frame would turn into two different ones
/// and round the corner into an ellipse.
pub(crate) fn outline(geometry: &Geometry, resolution: Resolution, anchor: Anchor) -> Outline {
    match *geometry {
        Geometry::Arrow {
            ref from,
            ref to,
            curve,
            heads,
        } => Outline::Arrow(Arrow {
            // An arrow with an attached end never reaches here — the renderer
            // sends those down the per-frame path, where the clip they follow
            // can be asked where it is. The origin is the honest answer to an
            // end that names no place at all.
            from: pixels(from.point().unwrap_or(Point::new(0.0, 0.0)), resolution),
            to: pixels(to.point().unwrap_or(Point::new(0.0, 0.0)), resolution),
            curve,
            heads,
        }),
        Geometry::Ellipse { width, height } => {
            Outline::Ellipse(boxed(width, height, resolution, anchor))
        }
        Geometry::Rectangle {
            width,
            height,
            radius,
        } => {
            let bounds = boxed(width, height, resolution, anchor);
            Outline::Rectangle {
                bounds,
                radius: radius as f32 * bounds.size.0.min(bounds.size.1),
            }
        }
    }
}

/// A closed shape's box in pixels, still to be placed by its anchor — which is
/// the compositor's, since only the drawing knows what a raster-sized layer has
/// left over to move within.
fn boxed(width: f64, height: f64, resolution: Resolution, anchor: Anchor) -> Boxed {
    Boxed {
        size: (
            (width * f64::from(resolution.width())) as f32,
            (height * f64::from(resolution.height())) as f32,
        ),
        anchor,
    }
}

/// One arrow endpoint in pixels from the frame's top-left.
///
/// **Absolute, where a closed shape's numbers are a size.** An arrow says where
/// its ends are outright, so no anchor is consulted — there is no rectangle to
/// rest against an edge.
fn pixels(point: Point, resolution: Resolution) -> (f32, f32) {
    (
        (point.x * f64::from(resolution.width())) as f32,
        (point.y * f64::from(resolution.height())) as f32,
    )
}
