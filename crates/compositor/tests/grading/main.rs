//! The grade, one knob at a time, by the number rather than the direction.
//!
//! What defends `grade.rs` otherwise is the golden set: whole 1080p frames
//! matching PNGs. That catches a wrong operator but cannot say *which* of the
//! five knobs turned it, needs ffmpeg and fixture media, and takes minutes. So
//! these feed one known colour to one knob and assert the byte that comes back.
//!
//! Every expected value here was worked out by hand from what `grade.rs`
//! documents — the Rec.709 weights, a quarter-gain on temperature, mid-grey as
//! the contrast pivot, brightness as a plain offset, and a squared distance to
//! the corner — and written down as a literal. Recomputing the arithmetic in
//! the test would assert only that the code agrees with itself.

mod colour;
mod tone;
mod vignette;

use scorsese_compositor::{
    BYTES_PER_PIXEL, Compositor, CpuCompositor, Frame, Layer, Properties, Resolution,
};
use scorsese_core::{Anchor, Grade};

/// The square these tests grade. 64 on a side, so a pixel's distance from the
/// centre is a fraction with a power of two under it and the vignette's
/// expected values are exact rather than rounded twice.
pub(crate) const SIZE: u32 = 64;

/// An opaque frame of one colour, the size of the canvas.
pub(crate) fn solid(colour: (u8, u8, u8)) -> Frame {
    let mut frame = Frame::black(raster());
    for pixel in frame.bytes_mut().chunks_exact_mut(BYTES_PER_PIXEL) {
        pixel.copy_from_slice(&[colour.0, colour.1, colour.2, u8::MAX]);
    }
    frame
}

/// That frame with a grade on it and no geometry at all, so what lands on the
/// canvas is the graded pixel and nothing a rasteriser did on the way.
pub(crate) fn layer(source: &Frame, grade: Grade) -> Layer<'_> {
    Layer {
        source,
        properties: Properties {
            grade,
            ..Properties::default()
        },
        anchor: Anchor::default(),
    }
}

/// The canvas those layers composite to, first at the bottom.
pub(crate) fn composited(layers: &[Layer<'_>]) -> Frame {
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .composite(&mut canvas, layers)
        .expect("compositing succeeds");
    canvas
}

/// One pixel as `(r, g, b, a)`.
pub(crate) fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let at = (y as usize * SIZE as usize + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    (bytes[0], bytes[1], bytes[2], bytes[3])
}

/// What one colour comes out as under a grade that treats every pixel of a
/// layer alike — which is all of them but the vignette.
pub(crate) fn graded(colour: (u8, u8, u8), grade: Grade) -> (u8, u8, u8, u8) {
    let source = solid(colour);
    pixel(&composited(&[layer(&source, grade)]), SIZE / 2, SIZE / 2)
}

fn raster() -> Resolution {
    Resolution::new(SIZE, SIZE).expect("a legal raster")
}
