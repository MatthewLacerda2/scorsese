//! Grain: that it moves, that it is the same every time, and that it looks the
//! way film does rather than the way a sensor does.
//!
//! Two files, because the two are different questions with different failure
//! modes. [`determinism`] asks whether the noise is a hash — the same project
//! twice, and a frame drawn alone against the same frame drawn after five
//! others. [`look`] asks what it does to a picture — monochrome, centred on
//! zero, and gone in the highlights.
//!
//! **Nothing here is a hand-computed literal**, unlike its siblings, and it
//! cannot be: the expected value of a hash is the hash, and writing one down
//! would assert that the code agrees with itself. What is asserted instead are
//! the properties the noise has to have.
//!
//! The golden fixtures make neither claim, and `docs/golden-renders.md` says
//! why: a committed reference of a noisy frame sits on the SSIM bar in both
//! directions at once.

mod determinism;
mod look;

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer, Properties};
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
