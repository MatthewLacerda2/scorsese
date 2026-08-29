//! Where the channels go, and how far — the two halves a test that only asks
//! whether a channel *moved* leaves open.

use super::{SIZE, STRONG, aberrated, pixel, ramp};

/// The pinned row, worked out by hand and written down.
///
/// The source is `4 · x` everywhere, so an unaberrated frame's row reads
/// `(4x, 4x, 4x)`. At `STRONG` the scale on the distance from the centre is
/// `0.25`, and the centre of a 64-wide raster is at `32.0` — which in the
/// index coordinates the ramp is written in is `31.5`. So red at column `x` is
/// the ramp sampled at `31.5 + (x − 31.5) · 0.75` and blue at
/// `31.5 + (x − 31.5) · 1.25`, both clamped to the raster, both times four:
///
/// | x | red index | red | green | blue index | blue |
/// | --- | --- | --- | --- | --- | --- |
/// | 0 | 7.875 | 32 | 0 | −7.875 → 0 | 0 |
/// | 32 | 31.875 | 128 | 128 | 32.125 | 129 |
/// | 40 | 37.875 | 152 | 160 | 42.125 | 169 |
/// | 48 | 43.875 | 176 | 192 | 52.125 | 209 |
/// | 63 | 55.125 | 221 | 252 | 70.875 → 63 | 252 |
///
/// Every one of those is what the *direction* and the *distance* both come to,
/// which is the point: a mutant that swapped red for blue, dropped the factor
/// of two out of the spread, or measured the radius from a corner instead of
/// the centre changes numbers in this table and nothing about the shape of the
/// picture that a comparison would catch.
const PINNED: [(u32, u8, u8, u8); 5] = [
    (0, 32, 0, 0),
    (32, 128, 128, 129),
    (40, 152, 160, 169),
    (48, 176, 192, 209),
    (63, 221, 252, 252),
];

#[test]
fn the_channels_land_exactly_where_the_arithmetic_says() {
    let frame = aberrated(&ramp(), STRONG);
    for (x, red, green, blue) in PINNED {
        assert_eq!(
            pixel(&frame, x, SIZE / 2),
            (red, green, blue, u8::MAX),
            "column {x}"
        );
    }
}

/// Nothing at the centre, growing outward — which is what makes it a lens
/// rather than a misregistration, and the one claim a uniform shift would also
/// satisfy every other test here.
#[test]
fn the_split_is_nothing_at_the_centre_and_widens_toward_the_edge() {
    let frame = aberrated(&ramp(), STRONG);
    let split = |x: u32| {
        let (red, _, blue, _) = pixel(&frame, x, SIZE / 2);
        i32::from(blue) - i32::from(red)
    };
    assert_eq!(
        split(32),
        1,
        "half a pixel from the centre, all but nothing"
    );
    assert_eq!(split(40), 17);
    assert_eq!(split(48), 33);
    assert!(split(32) < split(40) && split(40) < split(48));
}

/// Red outward and blue inward, on **both** sides of the centre — the
/// assertion a single column cannot make, because one column is equally
/// satisfied by every channel sliding the same way.
#[test]
fn red_goes_outward_and_blue_inward_on_both_sides() {
    let frame = aberrated(&ramp(), STRONG);
    // The ramp rises to the right, so a channel sampled *nearer* the centre
    // reads lower on the right half and higher on the left half. Red is the one
    // sampled nearer, so it lands displaced away from the centre.
    let (left_red, left_green, left_blue, _) = pixel(&frame, 8, SIZE / 2);
    assert!(left_red > left_green, "{left_red} vs {left_green}");
    assert!(left_blue < left_green, "{left_blue} vs {left_green}");

    let (right_red, right_green, right_blue, _) = pixel(&frame, 56, SIZE / 2);
    assert!(right_red < right_green, "{right_red} vs {right_green}");
    assert!(right_blue > right_green, "{right_blue} vs {right_green}");
}

/// The split is radial, so it happens down a column as well as along a row —
/// the claim that would be silently lost if the vertical coordinate were
/// dropped out of the displacement and only the horizontal one kept.
#[test]
fn a_ramp_down_the_frame_splits_the_same_way_a_ramp_across_it_does() {
    let mut source = ramp();
    // The same ramp, transposed: `4 · y`, constant along every row.
    let bytes = source.bytes().to_vec();
    for (index, pixel) in source
        .bytes_mut()
        .chunks_exact_mut(scorsese_compositor::BYTES_PER_PIXEL)
        .enumerate()
    {
        let from = (index as u32 % SIZE * SIZE + index as u32 / SIZE) as usize
            * scorsese_compositor::BYTES_PER_PIXEL;
        pixel.copy_from_slice(&bytes[from..from + scorsese_compositor::BYTES_PER_PIXEL]);
    }
    let frame = aberrated(&source, STRONG);
    for (y, red, green, blue) in PINNED {
        assert_eq!(
            pixel(&frame, SIZE / 2, y),
            (red, green, blue, u8::MAX),
            "row {y}"
        );
    }
}
