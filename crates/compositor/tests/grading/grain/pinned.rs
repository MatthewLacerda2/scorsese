//! One exact frame of noise, written down.
//!
//! Everything else in this directory asserts a *property* the grain has to
//! have — that it is the same twice, that it is monochrome, that it is centred,
//! that it is gone in the highlights — and **almost any hash at all satisfies
//! every one of them**. So nothing else here would notice the noise function
//! being replaced wholesale, which is precisely the change that silently makes
//! every project already on disk render a different picture.
//!
//! This is the assertion that notices, and it is the job a golden reference
//! would have done if the pixel gate could hold a noisy frame — see
//! `docs/golden-renders.md` for why it cannot. The claim is stronger here than
//! it would be there: these are the compositor's own bytes, with no encoder in
//! between, so it is exact rather than within a tolerance.
//!
//! It is portable because the arithmetic is: wrapping integer operations, and
//! then multiplies, adds and one division on `f64`, every one of them exactly
//! specified by IEEE 754. There is no `sin` or `exp` anywhere in the path, so
//! there is nothing for a platform's libm to disagree about.
//!
//! **A failure here is not a number to edit.** It means the noise moved. Either
//! that was deliberate — in which case say so, and say what the new grain looks
//! like — or it is the regression this file exists to catch.

use scorsese_compositor::Properties;
use scorsese_core::Frames;

use crate::{composited, pixel, solid};

use super::{MID, clip, grained};

/// FNV-1a over the whole frame, so a pixel that moved anywhere is a different
/// number here. The four pixels below say *where* when it does; this says
/// *whether*, over all four thousand of them.
fn digest(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
    for byte in bytes {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

#[test]
fn the_noise_field_is_this_field_and_no_other() {
    let canvas = composited(&[grained(&solid(MID), 0.5, 1)]);

    // Four pixels spread across the raster, named so a failure says *where*
    // rather than only *that*. The two corners are where an off-by-one in the
    // walk from pixel index to coordinate would land, and the one off the
    // diagonal is where swapping the two axes would.
    assert_eq!(
        [
            pixel(&canvas, 0, 0),
            pixel(&canvas, 17, 5),
            pixel(&canvas, 32, 32),
            pixel(&canvas, 63, 63),
        ],
        [
            (90, 130, 80, 255),
            (77, 117, 67, 255),
            (120, 160, 110, 255),
            (130, 170, 120, 255),
        ],
    );
    assert_eq!(digest(canvas.bytes()), 0x93AE_8F4F_5798_8EED);
}

#[test]
fn a_clip_seeds_its_noise_field_at_this_number_and_no_other() {
    // The frame above pins the noise *given* a seed; this pins where the seed
    // itself comes from. Both halves have to be written down, because every
    // other assertion about the seed is a comparison — this clip against that
    // one, this frame against the next — and a comparison is satisfied by any
    // function at all that happens to spread its answers out.
    let plate = clip("c-plate");
    assert_eq!(
        [
            Properties::at(&plate, Frames::ZERO).grain_seed,
            Properties::at(&plate, Frames(1)).grain_seed,
        ],
        [0xFA93_BBF6_C640_212C, 0xBF95_3707_066B_E43A],
    );
}
