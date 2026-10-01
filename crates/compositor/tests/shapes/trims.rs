//! A trimmed line: only the kept stretch of the outline is stroked, the head
//! rides the drawn end, and a fill is never trimmed.

use scorsese_compositor::Frame;
use scorsese_compositor::shape::{Arrow, Border, Figure, Outline, Stroking, draw, draw_stroked};
use scorsese_core::{Curve, Heads};

use crate::extent::extent;
use crate::{BLUE, RED, SIDE, at, bounds, centred, clear, filled, frame};

const WIDTH: f32 = 4.0;

fn arrow(heads: Heads) -> Figure {
    Figure {
        outline: Outline::Arrow(Arrow {
            from: (40.0, 100.0),
            to: (160.0, 100.0),
            curve: Curve::Straight,
            heads,
        }),
        fill: None,
        border: Some(Border {
            color: BLUE,
            width: WIDTH,
        }),
    }
}

fn trimmed(figure: &Figure, trim_start: f32, trim_end: f32) -> Frame {
    let mut frame = frame();
    let stroking = Stroking {
        trim_start,
        trim_end,
        dash: None,
    };
    draw_stroked(&mut frame, figure, &stroking);
    frame
}

fn column(frame: &Frame, x: u32) -> usize {
    (0..SIDE).filter(|&y| at(frame, x, y).3 > 0).count()
}

#[test]
fn trimming_the_end_draws_the_line_from_its_start_only_as_far_as_asked() {
    let half = trimmed(&arrow(Heads::None), 0.0, 0.5);
    assert!(!clear(&half, 60, 100), "the first half is drawn");
    assert!(clear(&half, 120, 100), "the second is not yet");

    let erased = trimmed(&arrow(Heads::None), 0.5, 1.0);
    assert!(clear(&erased, 60, 100), "trimming the start erases forward");
    assert!(!clear(&erased, 120, 100));
}

/// The decision the issue asked for: a drawing-on arrow is led by its head,
/// and a head never waits at `to` for a line that has not reached it.
#[test]
fn the_head_rides_the_trimmed_end_and_is_never_left_at_the_tip() {
    let half = trimmed(&arrow(Heads::End), 0.0, 0.5);
    // The line reaches x = 100; the head reaches four widths back from there.
    assert!(
        column(&half, 90) > column(&half, 60) + 2,
        "a head at the drawn end"
    );
    assert!(clear(&half, 150, 100), "and nothing at all where `to` is");
    assert_eq!(column(&half, 150), 0);

    let both = trimmed(&arrow(Heads::Both), 0.5, 1.0);
    assert!(
        column(&both, 110) > column(&both, 130) + 2,
        "the start head moved too"
    );
}

#[test]
fn trimmed_to_nothing_draws_nothing_not_even_a_head() {
    for (start, end) in [(0.0, 0.0), (0.6, 0.4), (1.0, 1.0)] {
        let frame = trimmed(&arrow(Heads::Both), start, end);
        assert_eq!(extent(&frame), None, "{start}..{end}");
    }
}

/// An easing that overshoots is an ordinary animation, so a trim past either
/// end is the end, not a refusal and not a line running off the outline.
#[test]
fn a_trim_outside_the_outline_is_clamped_to_it() {
    let mut whole = frame();
    draw(&mut whole, &arrow(Heads::End));
    for (start, end) in [(-0.3, 1.0), (0.0, 1.4), (-1.0, 2.0), (f32::NAN, f32::NAN)] {
        let frame = trimmed(&arrow(Heads::End), start, end);
        assert_eq!(frame.bytes(), whole.bytes(), "{start}..{end}");
    }
}

/// The trim is the line's alone: a filled box trimmed to nothing is still a
/// filled box.
#[test]
fn a_trim_leaves_the_fill_whole_and_cuts_the_border_clockwise_from_the_top_left() {
    let square = Figure {
        outline: Outline::Rectangle {
            bounds: bounds((100.0, 100.0), centred()),
            radius: 0.0,
        },
        fill: Some(RED.into()),
        border: Some(Border {
            color: BLUE,
            width: WIDTH,
        }),
    };
    let unbordered = trimmed(&square, 0.0, 0.0);
    assert!(filled(&unbordered, 100, 100), "the fill is untouched");
    assert!(
        filled(&unbordered, 51, 100),
        "and there is no border over it"
    );

    let quarter = trimmed(&square, 0.0, 0.25);
    assert_eq!(
        at(&quarter, 100, 50),
        (0, 0, 255, 255),
        "the top edge is drawn"
    );
    assert!(filled(&quarter, 149, 100), "the right edge is not yet");
}

/// Drawn on less far than the line stops short of its head, an arrow is a head
/// and no line yet (#608). Six pixels in, at this thickness the line would stop
/// twelve back from the tip — so nothing of it is stroked, and two pixels behind
/// the tip the column is the head's sliver, not the line's four.
#[test]
fn drawn_on_less_than_a_head_it_is_a_head_and_no_line() {
    let early = trimmed(&arrow(Heads::End), 0.0, 0.05);
    assert!(!clear(&early, 40, 100), "the head is there");
    assert!(column(&early, 44) <= 2, "{} high", column(&early, 44));
}
