//! The noise is a hash, not a generator.
//!
//! Every claim here is one the render pipeline actually relies on: workers draw
//! frames concurrently and out of order, and a `--range` renders a slice of the
//! timeline, so a frame's grain must depend on the frame and never on what was
//! drawn before it.

use scorsese_compositor::Properties;
use scorsese_core::{AssetId, Clip, ClipId, Frames};

use crate::{composited, solid};

use super::{MID, clip, drawn, grained};

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
fn a_layer_with_no_grain_carries_no_seed() {
    // A seed for a noise field nobody draws says nothing about the layer, and
    // resolving one anyway would make two instants of an ungrained clip compare
    // unequal over a number neither of them uses.
    let plain = Clip::new(
        ClipId::new("c"),
        AssetId::new("a"),
        Frames::ZERO,
        Frames(30),
    );
    assert_eq!(Properties::at(&plain, Frames(4)).grain_seed, 0);
}
