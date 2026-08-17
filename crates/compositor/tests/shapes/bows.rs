//! The bow of an S, on both of its axes, and where its ends point.
//!
//! **Both axes, because there are two branches and only one of them runs
//! across.** Which axis an S bows along is inferred from the run — the one its
//! ends are further apart on — so an arrow taller than it is wide goes through
//! arithmetic a horizontal one never reaches, and a mistake in that arithmetic is
//! invisible to a test that only ever draws sideways.
//!
//! **And with heads, because the tangent is the other half of it.** A cubic's
//! direction at each end runs from the end to the control point beside it, not
//! along the straight line between the ends; on a bowed arrow those differ by a
//! visible angle. A head aimed at the chord is the classic bug in this feature,
//! and it looks like a mistake in the diagram rather than in the renderer.

use scorsese_compositor::Frame;
use scorsese_compositor::shape::{Arrow, Border, Figure, Outline, draw};
use scorsese_core::{Curve, Heads};

use crate::extent::assert_extent;
use crate::{BLUE, clear, frame};

const WIDTH: f32 = 8.0;

/// Taller than it is wide, so the bow is along `y`: the run is 80 across and 140
/// down, and each control point sits directly above or below the end it belongs
/// to.
const TALL: ((f32, f32), (f32, f32)) = ((60.0, 30.0), (140.0, 170.0));

/// Wider than it is tall **and running leftwards**, so the bow is along `x` and
/// both tangents point the negative way. A direction taken with the wrong sign
/// puts a head on the far side of the arrow from its own tip, which a rightward
/// arrow could not tell apart from a length.
const WIDE: ((f32, f32), (f32, f32)) = ((160.0, 60.0), (40.0, 140.0));

/// A quarter of the way along, the tall S has already dropped to y ≈ 72 while
/// the straight line between the same two ends is only at y ≈ 52. Measured a
/// quarter along rather than at the midpoint because the two cross there: a cubic
/// with these control points passes exactly through the middle of its own chord.
#[test]
fn a_tall_s_bows_down_the_axis_its_ends_are_furthest_apart_on() {
    let bowed = drawn(TALL, Curve::S, Heads::None);
    let straight = drawn(TALL, Curve::Straight, Heads::None);

    assert!(
        !clear(&bowed, 72, 71),
        "the S has dropped to y ≈ 72 by x ≈ 72"
    );
    assert!(clear(&bowed, 72, 52), "and is nowhere near its own chord");
    assert!(!clear(&straight, 72, 52), "which is where the chord is");
    assert!(clear(&straight, 72, 72), "and not where the S is");

    assert!(
        !clear(&bowed, 128, 128),
        "and it arrives the same way round"
    );
    assert_extent(
        &bowed,
        (56, 30, 144, 169),
        "a bow along y stays inside the run's own box",
    );
}

/// The head is aimed along the curve, and on this arrow that is plainly visible:
/// the tangent at each end is *horizontal* — the control point beside it sits
/// level with it — while the chord runs down and across at 34°. So each head is
/// a triangle laid vertically across the run, and its two far corners are 12.8
/// pixels above and below the line. Aimed at the chord instead, both heads turn
/// and neither corner is inked.
#[test]
fn a_bowed_arrows_heads_are_aimed_along_the_curve_and_not_the_chord() {
    let two = drawn(WIDE, Curve::S, Heads::Both);

    assert!(
        !clear(&two, 70, 150),
        "the head at the end spreads below the line it caps"
    );
    assert!(
        !clear(&two, 130, 49),
        "and the one at the start spreads above it, pointing back the way it came"
    );
    assert_extent(
        &two,
        (40, 47, 159, 152),
        "so the heads, not the curve, are what reaches furthest up and down",
    );
}

fn drawn(ends: ((f32, f32), (f32, f32)), curve: Curve, heads: Heads) -> Frame {
    let mut frame = frame();
    draw(
        &mut frame,
        &Figure {
            outline: Outline::Arrow(Arrow {
                from: ends.0,
                to: ends.1,
                curve,
                heads,
            }),
            fill: None,
            border: Some(Border {
                color: BLUE,
                width: WIDTH,
            }),
        },
    );
    frame
}
