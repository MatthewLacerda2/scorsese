//! The vendored symbol set: that it is all there, and that all of it draws.
//!
//! The catalogue is data compiled in from a blob, so the failure this suite
//! exists for is a quiet one — a record that did not decode, a name the
//! conversion dropped, an icon whose contours came out empty. With seventeen
//! hundred of them, nobody is going to notice that by looking.

mod catalogue;
mod drawing;

use scorsese_compositor::icon::{self, Symbol};
use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Resolution};
use scorsese_core::{Anchor, AnchorX, AnchorY, Rgba};

/// Small, because this is run 1767 times over.
pub(crate) const SIDE: u32 = 48;

pub(crate) const WHITE: Rgba = Rgba::new(0xff, 0xff, 0xff, 0xff);

/// Transparent, which is what an icon layer is handed.
pub(crate) fn frame() -> Frame {
    let mut frame = Frame::black(Resolution::new(SIDE, SIDE).expect("a square raster"));
    frame.fill_transparent();
    frame
}

/// One symbol, filling most of the raster, at the thickness it was drawn at.
pub(crate) fn symbol(name: &str, anchor: Anchor) -> Symbol {
    let size = f32::from(u16::try_from(SIDE).expect("a small raster")) * 0.75;
    Symbol {
        icon: icon::find(name).unwrap_or_else(|| panic!("'{name}' is a vendored icon")),
        size,
        anchor,
        color: WHITE,
        width: icon::width_for(size),
    }
}

pub(crate) fn centred() -> Anchor {
    Anchor {
        x: AnchorX::Center,
        y: AnchorY::Center,
    }
}

/// How many pixels the drawing put any ink on at all.
pub(crate) fn ink(frame: &Frame) -> usize {
    frame
        .bytes()
        .chunks_exact(BYTES_PER_PIXEL)
        .filter(|pixel| pixel[3] > 0)
        .count()
}

/// The ink inside one quarter of the raster, so a test can say *where* the
/// symbol landed rather than only that it landed.
pub(crate) fn ink_in(frame: &Frame, quadrant: (bool, bool)) -> usize {
    let half = SIDE / 2;
    let (right, bottom) = quadrant;
    (0..SIDE)
        .flat_map(|y| (0..SIDE).map(move |x| (x, y)))
        .filter(|&(x, y)| (x >= half) == right && (y >= half) == bottom)
        .filter(|&(x, y)| {
            let at = ((y * SIDE + x) * BYTES_PER_PIXEL as u32) as usize;
            frame.bytes()[at + 3] > 0
        })
        .count()
}
