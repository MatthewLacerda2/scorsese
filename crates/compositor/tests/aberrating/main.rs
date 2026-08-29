//! What pulling a layer's colour channels apart does to its pixels.
//!
//! **A ramp, because a ramp makes the arithmetic exact.** These composite a
//! grey ramp — every channel `4 · x`, so all three start life on top of one
//! another — and read the bytes back. A displacement of a fraction of a pixel
//! is invisible on a flat plate and *bilinear sampling of a linear function is
//! exact*, so on a ramp every expected value below is a number worked out on
//! paper from what `aberration.rs` documents rather than a number read off a
//! run and blessed.
//!
//! Green is the control in every one of them: it is the channel that does not
//! move, so a green byte that is not `4 · x` says the effect leaked out of the
//! two channels it is meant to be confined to.
//!
//! What the golden fixtures do not cover, they cannot: a displacement this
//! small does not survive an x264 encode intact, which
//! `docs/golden-renders.md` measures and explains. So the claim is pinned here,
//! before an encoder has seen it, exactly as grain's is.

mod edges;
mod geometry;

use scorsese_compositor::{
    BYTES_PER_PIXEL, Compositor, CpuCompositor, Frame, Layer, Properties, Resolution,
};

/// The square these tests work on. 64 across, so the ramp below reaches 252
/// and the centre falls exactly between two pixels.
pub(crate) const SIZE: u32 = 64;

/// The aberration every pinned value is worked out at: `0.125`, so the scale on
/// the distance from the centre is `0.25`.
///
/// Far past anything anybody would put in a project — the point of a fringe is
/// that you feel it rather than see it — because a value in the ordinary range
/// moves a pixel by less than a byte of ramp and would be asserting rounding.
pub(crate) const STRONG: f64 = 0.125;

/// A grey ramp across the raster: `4 · x` on all three channels, opaque, and
/// constant down every column so a pixel's row never enters the arithmetic.
pub(crate) fn ramp() -> Frame {
    let mut frame = Frame::black(raster());
    for (index, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let value = (index as u32 % SIZE * 4) as u8;
        pixel.copy_from_slice(&[value, value, value, u8::MAX]);
    }
    frame
}

/// A layer with an aberration on it and no geometry at all, so what lands on
/// the canvas is this module's arithmetic and nothing a rasteriser did on the
/// way.
pub(crate) fn aberrated(source: &Frame, aberration: f64) -> Frame {
    let layer = Layer {
        properties: Properties {
            aberration,
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

/// One pixel as `(r, g, b, a)`.
pub(crate) fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let at = (y as usize * SIZE as usize + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    (bytes[0], bytes[1], bytes[2], bytes[3])
}

pub(crate) fn raster() -> Resolution {
    Resolution::new(SIZE, SIZE).expect("a legal raster")
}
