//! The rounded rectangle: eight Bézier control points, and the two measurements
//! that hold every one of them in place.
//!
//! **Area first.** A box of `w × h` rounded by `r` covers `w·h − (4 − π)r²` —
//! the four corners together give up exactly the square that a circle of `r`
//! does not fill. That single number holds every control point on the right side
//! of its own tangent, because a control point pulled the wrong way makes a
//! corner that bulges or one that sags, and both change the area.
//!
//! **Then symmetry**, because the area alone cannot see *which* corner. All four
//! are drawn from one pattern of the same arithmetic, so a square box comes out
//! symmetric about both axes; a mistake in a single corner breaks that while
//! moving the total by a fraction of a percent.

use std::f64::consts::PI;

use scorsese_compositor::Frame;
use scorsese_compositor::shape::{Figure, Outline, draw};

use crate::extent::{assert_coverage, assert_extent, column, row};
use crate::{SIDE, bounds, boxed, centred, frame};

/// A 100-square: centred on a 200 raster it runs 50..150 on both axes, which is
/// where every number below comes from.
const SIZE: (f32, f32) = (100.0, 100.0);

/// Rounded by 30 — tens of pixels of corner, and short of the limit so there is
/// still a straight edge between two of them to get wrong.
const RADIUS: f32 = 30.0;

/// Twelve pixels' worth of ink, over the seven to nine thousand a rounded box
/// covers.
///
/// Wider than the whole-pixel edges elsewhere allow, because a corner is a curve
/// and a cubic is not an arc: four of them stand *inside* a circle rather than on
/// it, so a curved shape measures about a tenth of a percent short of the area
/// the formula gives — 3.7 pixels at a radius of 30 and 8.5 at 50. This is that,
/// rounded up, and it is still a small fraction of what any of the arithmetic
/// here gets wrong.
const SLACK: f64 = 12.0;

/// How far the two halves of a symmetric shape may disagree, in pixels' worth of
/// ink along one row or column.
///
/// Not zero, because a scanline rasteriser is not exactly symmetric: measured
/// across these shapes the worst row differs by 0.07 and the worst column by
/// 0.25. A single corner drawn from a control point even a pixel out moves a row
/// by more than a whole pixel's worth, so this is loose enough to be quiet and
/// tight enough to object.
const LOPSIDED: f64 = 0.5;

#[test]
fn a_rounded_box_gives_up_exactly_the_area_its_corners_cut_away() {
    let frame = rounded(RADIUS);

    assert_extent(
        &frame,
        (50, 50, 149, 149),
        "a rounded box still fills its own box",
    );
    assert_coverage(
        &frame,
        10_000.0 - cut_away(RADIUS),
        SLACK,
        "a 100-square rounded by 30",
    );
}

/// The pill is the same statement at the limit. Rounded by half its shorter side
/// a square has become a circle, so what is left is π r² and there is no
/// straight edge anywhere — the case a radius clamped too late or not at all
/// would draw as a path folded back through itself.
#[test]
fn a_pill_is_the_same_arithmetic_at_the_limit() {
    let frame = rounded(50.0);

    assert_extent(&frame, (50, 50, 149, 149), "a pill still fills its own box");
    assert_coverage(
        &frame,
        10_000.0 - cut_away(50.0),
        SLACK,
        "a 100-square rounded until it is a circle",
    );
}

/// A square box rounded by one radius is symmetric about both of its axes, and
/// this says so a row and a column at a time. That is what notices a *single*
/// corner drawn from a control point a pixel or two out — the mistake that can
/// leave the total area near enough where it started while moving the picture
/// visibly.
///
/// Measured as a whole row's ink rather than pixel against mirrored pixel,
/// because a scanline rasteriser has a direction: individual anti-aliased pixels
/// disagree with their mirror image by a few levels out of 255, and summing a row
/// averages that away without giving up any of the sensitivity that matters.
#[test]
fn every_corner_is_the_same_corner() {
    let frame = rounded(RADIUS);
    let last = SIDE - 1;

    for i in 0..SIDE {
        let (top, bottom) = (row(&frame, i), row(&frame, last - i));
        assert!(
            (top - bottom).abs() <= LOPSIDED,
            "row {i} holds {top} pixels' worth of ink against {bottom} in its mirror"
        );
        let (left, right) = (column(&frame, i), column(&frame, last - i));
        assert!(
            (left - right).abs() <= LOPSIDED,
            "column {i} holds {left} pixels' worth of ink against {right} in its mirror"
        );
    }
}

/// The area four corners of `radius` take off a rectangle: for each, the square
/// of the radius less the quarter-circle inside it.
fn cut_away(radius: f32) -> f64 {
    (4.0 - PI) * f64::from(radius) * f64::from(radius)
}

fn rounded(radius: f32) -> Frame {
    let mut frame = frame();
    draw(
        &mut frame,
        &Figure {
            outline: Outline::Rectangle {
                bounds: bounds(SIZE, centred()),
                radius,
            },
            ..boxed(SIZE, centred())
        },
    );
    frame
}
