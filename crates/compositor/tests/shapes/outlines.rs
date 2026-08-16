//! Which outline was actually drawn — the corners are where that shows.

use scorsese_compositor::shape::{Figure, Outline, draw};

use crate::{bounds, boxed, centred, clear, filled, frame};

#[test]
fn an_ellipse_fills_its_middle_and_not_its_corners() {
    let mut frame = frame();
    draw(
        &mut frame,
        &Figure {
            outline: Outline::Ellipse(bounds((100.0, 100.0), centred())),
            ..boxed((100.0, 100.0), centred())
        },
    );

    assert!(filled(&frame, 100, 100), "the middle");
    assert!(
        clear(&frame, 55, 55),
        "the corner of the box it is drawn in"
    );
}

/// An ellipse is the primitive, so an unequal box has to come out unequal: one
/// measured against a single axis would be a circle here.
#[test]
fn an_ellipse_takes_both_of_its_dimensions() {
    let mut frame = frame();
    draw(
        &mut frame,
        &Figure {
            outline: Outline::Ellipse(bounds((160.0, 40.0), centred())),
            ..boxed((160.0, 40.0), centred())
        },
    );

    assert!(filled(&frame, 30, 100), "wide: 160 across reaches x=30");
    assert!(
        clear(&frame, 100, 70),
        "and tall: 40 down does not reach y=70"
    );
}

/// A rounded corner is the whole difference between the two rectangle cases,
/// and the corner pixel is where it shows.
#[test]
fn a_rounded_corner_is_cut_away_and_a_square_one_is_not() {
    let mut square = frame();
    draw(&mut square, &boxed((100.0, 100.0), centred()));
    assert!(
        !clear(&square, 52, 52),
        "a square corner reaches the corner"
    );

    let mut round = frame();
    draw(
        &mut round,
        &Figure {
            outline: Outline::Rectangle {
                bounds: bounds((100.0, 100.0), centred()),
                radius: 30.0,
            },
            ..boxed((100.0, 100.0), centred())
        },
    );
    assert!(clear(&round, 52, 52), "a rounded one does not");
}

/// Half the shorter side is a pill; asking for more must not fold the path back
/// through itself.
#[test]
fn a_radius_past_the_limit_is_clamped_to_a_pill() {
    let mut frame = frame();
    draw(
        &mut frame,
        &Figure {
            outline: Outline::Rectangle {
                bounds: bounds((100.0, 60.0), centred()),
                radius: 500.0,
            },
            ..boxed((100.0, 60.0), centred())
        },
    );

    assert!(filled(&frame, 100, 100), "the middle");
    assert!(clear(&frame, 52, 72), "the corner a pill has curved away");
}
