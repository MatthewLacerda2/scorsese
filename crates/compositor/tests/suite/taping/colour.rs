//! The chroma smear, and the mode that says there is no chroma to smear.

use scorsese_core::Vhs;

use super::{LEFT, RIGHT, SIZE, STEP, instant, luma, pixel, plate, taped};

/// A smear at full strength, which on a 64-wide plate is a window of five
/// columns: `1.0 × 0.08 × 64` is 5.12, and the window is that rounded.
const HEAVY: Vhs = Vhs {
    chroma_bleed: 1.0,
    ..Vhs::NONE
};

/// The pinned row, worked out from what the module documents and written down.
///
/// The plate is `(200, 40, 40)` up to column 32 and `(40, 40, 200)` from there.
/// On the colour-difference axes that is `Y 74.016, Cb −34.016, Cr 125.984` and
/// `Y 51.552, Cb 148.448, Cr −11.552`. The smear is a **trailing** mean over
/// the five columns ending at each pixel, on `Cb` and `Cr` only, so column `x`
/// past the step mixes `x − 31` of the right half with the rest of the left:
///
/// | x | Cb | Cr | r | g | b |
/// | --- | --- | --- | --- | --- | --- |
/// | 31 | −34.016 | 125.984 | 200 | 40 | 40 |
/// | 32 | 2.477 | 98.477 | 150 | 22 | 54 |
/// | 33 | 38.970 | 70.970 | 123 | 27 | 91 |
/// | 34 | 75.462 | 43.462 | 95 | 31 | 127 |
/// | 35 | 111.955 | 15.955 | 68 | 36 | 164 |
/// | 36 | 148.448 | −11.552 | 40 | 40 | 200 |
///
/// Every one of those is what the *direction* and the *distance* both come to,
/// which is the point: a centred window, a window one column wider, or a smear
/// that ran leftward changes numbers in this table and nothing about the shape
/// of the picture that a comparison would catch.
const PINNED: [(u32, u8, u8, u8); 6] = [
    (31, 200, 40, 40),
    (32, 150, 22, 54),
    (33, 123, 27, 91),
    (34, 95, 31, 127),
    (35, 68, 36, 164),
    (36, 40, 40, 200),
];

#[test]
fn the_colour_lands_exactly_where_the_arithmetic_says() {
    let frame = taped(&plate(), instant(HEAVY, "c1", 0));
    for (x, r, g, b) in PINNED {
        assert_eq!(pixel(&frame, x, SIZE / 2), (r, g, b, u8::MAX), "column {x}");
    }
}

/// Rightward and never leftward, which is the whole of what makes it a tape:
/// the colour arrived **late**. A centred window would fringe both sides of the
/// step equally, which is what a lens does and is `aberration`'s job.
#[test]
fn nothing_at_all_happens_on_the_near_side_of_the_edge() {
    let frame = taped(&plate(), instant(HEAVY, "c1", 0));
    for x in 0..STEP {
        assert_eq!(
            pixel(&frame, x, SIZE / 2),
            (LEFT[0], LEFT[1], LEFT[2], u8::MAX),
            "column {x} is before the step and must be untouched"
        );
    }
}

/// The brightness is not part of it. The smear runs on the colour differences,
/// so a pixel in the middle of it is the *right* brightness in the wrong
/// colour — which is what a low chroma bandwidth means and what separates this
/// from a blur.
#[test]
fn the_smear_leaves_the_brightness_alone() {
    let frame = taped(&plate(), instant(HEAVY, "c1", 0));
    let want = luma((RIGHT[0], RIGHT[1], RIGHT[2], u8::MAX));
    for x in STEP..STEP + 5 {
        let got = luma(pixel(&frame, x, SIZE / 2));
        assert!(
            (got - want).abs() <= 1.0,
            "column {x}: brightness {got:.2}, wanted {want:.2}"
        );
    }
}

/// `mono` is not "the smear plus a desaturation". It is the chroma path not
/// being modelled at all, so the picture comes back grey — every channel equal,
/// at the brightness the pixel already had — and the smear does nothing, there
/// being nothing left for it to move.
#[test]
fn mono_greys_the_picture_and_leaves_the_smear_nothing_to_do() {
    let frame = taped(
        &plate(),
        instant(
            Vhs {
                mono: true,
                ..HEAVY
            },
            "c1",
            0,
        ),
    );
    let plain = taped(
        &plate(),
        instant(
            Vhs {
                mono: true,
                ..Vhs::NONE
            },
            "c1",
            0,
        ),
    );
    for x in 0..SIZE {
        let (r, g, b, a) = pixel(&frame, x, SIZE / 2);
        assert_eq!((r, g, b), (r, r, r), "column {x} is grey");
        assert_eq!(a, u8::MAX);
        assert_eq!(
            pixel(&frame, x, SIZE / 2),
            pixel(&plain, x, SIZE / 2),
            "column {x}: a smear with no chroma path to run on changes nothing"
        );
    }
    // And the grey is the pixel's own brightness rather than an average of its
    // channels, which would make the left half far darker than it looks.
    let (r, ..) = pixel(&frame, 0, SIZE / 2);
    assert_eq!(
        f64::from(r).round() as u8,
        luma((200, 40, 40, 255)).round() as u8
    );
}
