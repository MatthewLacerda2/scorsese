//! The bytes themselves, written down.
//!
//! Everything else in this directory asserts a *property* — the picture
//! changed, the same instant twice is one picture, some pixel is not grey — and
//! the mutation report for this branch is what those properties are worth: a
//! `+` swapped for a `*`, a `*` for a `/`, and the whole snow drawn a different
//! way, with every one of those assertions still passing. Any arithmetic
//! satisfies a comparison.
//!
//! So these are literals. **Read off a run and then checked**, which is what
//! `crates/compositor/tests/grading/grain/pinned.rs` does and for the same
//! reason: what comes out of a hash has no paper derivation to check it
//! against, and the alternative to writing the number down is asserting
//! nothing about it. What makes that safe is what makes grain's safe — the
//! arithmetic is integer until the last step and then only IEEE, with no
//! transcendental anywhere, so every target computes these same bytes.
//!
//! They are also the branch's only assertion that the *seed* reaches the
//! effect the way it is documented to: every number here moves if
//! `grain::field` splits one seed differently.

use scorsese_core::Vhs;

use super::{SIZE, flat, instant, pixel, plate, shift_of, taped};

/// The plate the snow is laid on: a flat mid-grey, whose brightness is exactly
/// `128` and whose colour differences are exactly zero — so every departure
/// below is the snow and nothing else.
const GREY: [u8; 4] = [128, 128, 128, 255];

/// Snow at half strength on that plate, clip `c1`, frame 0.
///
/// | pixel | r | g | b |
/// | --- | --- | --- | --- |
/// | 0, 0 | 161 | 143 | 164 |
/// | 1, 0 | 121 | 132 | 146 |
/// | 7, 3 | 139 | 158 | 169 |
/// | 31, 31 | 139 | 153 | 141 |
/// | 63, 63 | 123 | 102 | 112 |
///
/// Three different channels on every one of them, because a tape speckles the
/// two colour differences as well as the brightness — and each channel is a
/// *different* field of the frame's seed, which is the claim
/// [`super::snow`]'s "three fields are three" makes qualitatively and this
/// one makes to the byte.
const COLOUR: [(u32, u32, u8, u8, u8); 5] = [
    (0, 0, 161, 143, 164),
    (1, 0, 121, 132, 146),
    (7, 3, 139, 158, 169),
    (31, 31, 139, 153, 141),
    (63, 63, 123, 102, 112),
];

/// The same instant in `mono`, where the two colour fields are not drawn at
/// all. Every pixel is grey, and it is a *different* grey from the colour
/// run's red channel — 148 against 161 at the first pixel — because there the
/// red carries a colour difference this one has no chroma path for.
const MONO: [(u32, u32, u8); 5] = [
    (0, 0, 148),
    (1, 0, 131),
    (7, 3, 154),
    (31, 31, 149),
    (63, 63, 108),
];

#[test]
fn the_snow_lands_on_exactly_these_bytes() {
    let frame = taped(
        &flat(GREY),
        instant(
            Vhs {
                noise: 0.5,
                ..Vhs::NONE
            },
            "c1",
            0,
        ),
    );
    for (x, y, r, g, b) in COLOUR {
        assert_eq!(pixel(&frame, x, y), (r, g, b, u8::MAX), "({x}, {y})");
    }
}

#[test]
fn mono_snow_lands_on_exactly_these_other_bytes() {
    let frame = taped(
        &flat(GREY),
        instant(
            Vhs {
                noise: 0.5,
                mono: true,
                ..Vhs::NONE
            },
            "c1",
            0,
        ),
    );
    for (x, y, grey) in MONO {
        assert_eq!(
            pixel(&frame, x, y),
            (grey, grey, grey, u8::MAX),
            "({x}, {y})"
        );
    }
}

/// How far each row of the head-switching band is torn, in pixels, on clip
/// `c1` at frame 0 with `head_switch` at `1.0` — a band of five rows on a
/// 64-tall layer, and a tear of `0.3 × 64` pixels at the bottom of it.
///
/// **The raggedness is the whole reason this is written down.** Without it the
/// tear would be `19.2` times the row's depth into the band, rounded: `4, 8,
/// 12, 15, 19`. It is not, and the fourth row is the tell — `14` where a clean
/// ramp gives `15`, because the signal falling apart is what the band is.
const TEAR: [i32; 5] = [3, 8, 13, 14, 15];

#[test]
fn the_band_tears_by_exactly_these_amounts() {
    let frame = taped(
        &plate(),
        instant(
            Vhs {
                head_switch: 1.0,
                ..Vhs::NONE
            },
            "c1",
            0,
        ),
    );
    for (index, want) in TEAR.into_iter().enumerate() {
        let y = SIZE - 5 + index as u32;
        assert_eq!(shift_of(&frame, y), want, "row {y}");
    }
}

/// And how far the wobble leans, sampled every eighth row: eight waves down
/// the frame, so this is roughly one reading per wave.
const WOBBLE: [(u32, i32); 9] = [
    (0, 1),
    (8, -3),
    (16, 1),
    (24, -2),
    (32, -1),
    (40, 0),
    (48, -3),
    (56, 0),
    (63, -3),
];

#[test]
fn the_wobble_leans_by_exactly_these_amounts() {
    let frame = taped(
        &plate(),
        instant(
            Vhs {
                jitter: 1.0,
                ..Vhs::NONE
            },
            "c1",
            0,
        ),
    );
    for (y, want) in WOBBLE {
        assert_eq!(shift_of(&frame, y), want, "row {y}");
    }
}
