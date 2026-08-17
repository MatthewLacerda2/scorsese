//! What a closed shape covers, measured rather than sampled.
//!
//! A sampled pixel says a box is roughly where it belongs. Two numbers say it
//! exactly: the box the ink occupies, and how many pixels' worth of ink there is
//! inside it. The second is what objects to a shape landing in *almost* the
//! right place — an anchor computed by an addition, an ellipse drawn as the
//! rectangle it is inscribed in — because the area a geometry encloses can be
//! worked out on paper and then compared against.

use std::f64::consts::PI;

use scorsese_compositor::Resolution;
use scorsese_compositor::shape::{Arrow, Figure, Outline, area_of, draw};
use scorsese_core::{Anchor, AnchorX, AnchorY, Curve, Heads};

use crate::extent::{assert_coverage, assert_extent};
use crate::{SIDE, bounds, boxed, centred, frame};

/// Half the raster on both axes, so every edge's arithmetic comes out a number
/// that can be read off by hand.
const HALF: (f32, f32) = (100.0, 100.0);

/// How much of the area a rasteriser is allowed to disagree about on a
/// straight-edged shape: a pixel's worth, over ten thousand. Every edge of a
/// rectangle here falls on a whole pixel, so there is no fringe to account for
/// and this is for arithmetic in `f32` rather than for anti-aliasing.
const SLACK: f64 = 1.0;

/// And on a curved one, where a cubic is not an arc: four of them stand *inside*
/// an ellipse rather than on it, so a measured area comes out a tenth to a
/// quarter of a percent short of the formula — 10 pixels on a circle of radius
/// 50, and 12 on an ellipse four times as wide as it is tall. Sixteen covers
/// both, and is still a small fraction of a wrong axis or a filled bounding box.
const CURVED: f64 = 16.0;

#[test]
fn a_centred_box_covers_exactly_its_own_hundred_squared() {
    let mut frame = frame();
    draw(&mut frame, &boxed(HALF, centred()));

    assert_extent(
        &frame,
        (50, 50, 149, 149),
        "a centred 100-square on a 200 raster",
    );
    assert_coverage(&frame, 10_000.0, SLACK, "a filled 100×100 box");
}

/// Each corner separately, because a pair of anchors with their axes crossed
/// would still look right on a square shape in a square frame.
#[test]
fn an_anchored_box_sits_hard_against_the_edges_it_names() {
    let mut near = frame();
    draw(
        &mut near,
        &boxed(
            HALF,
            Anchor {
                x: AnchorX::Left,
                y: AnchorY::Top,
            },
        ),
    );
    assert_extent(&near, (0, 0, 99, 99), "measured from the left and the top");

    let mut far = frame();
    draw(
        &mut far,
        &boxed(
            (60.0, 30.0),
            Anchor {
                x: AnchorX::Right,
                y: AnchorY::Bottom,
            },
        ),
    );
    assert_extent(
        &far,
        (140, 170, 199, 199),
        "a 60×30 box measured from the right and the bottom",
    );
    assert_coverage(&far, 1_800.0, SLACK, "which is still 60 by 30");
}

/// The ellipse inscribed in a box covers π/4 of it. That is the one number
/// separating it from the box it is drawn in — and, on an unequal box, from a
/// circle measured against a single axis.
#[test]
fn an_ellipse_covers_a_quarter_of_pi_of_the_box_it_fits_in() {
    let mut round = frame();
    draw(&mut round, &oval(HALF));
    assert_extent(&round, (50, 50, 149, 149), "an ellipse fills its own box");
    assert_coverage(
        &round,
        PI / 4.0 * 10_000.0,
        CURVED,
        "the ellipse inside a 100×100 box",
    );

    let mut wide = frame();
    draw(&mut wide, &oval((160.0, 40.0)));
    assert_extent(&wide, (20, 80, 179, 119), "160 across and 40 down");
    assert_coverage(
        &wide,
        PI / 4.0 * 160.0 * 40.0,
        CURVED,
        "the ellipse inside a 160×40 box",
    );
}

/// Where a shape's box lands is what an arrow attaches *to*, and it is read by
/// the render rather than by anything that draws. So this is the only place that
/// can say the box is the shape's own and not the raster-sized layer it is drawn
/// into — a distinction no pixel of a shape-only frame would show.
#[test]
fn a_closed_shapes_area_is_its_own_box_and_a_line_has_none() {
    let raster = Resolution::new(SIDE, SIDE).expect("a square raster");

    let rectangle = Outline::Rectangle {
        bounds: bounds(HALF, centred()),
        radius: 0.0,
    };
    let area = area_of(&rectangle, raster).expect("a rectangle has a box");
    assert_eq!(
        (area.left, area.top, area.width, area.height),
        (50.0, 50.0, 100.0, 100.0),
        "the centred box, in pixels of the raster"
    );

    let ellipse = Outline::Ellipse(bounds(HALF, centred()));
    assert_eq!(
        area_of(&ellipse, raster),
        Some(area),
        "an ellipse's box is the one it fits inside"
    );

    let line = Outline::Arrow(Arrow {
        from: (10.0, 10.0),
        to: (90.0, 90.0),
        curve: Curve::Straight,
        heads: Heads::End,
    });
    assert_eq!(
        area_of(&line, raster),
        None,
        "a line encloses nothing to attach to"
    );
}

fn oval(size: (f32, f32)) -> Figure {
    Figure {
        outline: Outline::Ellipse(bounds(size, centred())),
        ..boxed(size, centred())
    }
}
