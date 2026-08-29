//! The two things a tape does to whole rows: the line pitch, and the switch.

use scorsese_core::Vhs;

use super::{SIZE, flat, instant, pixel, plate, shift_of, taped};

/// A flat mid-grey, whose brightness is exactly `128` and whose colour
/// differences are exactly zero — so a darkened row is one multiplication and
/// nothing else.
const GREY: [u8; 4] = [128, 128, 128, 255];

/// Alternate rows keep `1 − 0.5 × 0.6 = 0.7` of their brightness, so a `128`
/// row comes back `89.6`, which rounds to `90`. The first half of every period
/// is the dark one, and on a 64-tall layer the period is two rows — there being
/// nothing finer for a pattern of two states to be — so the even rows are dark.
#[test]
fn the_dark_rows_are_the_even_ones_and_they_darken_by_exactly_the_amount_asked() {
    let vhs = Vhs {
        scanlines: 0.5,
        ..Vhs::NONE
    };
    let frame = taped(&flat(GREY), instant(vhs, "c1", 0));
    for y in 0..SIZE {
        let want = if y % 2 == 0 { 90 } else { 128 };
        assert_eq!(
            pixel(&frame, SIZE / 2, y),
            (want, want, want, u8::MAX),
            "row {y}"
        );
    }
}

/// A scanline darkens the *picture*, so it scales the colour differences with
/// the brightness rather than draining the colour out of every other row.
#[test]
fn a_dark_row_keeps_its_colour() {
    let vhs = Vhs {
        scanlines: 1.0,
        ..Vhs::NONE
    };
    let frame = taped(&flat([200, 40, 40, 255]), instant(vhs, "c1", 0));
    let (r, g, b, _) = pixel(&frame, 0, 0);
    let (bright_r, bright_g, bright_b, _) = pixel(&frame, 0, 1);
    assert_eq!((bright_r, bright_g, bright_b), (200, 40, 40));
    assert!(
        r < bright_r,
        "the dark row is darker: {r} against {bright_r}"
    );
    // Scaled, not desaturated: every channel took the same factor, so the ratio
    // between them survives. `0.4` of the bright row, to within rounding.
    for (dark, bright) in [(r, bright_r), (g, bright_g), (b, bright_b)] {
        let scaled = f64::from(bright) * 0.4;
        assert!(
            (f64::from(dark) - scaled).abs() <= 1.0,
            "{dark} against {scaled:.2}"
        );
    }
}

/// The band is a count of rows at the **bottom** and nothing above it moves:
/// `1.0 × 0.08 × 64` is `5.12`, so five rows, and rows 0 to 58 come back
/// exactly as a layer with no switch at all.
#[test]
fn the_switch_tears_five_rows_at_the_bottom_and_leaves_the_rest_alone() {
    let switched = taped(
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
    // A tape that does something else entirely, so the comparison is against a
    // layer that went through this stage rather than one that skipped it.
    let plain = taped(
        &plate(),
        instant(
            Vhs {
                scanlines: 0.0,
                mono: false,
                ..Vhs::NONE
            },
            "c1",
            0,
        ),
    );
    for y in 0..SIZE - 5 {
        assert_eq!(shift_of(&switched, y), shift_of(&plain, y), "row {y}");
    }
    for y in SIZE - 5..SIZE {
        assert!(shift_of(&switched, y) > 0, "row {y} is torn rightward");
    }
}

/// Worst at the bottom, which is where the switch is: the tear grows down the
/// band rather than displacing the whole of it by one amount. Even at the
/// raggedest the band's first row cannot reach its last — `0.65` of the full
/// tear against `0.2` of it.
#[test]
fn the_tear_widens_toward_the_bottom_edge() {
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
    assert!(shift_of(&frame, SIZE - 1) > shift_of(&frame, SIZE - 5));
}

/// And it loses its colour with its position, in colour mode: what is left of
/// the signal where the heads hand over is not a picture with a cast.
#[test]
fn the_torn_band_comes_back_grey_even_in_colour() {
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
    let (r, g, b, _) = pixel(&frame, 0, SIZE - 1);
    assert_eq!((g, b), (r, r), "the band is grey");
    let (r, g, b, _) = pixel(&frame, 0, SIZE - 6);
    assert_eq!((r, g, b), (200, 40, 40), "and the row above it is not");
}
