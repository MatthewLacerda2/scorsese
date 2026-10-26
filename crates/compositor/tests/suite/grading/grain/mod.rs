//! Grain: that it moves, that it is the same every time, and that it looks the
//! way film does rather than the way a sensor does.
//!
//! Three files, because these are three different questions with three
//! different failure modes. [`determinism`] asks whether the noise is a hash —
//! the same project twice, and a frame drawn alone against the same frame drawn
//! after five others. [`look`] asks what it does to a picture — monochrome,
//! centred on zero, and gone in the highlights. [`pinned`] asks whether it is
//! *this* noise, which the other two cannot: every assertion they make is a
//! comparison or a property, and almost any hash at all satisfies every one of
//! them, so between them they would not notice the noise function being
//! replaced wholesale.
//!
//! That is why [`pinned`] holds literals where the rest of `grading` holds
//! hand-computed arithmetic. A hand-computed expected value is impossible here
//! — the expected value of a hash is the hash — but writing the answer down is
//! not the same thing as letting the code agree with itself, because what it
//! pins is that *this project renders this picture, on every machine, forever*.
//!
//! The golden fixtures make none of these claims, and `docs/golden-renders.md`
//! says why: a committed reference of a noisy frame sits on the SSIM bar in
//! both directions at once.

mod determinism;
mod look;
mod pinned;

use scorsese_compositor::{
    BYTES_PER_PIXEL, Compositor, CpuCompositor, Frame, Layer, Properties, Resolution,
};
use scorsese_core::{AssetId, Clip, ClipId, Frames, Grade};

use super::raster;

/// A colour whose Rec.709 luma is almost exactly mid-grey, with three channels
/// far enough apart that a per-channel noise could not hide in the rounding.
const MID: (u8, u8, u8) = (100, 140, 90);

/// A grade with grain and nothing else.
fn grain(amount: f64) -> Grade {
    Grade {
        grain: amount,
        ..Grade::NEUTRAL
    }
}

/// A grained layer whose noise field sits at `seed`.
fn grained(source: &Frame, amount: f64, seed: u64) -> Layer<'_> {
    Layer {
        properties: Properties {
            grade: grain(amount),
            grain_seed: seed,
            ..Properties::default()
        },
        ..Layer::plain(source)
    }
}

/// A grained clip, which is what a seed is actually derived from.
fn clip(id: &str) -> Clip {
    let mut clip = Clip::new(
        ClipId::new(id),
        AssetId::new("a1"),
        Frames::ZERO,
        Frames(30),
    );
    clip.grade = grain(0.6);
    clip
}

/// The named frames of one clip, drawn in order through **one** compositor —
/// which is what a render does, and what a grain carrying state between frames
/// would pass while a single-frame draw failed. What comes back is the last
/// frame drawn.
fn drawn(clip: &Clip, source: &Frame, frames: impl IntoIterator<Item = u64>) -> Frame {
    let mut compositor = CpuCompositor::new();
    let mut canvas = Frame::black(raster());
    for t in frames {
        let layer = Layer {
            properties: Properties::at(clip, Frames(t)),
            ..Layer::plain(source)
        };
        compositor
            .composite(&mut canvas, &[layer])
            .expect("compositing succeeds");
    }
    canvas
}

/// A grained layer of an arbitrary raster, composited onto a canvas its own
/// size.
///
/// The 64×64 helpers in [`super`] cannot make the one claim that is about a
/// **tall** layer: a grain cell there is one pixel, and at one pixel the
/// arithmetic that sizes a cell has nothing left to do.
fn grained_raster(width: u32, height: u32, amount: f64, seed: u64) -> Frame {
    let resolution = Resolution::new(width, height).expect("a legal raster");
    let mut source = Frame::black(resolution);
    for pixel in source.bytes_mut().chunks_exact_mut(BYTES_PER_PIXEL) {
        pixel.copy_from_slice(&[MID.0, MID.1, MID.2, u8::MAX]);
    }
    let mut canvas = Frame::black(resolution);
    CpuCompositor::new()
        .composite(&mut canvas, &[grained(&source, amount, seed)])
        .expect("compositing succeeds");
    canvas
}

/// One pixel of a frame of any width, as `(r, g, b)`.
fn colour_at(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let width = frame.resolution().width() as usize;
    let at = (y as usize * width + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    (bytes[0], bytes[1], bytes[2])
}
