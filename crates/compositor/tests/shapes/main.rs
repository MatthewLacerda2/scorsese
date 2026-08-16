//! Drawn shapes, grouped by what is being asked of them.
//!
//! The arithmetic only — that a box of a given size sits where the anchor says,
//! that a border straddles the outline it bounds, that an ellipse does not
//! reach into the corners of its own box. How an anti-aliased edge comes out is
//! the golden renders' question, and asserting a soft pixel here would be
//! pinning tiny-skia's rasteriser somewhere no image diff could ever review.

mod outlines;
mod painting;
mod placing;

use scorsese_compositor::shape::{Figure, Outline};
use scorsese_compositor::{Frame, Resolution};
use scorsese_core::{Anchor, AnchorX, AnchorY, Rgba};

pub(crate) const RED: Rgba = Rgba::new(0xff, 0x00, 0x00, 0xff);
pub(crate) const BLUE: Rgba = Rgba::new(0x00, 0x00, 0xff, 0xff);

/// Square, so both axes are read the same way, and large enough that a border
/// is many pixels rather than a rounding.
pub(crate) const SIDE: u32 = 200;

/// Transparent, which is what a shape layer is handed: the shape is all it
/// contributes, and everywhere it is not the tracks below have to show through.
pub(crate) fn frame() -> Frame {
    let mut frame = Frame::black(Resolution::new(SIDE, SIDE).expect("a square raster"));
    frame.fill_transparent();
    frame
}

pub(crate) fn at(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let i = ((y * SIDE + x) * 4) as usize;
    let b = frame.bytes();
    (b[i], b[i + 1], b[i + 2], b[i + 3])
}

/// True where nothing has been drawn. Checked as often as the geometry is: a
/// shape that filled its surround would pass every test that only looked at
/// where the shape *is*.
pub(crate) fn clear(frame: &Frame, x: u32, y: u32) -> bool {
    at(frame, x, y).3 == 0
}

/// Solid red, which is what every fill in these tests is.
pub(crate) fn filled(frame: &Frame, x: u32, y: u32) -> bool {
    at(frame, x, y) == (RED.r, RED.g, RED.b, 0xff)
}

pub(crate) fn boxed(size: (f32, f32), anchor: Anchor) -> Figure {
    Figure {
        size,
        outline: Outline::Rectangle { radius: 0.0 },
        anchor,
        fill: Some(RED),
        border: None,
    }
}

pub(crate) fn centred() -> Anchor {
    Anchor {
        x: AnchorX::Center,
        y: AnchorY::Center,
    }
}
