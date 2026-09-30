//! What a layer's light of its own does to the frame: the shadow it casts, the
//! glow it gives off, and the blend it lands with.
//!
//! **A small square on a larger canvas, on purpose.** Every source here is a
//! 16×16 picture resting in the middle of a 64×64 frame — a layer smaller than
//! the canvas, as a cut-out picture at its native size is. That is the case
//! where a halo has to reach past the layer's own raster, and so the case that
//! proves the lit picture was padded rather than cut off square at its old box:
//! every assertion about a pixel just outside the square is that proof.
//!
//! The expected values are worked out from what `light/` documents — a shadow
//! offset by whole pixels, a halo exactly three blur radii deep, the textbook
//! blend formulas — rather than read off a run.

mod animated;
mod blend;
mod glow;
mod shadow;

use scorsese_compositor::{
    BYTES_PER_PIXEL, Compositor, CpuCompositor, Frame, Layer, Properties, Resolution,
};

/// The canvas, across and down.
pub(crate) const CANVAS: u32 = 64;

/// The square, across and down — so it rests on pixels 24 to 39 of both axes.
pub(crate) const SQUARE: u32 = 16;

/// The first pixel past the square's right and bottom edges.
pub(crate) const PAST: u32 = (CANVAS + SQUARE) / 2;

/// The canvas's middle row and column, which runs through the square.
pub(crate) const MIDDLE: u32 = CANVAS / 2;

pub(crate) fn canvas() -> Resolution {
    Resolution::new(CANVAS, CANVAS).expect("a legal raster")
}

/// A frame of one straight colour at a size of its own.
pub(crate) fn filled(colour: [u8; 4], size: u32) -> Frame {
    let mut frame = Frame::black(Resolution::source(size, size).expect("a legal source raster"));
    for pixel in frame.bytes_mut().chunks_exact_mut(BYTES_PER_PIXEL) {
        pixel.copy_from_slice(&colour);
    }
    frame
}

/// The opaque square, in one colour.
pub(crate) fn square(colour: [u8; 3]) -> Frame {
    filled([colour[0], colour[1], colour[2], u8::MAX], SQUARE)
}

/// A layer with these properties.
pub(crate) fn lit(source: &Frame, properties: Properties) -> Layer<'_> {
    Layer {
        properties,
        ..Layer::plain(source)
    }
}

/// The layers composited onto a fresh, black canvas.
pub(crate) fn composited(layers: &[Layer<'_>]) -> Frame {
    let mut frame = Frame::black(canvas());
    CpuCompositor::new()
        .composite(&mut frame, layers)
        .expect("compositing succeeds");
    frame
}

/// One pixel as `[r, g, b, a]`.
pub(crate) fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let width = frame.resolution().width() as usize;
    let at = (y as usize * width + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// Asserts a pixel's colour to within two levels — the blend's own rounding.
#[track_caller]
pub(crate) fn about(frame: &Frame, at: (u32, u32), expected: [u8; 3], what: &str) {
    let found = pixel(frame, at.0, at.1);
    let close = found.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 2);
    assert!(
        close,
        "{what}: {at:?} should be about {expected:?}, found {found:?}"
    );
    assert_eq!(found[3], u8::MAX, "{what}: the frame stays opaque");
}
