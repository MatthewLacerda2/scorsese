//! Grain: that it moves, that it is the same every time, and above all that a
//! frame owes nothing to the frame drawn before it.
//!
//! The golden fixtures make the same claim about a delivered file. These make
//! it at the byte, in milliseconds, and say which half broke — a noise function
//! that drifted, or a render that stopped feeding it the frame.

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer, Properties};
use scorsese_core::{AssetId, Clip, ClipId, Frames, Grade};

use super::{SIZE, composited, pixel, raster, solid};

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

#[test]
fn no_grain_is_the_picture_exactly_as_it_arrived() {
    assert!(grain(0.0).is_neutral(), "a grain of zero changes nothing");
    let source = solid(MID);
    assert_eq!(
        composited(&[grained(&source, 0.0, 7)]).bytes(),
        source.bytes()
    );
    // And a seed nobody uses is not a difference between two instants.
    let plain = Clip::new(
        ClipId::new("c"),
        AssetId::new("a"),
        Frames::ZERO,
        Frames(30),
    );
    assert_eq!(Properties::at(&plain, Frames(4)).grain_seed, 0);
}

#[test]
fn grain_moves_pixels_and_moves_each_one_differently() {
    let canvas = composited(&[grained(&solid(MID), 1.0, 7)]);
    let mut seen = std::collections::BTreeSet::new();
    for y in 0..SIZE {
        seen.insert(pixel(&canvas, y % SIZE, y).0);
    }
    assert!(
        seen.len() > SIZE as usize / 2,
        "the noise should differ pixel to pixel, found {} values",
        seen.len()
    );
}

#[test]
fn the_same_seed_draws_the_same_grain_every_time() {
    let source = solid(MID);
    let once = composited(&[grained(&source, 0.4, 99)]);
    let again = composited(&[grained(&source, 0.4, 99)]);
    assert_eq!(
        once.bytes(),
        again.bytes(),
        "the noise is a hash, not a generator"
    );
}

#[test]
fn a_frame_owes_nothing_to_the_frames_drawn_before_it() {
    // The requirement the render pipeline actually imposes: workers draw frames
    // concurrently and out of order, so frame 5 has to come out the same
    // whether or not frames 0 to 4 were ever drawn.
    let source = solid(MID);
    let clip = clip("c-plate");
    let after = drawn(&clip, &source, 0..=5);
    let alone = drawn(&clip, &source, 5..=5);
    assert_eq!(after.bytes(), alone.bytes());
}

#[test]
fn the_grain_moves_from_one_frame_to_the_next() {
    // Static grain reads as dirt on the lens rather than as film.
    let source = solid(MID);
    let clip = clip("c-plate");
    assert_ne!(
        drawn(&clip, &source, 0..=0).bytes(),
        drawn(&clip, &source, 1..=1).bytes()
    );
}

#[test]
fn two_clips_of_the_same_footage_carry_different_grain() {
    let source = solid(MID);
    assert_ne!(
        Properties::at(&clip("c-a"), Frames(3)).grain_seed,
        Properties::at(&clip("c-b"), Frames(3)).grain_seed
    );
    assert_ne!(
        drawn(&clip("c-a"), &source, 3..=3).bytes(),
        drawn(&clip("c-b"), &source, 3..=3).bytes()
    );
}

#[test]
fn the_grain_is_one_value_on_all_three_channels() {
    // Monochrome is film; noise drawn per channel is a video sensor. The two
    // look different, and only one of them is what this is for.
    let canvas = composited(&[grained(&solid(MID), 1.0, 21)]);
    for y in 0..SIZE {
        let (r, g, b, _) = pixel(&canvas, y % SIZE, y);
        let moved = |found: u8, from: u8| i32::from(found) - i32::from(from);
        assert_eq!(
            (moved(r, MID.0), moved(g, MID.1)),
            (moved(b, MID.2), moved(b, MID.2)),
            "row {y} moved by different amounts per channel"
        );
    }
}

#[test]
fn grain_falls_away_at_both_ends_of_the_range() {
    // Uniform noise across the whole range is what reads as digital noise added
    // in post, because it puts texture into blown highlights and crushed blacks
    // where a photochemical image has none.
    for extreme in [(0, 0, 0), (255, 255, 255)] {
        let source = solid(extreme);
        assert_eq!(
            composited(&[grained(&source, 1.0, 5)]).bytes(),
            source.bytes(),
            "{extreme:?} should be left alone"
        );
    }
}
