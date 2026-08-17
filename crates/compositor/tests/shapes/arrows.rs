//! Arrows: where the line runs, and which end is pointed.
//!
//! A head is measured rather than looked at. Ink counted down a column tells a
//! plain line from a headed one — four pixels against thirteen — where sampling
//! a single pixel would find both of them inked and say nothing. The golden
//! fixture is what judges whether a head *looks* right.

use scorsese_compositor::Frame;
use scorsese_compositor::shape::{Arrow, Border, Figure, Outline, draw};
use scorsese_core::{Curve, Heads};

use crate::extent::extent;
use crate::{BLUE, SIDE, at, clear, frame};

/// Thick enough that a head is many pixels wider than the line it caps.
const WIDTH: f32 = 4.0;

/// A horizontal run across the middle, so a head's spread is a column of ink
/// and the arithmetic is readable by hand.
const FROM: (f32, f32) = (40.0, 100.0);
const TO: (f32, f32) = (160.0, 100.0);

fn arrow(from: (f32, f32), to: (f32, f32), curve: Curve, heads: Heads) -> Figure {
    Figure {
        outline: Outline::Arrow(Arrow {
            from,
            to,
            curve,
            heads,
        }),
        fill: None,
        border: Some(Border {
            color: BLUE,
            width: WIDTH,
        }),
    }
}

fn drawn(figure: &Figure) -> Frame {
    let mut frame = frame();
    draw(&mut frame, figure);
    frame
}

/// How many pixels of the column at `x` have any ink in them — the measure that
/// separates a line from a head, since both are inked at their centre.
fn column(frame: &Frame, x: u32) -> u32 {
    (0..SIDE).filter(|&y| at(frame, x, y).3 > 0).count() as u32
}

#[test]
fn a_straight_arrow_runs_between_its_ends_and_nowhere_else() {
    let frame = drawn(&arrow(FROM, TO, Curve::Straight, Heads::None));

    assert!(!clear(&frame, 100, 100), "the middle of the run");
    assert!(clear(&frame, 30, 100), "before the start");
    assert!(clear(&frame, 170, 100), "past the end");
    assert!(clear(&frame, 100, 130), "well off the line");
}

/// The head is the difference between an arrow and a line, and it is at the end
/// it points at — not at both, and not at the wrong one.
#[test]
fn a_head_widens_the_end_it_points_at_and_leaves_the_other_alone() {
    let plain = drawn(&arrow(FROM, TO, Curve::Straight, Heads::None));
    let headed = drawn(&arrow(FROM, TO, Curve::Straight, Heads::End));

    // Ten pixels back from each end: inside a head, which reaches four widths.
    let (near_end, near_start) = (150, 50);
    assert!(
        column(&headed, near_end) > column(&plain, near_end) + 2,
        "the pointed end is wider than a plain line: {} vs {}",
        column(&headed, near_end),
        column(&plain, near_end)
    );
    assert_eq!(
        column(&headed, near_start),
        column(&plain, near_start),
        "and the other end is untouched"
    );
}

#[test]
fn both_heads_points_the_start_as_well() {
    let one = drawn(&arrow(FROM, TO, Curve::Straight, Heads::End));
    let two = drawn(&arrow(FROM, TO, Curve::Straight, Heads::Both));

    assert!(
        column(&two, 50) > column(&one, 50) + 2,
        "the start is now pointed too"
    );
    assert_eq!(column(&two, 150), column(&one, 150), "the end is unchanged");
}

/// An S leaves its start along the axis the two ends are furthest apart on, so
/// it hangs back towards that start where a straight line has already set off
/// diagonally. This is the assertion that a bowed arrow is actually bowed — and
/// it is made at a fifth along rather than at the midpoint, because the two
/// cross there: a cubic with these control points passes exactly through the
/// middle of the straight line between its ends, so a test sampling the centre
/// would find both inked and prove nothing.
#[test]
fn an_s_curve_hangs_back_where_a_straight_line_has_already_set_off() {
    let (from, to) = ((40.0, 60.0), (160.0, 140.0));
    let bowed = drawn(&arrow(from, to, Curve::S, Heads::None));
    let straight = drawn(&arrow(from, to, Curve::Straight, Heads::None));

    // At x = 70 the straight line is a quarter of the way down, at y = 80. The
    // S is still at y ≈ 68, twelve pixels behind it — comfortably more than the
    // four the line is thick.
    assert!(!clear(&bowed, 70, 68), "the S is still climbing gently");
    assert!(
        clear(&bowed, 70, 80),
        "and has not reached the straight line"
    );
    assert!(
        !clear(&straight, 70, 80),
        "which is a quarter of the way down"
    );
    assert!(clear(&straight, 70, 68), "and nowhere near the S");
}

/// Both ends in the same place has no direction to draw along and none to aim a
/// head down. Validation refuses it in a document; in memory it draws nothing
/// rather than dividing by a zero-length run.
#[test]
fn an_arrow_with_no_length_draws_nothing() {
    let frame = drawn(&arrow(FROM, FROM, Curve::Straight, Heads::End));
    assert_eq!(
        extent(&frame),
        None,
        "no ink anywhere, not merely none at the point the two ends share"
    );
}

/// An arrow is its stroke, so a width that is not a width leaves an empty frame.
/// The check is this module's own — a closed shape reaches the one in `paint`
/// instead — and it has to cover the head as much as the line: a **negative**
/// width runs the head's arithmetic backwards, which loses the line and draws a
/// mirrored triangle behind the tip instead of nothing at all.
#[test]
fn an_arrow_whose_border_has_no_width_draws_nothing() {
    for width in [0.0, -4.0, f32::NAN, f32::INFINITY] {
        let mut figure = arrow(FROM, TO, Curve::Straight, Heads::Both);
        figure.border = Some(Border { color: BLUE, width });
        assert_eq!(
            extent(&drawn(&figure)),
            None,
            "a border {width} wide is not a border"
        );
    }
}

/// A line is its stroke and nothing else, so there is no second way for one to
/// reach the frame — a `fill` on an arrow is refused by validation, and ignored
/// here.
#[test]
fn an_arrow_without_a_border_draws_nothing() {
    let mut figure = arrow(FROM, TO, Curve::Straight, Heads::End);
    figure.border = None;
    figure.fill = Some(BLUE);
    let frame = drawn(&figure);

    assert!(clear(&frame, 100, 100), "nothing along the run");
    assert!(
        clear(&frame, 158, 100),
        "and nothing where the head would be"
    );
}
