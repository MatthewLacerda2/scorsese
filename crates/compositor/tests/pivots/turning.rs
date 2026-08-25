//! Turning a layer over: a rotation and a flip pivot where a scale does.

use scorsese_compositor::{Frame, Properties};
use scorsese_core::{Origin, OriginX, OriginY};

use crate::common::{BLACK, CENTRE, RED, SIZE, assert_pixel, solid, solid_of};
use crate::{assert_span, at, drawn, span};

/// A bar as wide as the canvas and a quarter as tall, resting across the
/// middle. A quarter turn about its own centre stands it upright through the
/// middle; about its left edge it swings away from there entirely.
fn bar() -> Frame {
    solid_of(RED, SIZE, SIZE / 4)
}

#[test]
fn a_turn_happens_about_the_origin_too() {
    let source = bar();
    let quarter = Properties {
        rotation: 90.0,
        ..Properties::default()
    };

    let centred = drawn(&source, quarter, Origin::default());
    assert_pixel(&centred, CENTRE, RED, "a centred turn stays in the middle");

    // Pivoting on the left edge, the bar's near end is 8 pixels either side of
    // where it rested and its far end swings down off the canvas — so what is
    // left is a strip 16 wide, half of it past the left edge.
    let hinged = drawn(&source, quarter, at(OriginX::Left, OriginY::Center));
    assert_pixel(&hinged, CENTRE, BLACK, "a hinged one no longer crosses it");
    assert_span(
        span(&hinged, |x| (x, 40)),
        (0, 7),
        "hinged on its left edge",
    );
    assert_pixel(&hinged, (4, 24), BLACK, "and swung clockwise, not up");
}

#[test]
fn a_flip_hinges_on_the_origin_rather_than_the_middle() {
    // A flip is a foreshortening — the same linear part of the same matrix a
    // scale uses — so it pivots where the scale does, and that is the card
    // hinging on its left edge rather than spinning about its own spine.
    // `flip.y` at 60° leaves `cos 60° = 0.5` of the width.
    let source = solid(RED);
    let turned = |origin| {
        drawn(
            &source,
            Properties {
                flip: (0.0, 60.0),
                ..Properties::default()
            },
            origin,
        )
    };
    let row = |frame: &Frame| span(frame, |x| (x, CENTRE.1));

    let centred = turned(Origin::default());
    assert_span(row(&centred), (16, 47), "turned about its own spine");

    let hinged = turned(at(OriginX::Left, OriginY::Center));
    assert_span(row(&hinged), (0, 31), "hinged on its left edge");
}
