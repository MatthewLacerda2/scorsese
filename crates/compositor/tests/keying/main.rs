//! What a chroma key does to a layer's pixels, end to end through the
//! compositor.
//!
//! **Over black, and that is what makes the alpha readable.** A canvas is
//! opaque, so a keyed layer's transparency is not something a test can look at
//! directly — but composited alone onto the black a canvas starts as, a pixel
//! comes back as its own colour scaled by how much of it survived. A screen
//! that keyed to nothing is black; a subject that survived whole is exactly
//! itself; an edge in the ramp is somewhere between, and *where* between pins
//! the alpha to a level or two.
//!
//! That path is also the one thing a unit test on the arithmetic cannot reach:
//! a keyed layer is no longer opaque whatever its source was, so it has to be
//! premultiplied before the rasteriser sees it. Keyed pixels composited at
//! their unscaled colours would be visibly wrong here and nowhere else.
//!
//! Every expected number below is worked out on paper from what `chroma.rs`
//! documents — the chromaticity of each colour, its distance from the screen,
//! and the ramp — rather than read off a run and blessed.

mod matte;
mod resolving;
mod spill;

use scorsese_compositor::{
    BYTES_PER_PIXEL, Compositor, CpuCompositor, Frame, Layer, Properties, Resolution,
};
use scorsese_core::{ChromaKey, Rgba};

/// The plate is one row per case and one column per lighting band, which is all
/// these need: a key reads one pixel and writes one, so a bigger raster would
/// assert the same thing more times.
pub(crate) const WIDTH: u32 = 6;
pub(crate) const HEIGHT: u32 = 2;

/// The green most screens are actually painted — never `#00ff00`, which is why
/// a key names a colour rather than assuming one.
pub(crate) const SCREEN: Rgba = Rgba::opaque(0, 177, 64);
/// The same screen at 42% and at 20% of the light: **one chroma, three lumas**,
/// which is what an unevenly lit screen is. They sit `0.003` and `0.009` from
/// the lit one in the plane the key measures in, where an RGB distance would
/// put them `0.41` and `0.53` away.
pub(crate) const DIM: Rgba = Rgba::opaque(0, 74, 27);
pub(crate) const DIMMER: Rgba = Rgba::opaque(0, 35, 13);
/// A subject, `0.702` from the screen — further than a primary is from white.
pub(crate) const SKIN: Rgba = Rgba::opaque(199, 161, 140);
/// A strand of hair three-quarters covered by screen: `0.288` out, which is
/// inside the ramp the tests below key it with and outside a narrower one.
pub(crate) const STRAND: Rgba = Rgba::opaque(50, 173, 83);
/// The subject with the screen's bounce laid over it — the thing a despill
/// exists for. It is `0.601` out, so the key keeps it whole and only the
/// suppression touches it.
pub(crate) const SPILLED: Rgba = Rgba::opaque(199, 223, 162);

/// The plate, a column per case and the same in both rows.
pub(crate) fn plate() -> Frame {
    let mut frame = Frame::black(raster());
    let columns = [SCREEN, DIM, DIMMER, STRAND, SKIN, SPILLED];
    for (index, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        pixel.copy_from_slice(&columns[index % WIDTH as usize].channels());
    }
    frame
}

/// A key on the screen, at a tolerance and with the ramp the pinned numbers
/// were worked out at.
pub(crate) fn key(tolerance: f64) -> ChromaKey {
    ChromaKey {
        color: SCREEN,
        tolerance,
        softness: 0.1,
        spill: false,
    }
}

/// `source` keyed and composited onto black, with no geometry at all — so what
/// lands on the canvas is this key's arithmetic and nothing a rasteriser did on
/// the way.
pub(crate) fn keyed(source: &Frame, key: ChromaKey) -> Frame {
    let layer = Layer {
        properties: Properties {
            chroma_key: Some(key),
            ..Properties::default()
        },
        ..Layer::plain(source)
    };
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .composite(&mut canvas, &[layer])
        .expect("compositing succeeds");
    canvas
}

/// One pixel as `(r, g, b)`. The alpha is not worth returning: a canvas is
/// opaque, which is the whole reason these read colours over black.
pub(crate) fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let at = (y as usize * WIDTH as usize + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    (bytes[0], bytes[1], bytes[2])
}

/// Within a level, which is what a premultiply and a rasterise can move a byte
/// by and no arithmetic error here would stop at.
pub(crate) fn near(found: (u8, u8, u8), expected: (u8, u8, u8), what: &str) {
    let close = |a: u8, b: u8| a.abs_diff(b) <= 1;
    assert!(
        close(found.0, expected.0) && close(found.1, expected.1) && close(found.2, expected.2),
        "{what}: found {found:?}, expected {expected:?}"
    );
}

pub(crate) fn raster() -> Resolution {
    Resolution::new(WIDTH, HEIGHT).expect("a legal raster")
}
