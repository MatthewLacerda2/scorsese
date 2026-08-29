//! The tracking wobble: whole pixels, varying down the frame, moving with it.

use std::collections::BTreeSet;

use scorsese_core::Vhs;

use super::{LEFT, RIGHT, SIZE, instant, pixel, plate, shift_of, taped};

/// The wobble at full strength, which on a 64-wide layer is `1.0 × 0.05 × 64`,
/// so a peak of three pixels either way once rounded.
const WOBBLE: Vhs = Vhs {
    jitter: 1.0,
    ..Vhs::NONE
};

/// A row read late is a row *moved over*, not a row resampled — so every pixel
/// of the result is one of the two colours the plate is made of and never a
/// blend of them. This is what makes the artefact structure rather than
/// texture, which `docs/golden-renders.md` turns on.
#[test]
fn the_displacement_is_whole_pixels_and_blends_nothing() {
    let frame = taped(&plate(), instant(WOBBLE, "c1", 0));
    for y in 0..SIZE {
        for x in 0..SIZE {
            let found = pixel(&frame, x, y);
            assert!(
                found == (LEFT[0], LEFT[1], LEFT[2], u8::MAX)
                    || found == (RIGHT[0], RIGHT[1], RIGHT[2], u8::MAX),
                "({x}, {y}) is {found:?}, which is neither half of the plate"
            );
        }
    }
}

/// Never further than the amplitude asked for: `0.05` of the width at `1.0`,
/// which is 3.2 pixels on this layer and 3 once a whole-pixel displacement has
/// rounded it.
#[test]
fn the_wobble_stays_inside_the_amplitude() {
    let frame = taped(&plate(), instant(WOBBLE, "c1", 0));
    for y in 0..SIZE {
        assert!(shift_of(&frame, y).abs() <= 3, "row {y}");
    }
}

/// A *wobble* and not a shift: the displacement varies down the frame, so the
/// picture leans rather than sliding sideways in one piece.
#[test]
fn the_displacement_varies_down_the_frame() {
    let frame = taped(&plate(), instant(WOBBLE, "c1", 0));
    let shifts: BTreeSet<i32> = (0..SIZE).map(|y| shift_of(&frame, y)).collect();
    assert!(
        shifts.len() > 1,
        "every row moved by the same {shifts:?}, which is a slide and not a wobble"
    );
}

/// Two renders of one instant are one picture. A wobble drawn from a generator
/// would differ between them, which does not fail the pixel gate loudly — it
/// makes it meaningless.
#[test]
fn the_same_instant_twice_is_the_same_picture() {
    let once = taped(&plate(), instant(WOBBLE, "c1", 7));
    let again = taped(&plate(), instant(WOBBLE, "c1", 7));
    assert_eq!(once.bytes(), again.bytes());
}

/// And it moves. A tracking error that held still would be a picture nailed
/// crooked to the wall rather than a tape being read.
#[test]
fn a_later_frame_of_the_same_clip_leans_differently() {
    let first = taped(&plate(), instant(WOBBLE, "c1", 0));
    let later = taped(&plate(), instant(WOBBLE, "c1", 1));
    assert_ne!(first.bytes(), later.bytes());
}

/// Two clips of the same footage never wobble together, which is the mistake
/// somebody would notice: the clip's own id is in the seed.
#[test]
fn two_clips_at_one_instant_wobble_apart() {
    let one = taped(&plate(), instant(WOBBLE, "c1", 0));
    let other = taped(&plate(), instant(WOBBLE, "c2", 0));
    assert_ne!(one.bytes(), other.bytes());
}

/// The clip's own field reaches the compositor. Every other test here would
/// pass just as well if the tape were read from somewhere else, so this is the
/// one that says `Properties::at` looks at `Clip::vhs` — and a layer carrying
/// nothing but a tape must not take the copy path, which is the other half of
/// the same claim.
#[test]
fn a_clip_carrying_only_a_tape_is_not_copied_through() {
    let plain = taped(&plate(), instant(Vhs::NONE, "c1", 0));
    assert_eq!(plain.bytes(), plate().bytes(), "no tape changes nothing");
    let taped_frame = taped(&plate(), instant(WOBBLE, "c1", 0));
    assert_ne!(taped_frame.bytes(), plate().bytes());
}
