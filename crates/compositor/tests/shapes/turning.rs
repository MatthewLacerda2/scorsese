//! Measuring an outline at its awkward places: exactly on a corner, across
//! one, and at a size no length fits in.
//!
//! `along` covers the ordinary walk. These are the edges of it — the ones a
//! test pinned to the middle of an edge never touches.

use scorsese_compositor::shape::{Border, Figure, Outline, Stroking, draw_stroked, measure};
use scorsese_compositor::{Frame, Resolution};

use crate::{BLUE, SIDE, bounds, centred, clear, frame};

fn raster() -> Resolution {
    Resolution::new(SIDE, SIDE).expect("a square raster")
}

/// A hundred-pixel square from (50, 50) to (150, 150): 400 round, so a quarter
/// of the way is exactly its top-right corner.
fn square() -> Outline {
    Outline::Rectangle {
        bounds: bounds((100.0, 100.0), centred()),
        radius: 0.0,
    }
}

/// The documented rule: the tip of a line trimmed to end on a corner points the
/// way the line was going, not the way it is about to turn.
#[test]
fn exactly_on_a_corner_the_heading_is_the_one_it_arrives_with() {
    let measured = measure(&square(), raster()).expect("a square has length");
    let corner = measured.at(0.25);
    assert_eq!(corner.position, (150.0, 50.0));
    assert_eq!(corner.tangent, (1.0, 0.0), "still running along the top");
    assert_eq!(measured.at(0.5).tangent, (0.0, 1.0), "down the right side");
}

/// A trim that reaches past a corner goes round it. Cutting across instead
/// would draw a diagonal through the middle of the box.
#[test]
fn a_trim_across_a_corner_goes_round_it() {
    let mut drawn = frame();
    let figure = Figure {
        outline: square(),
        fill: None,
        border: Some(Border {
            color: BLUE,
            width: 4.0,
        }),
    };
    let half = Stroking {
        trim_start: 0.125,
        trim_end: 0.625,
        ..Stroking::WHOLE
    };
    draw_stroked(&mut drawn, &figure, &half);
    let inked = |frame: &Frame, x, y| !clear(frame, x, y);
    assert!(inked(&drawn, 140, 50), "the second half of the top");
    assert!(inked(&drawn, 150, 100), "all of the right side");
    assert!(inked(&drawn, 140, 150), "the first half of the bottom");
    assert!(clear(&drawn, 100, 100), "and nothing through the middle");
}

/// Every edge of this box is a finite number of pixels, and their sum is not.
/// A length that is infinity would make every fraction of it infinity too.
#[test]
fn an_outline_too_long_to_measure_is_not_measured() {
    let vast = Outline::Rectangle {
        bounds: bounds((3.0e38, 3.0e38), centred()),
        radius: 0.0,
    };
    assert_eq!(measure(&vast, raster()), None);
}
