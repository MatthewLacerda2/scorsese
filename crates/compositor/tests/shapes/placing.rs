//! Where a shape lands on the raster, and what it leaves alone.

use scorsese_compositor::shape::draw;
use scorsese_core::{Anchor, AnchorX, AnchorY};

use crate::{boxed, centred, clear, filled, frame};

#[test]
fn a_centred_box_is_centred_and_the_surround_is_untouched() {
    let mut frame = frame();
    draw(&mut frame, &boxed((100.0, 100.0), centred()));

    assert!(filled(&frame, 100, 100), "the middle of the box");
    // The box runs 50..150 on both axes, so a pixel just outside every edge is
    // still transparent.
    assert!(clear(&frame, 100, 48), "above the box");
    assert!(clear(&frame, 100, 152), "below the box");
    assert!(clear(&frame, 48, 100), "left of the box");
    assert!(clear(&frame, 152, 100), "right of the box");
}

/// The anchor has to reach the drawing: a shape layer is raster-sized, so a
/// layer-level anchor would have nothing to rest against and every shape would
/// come out centred whatever the document asked for.
#[test]
fn an_anchor_moves_the_box_to_the_corner_it_names() {
    let mut frame = frame();
    let corner = Anchor {
        x: AnchorX::Left,
        y: AnchorY::Top,
    };
    draw(&mut frame, &boxed((100.0, 100.0), corner));

    assert!(filled(&frame, 2, 2), "hard against the top-left corner");
    assert!(clear(&frame, 102, 50), "past the box's right edge");
    assert!(clear(&frame, 50, 102), "past the box's bottom edge");
}

/// Each edge separately, because a bottom-right anchor that had the two axes
/// crossed would still look right on a square shape in a square frame.
#[test]
fn the_far_edges_are_measured_from_the_far_side() {
    let mut frame = frame();
    let far = Anchor {
        x: AnchorX::Right,
        y: AnchorY::Bottom,
    };
    draw(&mut frame, &boxed((60.0, 30.0), far));

    assert!(filled(&frame, 197, 197), "hard against the bottom-right");
    assert!(clear(&frame, 137, 197), "left of a 60-wide box");
    assert!(clear(&frame, 197, 167), "above a 30-tall one");
}

/// A size that could not describe a rectangle draws nothing rather than
/// panicking or filling the frame.
#[test]
fn a_shape_with_no_area_draws_nothing() {
    let mut frame = frame();
    draw(&mut frame, &boxed((100.0, 0.0), centred()));
    assert!(clear(&frame, 100, 100), "nothing anywhere");
}
