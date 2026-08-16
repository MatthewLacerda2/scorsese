//! Drawing the shapes a diagram is made of: boxes, ellipses and arrows.
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
//! **A closed shape and an arrow are placed differently, and the split is the
//! shape of this module.** `closed` draws something with an area: it has a
//! size, and where it sits inside the raster is decided from its `anchor` —
//! here rather than by the compositor, because a shape layer is raster-sized
//! and a raster-sized layer rests at the origin whatever its anchor. `arrow`
//! draws a line between two points that already say where they are, so no
//! anchor reaches it at all.
//!
//! Not to be confused with `text::shape`, which is a verb: turning characters
//! into positioned glyphs. Nothing in this module has anything to do with it.

pub(crate) mod arrow;
pub(crate) mod closed;

use scorsese_core::{Anchor, Curve, Heads, Rgba};

use crate::area::Area;
use crate::frame::{Frame, Resolution};

/// One shape to draw, in pixels of the raster it goes on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Figure {
    /// Which outline, and — depending on the outline — how big or where.
    pub outline: Outline,
    /// What the inside is painted, if anything. **Read only for a closed
    /// shape**: a line has no inside, and a project that gave one a fill is
    /// refused before it reaches here.
    pub fill: Option<Rgba>,
    /// What the line is drawn in and how thick, if there is one. For an arrow
    /// this is the whole of it — no border, no arrow.
    pub border: Option<Border>,
}

/// The outline a figure has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outline {
    /// Four corners, rounded by this many pixels — `0.0` for square ones.
    Rectangle {
        /// The box it fills.
        bounds: Boxed,
        /// Corner radius in pixels, clamped on the way in to half the shorter
        /// side. Past that there is no straight edge left between two corners
        /// and the curves would cross.
        radius: f32,
    },
    /// The ellipse inscribed in a box.
    Ellipse(Boxed),
    /// A line from one point to another, with a head on it.
    Arrow(Arrow),
}

/// How big a closed shape is and which edges it is measured from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Boxed {
    /// Width and height in pixels.
    pub size: (f32, f32),
    /// Which edges of the frame the shape's own edges are measured from.
    pub anchor: Anchor,
}

/// A line between two points of the raster, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrow {
    /// Where it starts, in pixels from the frame's top-left.
    pub from: (f32, f32),
    /// Where it ends, and where a head goes.
    pub to: (f32, f32),
    /// Straight, or bowed into an S.
    pub curve: Curve,
    /// Which ends carry a head.
    pub heads: Heads,
}

/// A border: what colour, and how thick in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    /// The colour the line is drawn in, alpha included.
    pub color: Rgba,
    /// How thick the line is, in pixels. It straddles the outline.
    pub width: f32,
}

/// Where a closed shape's box lands on the raster, in pixels.
///
/// `None` for an arrow, which has no box, and for a size that could not
/// describe a rectangle. This is what an arrow attaches *to*: the shape's own
/// box rather than the raster-sized layer it is drawn into, because a box is
/// what a reader sees and the layer's edges are not on screen at all.
pub fn area_of(outline: &Outline, resolution: Resolution) -> Option<Area> {
    let rect = match outline {
        Outline::Rectangle { bounds, .. } => closed::bounds(*bounds, resolution)?,
        Outline::Ellipse(bounds) => closed::bounds(*bounds, resolution)?,
        Outline::Arrow(_) => return None,
    };
    Some(Area {
        left: rect.left(),
        top: rect.top(),
        width: rect.width(),
        height: rect.height(),
    })
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
    if let Outline::Arrow(arrow) = figure.outline {
        // No border, no arrow. A line is its stroke and nothing else, so there
        // is no second way for one to reach the frame.
        if let Some(border) = figure.border {
            arrow::draw(frame, arrow, border);
        }
        return;
    }
    closed::draw(frame, figure);
}
