//! Where a symbol landed and how thick it came out, measured rather than
//! sampled.
//!
//! `drawing.rs` asks whether a symbol put ink on the raster, which is the right
//! question for one malformed icon among seventeen hundred and nearly blind to
//! arithmetic: an icon centred by an addition instead of a division still puts
//! ink on the raster, and so does one whose stroke is half the thickness it was
//! asked for. These numbers are not — the box the ink occupies and how much ink
//! there is both move the moment the placement does.
//!
//! Every expected value here is worked out from the vendored path data and the
//! arithmetic in `icon::draw`, never read off a run. `minus` is `M5 12h14` and
//! `chevron-right` is `m9 18 6-6-6-6`, so the geometry is two or three points
//! and a thickness, which is what makes an area derivable on paper.

use std::f64::consts::PI;

use scorsese_compositor::Frame;
use scorsese_compositor::icon::{self, Symbol};
use scorsese_core::{Anchor, AnchorX, AnchorY};

use crate::extent::{assert_coverage, assert_extent};
use crate::{centred, frame, symbol};

/// The whole 24-unit viewbox at one pixel per unit, so a path coordinate and a
/// pixel are the same number and the arithmetic can be read by eye.
const UNIT: f32 = 24.0;

/// Lucide's own two units of stroke at one pixel each: the line straddles the
/// path, so it reaches a pixel either side of it. A centred square's origin is
/// `(48 - 24) / 2`, which is 12 on both axes.
const STROKE: f32 = 2.0;

/// A pixel's worth of area, over the tens a wrong placement moves. The round
/// caps are the only curve in any of this and a cubic standing inside an arc
/// costs a thousandth of one.
const SLACK: f64 = 1.0;

fn drawn(name: &str, anchor: Anchor, size: f32, width: f32) -> Frame {
    let mut frame = frame();
    icon::draw(
        &mut frame,
        &Symbol {
            size,
            width,
            ..symbol(name, anchor)
        },
    );
    frame
}

fn corner(x: AnchorX, y: AnchorY) -> Anchor {
    Anchor { x, y }
}

/// The centring arithmetic, which is the one #350 was filed for: a 24-unit box
/// on a 48-pixel raster starts at 12, so `minus` runs from (17, 24) to (31, 24)
/// with a cap of radius 1 either end.
#[test]
fn a_centred_square_puts_the_path_where_the_arithmetic_says() {
    let frame = drawn("minus", centred(), UNIT, STROKE);
    assert_extent(&frame, (16, 23, 31, 24), "a centred minus");
    assert_coverage(
        &frame,
        14.0 * 2.0 + PI,
        SLACK,
        "fourteen long, two thick, and a round cap either end",
    );
}

/// Each edge on its own, because the two axes are separate arithmetic and a
/// crossed pair reads as *nearly right* on a square raster. Left and top put
/// the square at the origin; right and bottom put it at `48 - 24`.
#[test]
fn the_anchor_measures_the_square_from_the_edges_it_names() {
    let near = drawn("minus", corner(AnchorX::Left, AnchorY::Top), UNIT, STROKE);
    assert_extent(&near, (4, 11, 19, 12), "measured from the left and the top");

    let far = drawn(
        "minus",
        corner(AnchorX::Right, AnchorY::Bottom),
        UNIT,
        STROKE,
    );
    assert_extent(&far, (28, 35, 43, 36), "and from the right and the bottom");

    let mixed = drawn("minus", corner(AnchorX::Right, AnchorY::Top), UNIT, STROKE);
    assert_extent(&mixed, (28, 11, 43, 12), "one axis from each");
}

/// Size is the side of the square in pixels, so halving it halves every
/// coordinate *and* the thickness — the whole drawing scales rather than a
/// same-sized symbol moving inwards.
#[test]
fn a_smaller_square_scales_the_whole_drawing() {
    let half = UNIT / 2.0;
    let frame = drawn("minus", centred(), half, icon::width_for(half));
    assert_extent(&frame, (20, 23, 27, 24), "a half-size minus, still centred");
    assert_coverage(
        &frame,
        7.0 * 1.0 + PI / 4.0,
        SLACK,
        "seven long and one thick",
    );
}

/// The thickness `width_for` names is Lucide's two units scaled, and it is the
/// thickness that reaches the raster. Both halves are asserted: the number, and
/// the ink it produces.
#[test]
fn the_thickness_is_two_units_of_the_square() {
    assert_eq!(icon::width_for(UNIT), STROKE, "two units at one pixel each");
    assert_eq!(
        icon::width_for(120.0),
        10.0,
        "and it scales with the square"
    );

    let fat = drawn("minus", centred(), UNIT, 6.0);
    assert_extent(
        &fat,
        (14, 21, 33, 26),
        "six pixels thick, three either side",
    );
    assert_coverage(
        &fat,
        14.0 * 6.0 + PI * 9.0,
        SLACK,
        "fourteen long by six thick, capped with a disc of radius three",
    );
}

/// Round ends and round corners, which are part of the drawing rather than a
/// finish on it: `chevron-right` turns a right angle at (27, 24), and a mitred
/// corner would spike two and a half pixels past the arc that belongs there.
/// Drawn twelve pixels thick so that difference is pixels and not a rounding.
#[test]
fn the_corners_are_rounded_off_rather_than_mitred() {
    let frame = drawn("chevron-right", centred(), UNIT, 12.0);
    assert_extent(
        &frame,
        (15, 12, 32, 35),
        "the arc at the corner reaches 27 + 6, where a mitre would reach 27 + 8.49",
    );
}
