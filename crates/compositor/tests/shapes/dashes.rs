//! A dashed line: the pattern along the outline, the offset that moves it, and
//! a trim that does not drag the dashes along with it.

use scorsese_compositor::Frame;
use scorsese_compositor::shape::{Arrow, Border, Dash, Figure, Outline, Stroking, draw_stroked};
use scorsese_core::{Curve, Heads};

use crate::{BLUE, clear, frame};

/// A plain horizontal line from x = 40 to x = 160, so a dash's position along
/// it is its x less forty.
fn line() -> Figure {
    Figure {
        outline: Outline::Arrow(Arrow {
            from: (40.0, 100.0),
            to: (160.0, 100.0),
            curve: Curve::Straight,
            heads: Heads::None,
        }),
        fill: None,
        border: Some(Border {
            color: BLUE,
            width: 4.0,
        }),
    }
}

fn dashed(pattern: &[f32], offset: f32, trim_start: f32) -> Frame {
    let mut frame = frame();
    let stroking = Stroking {
        trim_start,
        trim_end: 1.0,
        dash: Some(Dash {
            pattern: pattern.to_vec(),
            offset,
        }),
    };
    draw_stroked(&mut frame, &line(), &stroking);
    frame
}

/// Which of the samples along the line, one every five pixels from x = 45,
/// have ink on them.
fn inked(frame: &Frame) -> Vec<bool> {
    (0..23).map(|i| !clear(frame, 45 + i * 5, 100)).collect()
}

#[test]
fn a_pattern_alternates_dashes_and_gaps_from_the_start_of_the_line() {
    let frame = dashed(&[10.0, 10.0], 0.0, 0.0);
    assert!(!clear(&frame, 45, 100), "a dash from 40 to 50");
    assert!(clear(&frame, 55, 100), "a gap from 50 to 60");
    assert!(!clear(&frame, 65, 100), "and a dash again");
}

/// Marching ants: increasing the offset moves every dash toward the line's end.
#[test]
fn an_offset_moves_the_dashes_toward_the_end() {
    let moved = dashed(&[10.0, 10.0], 10.0, 0.0);
    assert!(clear(&moved, 45, 100), "the first dash has moved on");
    assert!(!clear(&moved, 55, 100), "into what was a gap");
    // Half a period, so the direction shows: ten either way looks the same on
    // a pattern twenty long, and five backward would ink 42 and clear 52.
    let nudged = dashed(&[10.0, 10.0], 5.0, 0.0);
    assert!(clear(&nudged, 42, 100), "the line opens on a gap's end");
    assert!(!clear(&nudged, 52, 100), "and the first dash runs 45 to 55");
    // A whole pattern along is where it started.
    assert_eq!(
        inked(&dashed(&[10.0, 10.0], 20.0, 0.0)),
        inked(&dashed(&[10.0, 10.0], 0.0, 0.0))
    );
}

#[test]
fn an_odd_pattern_is_read_twice_over() {
    assert_eq!(
        inked(&dashed(&[10.0], 0.0, 0.0)),
        inked(&dashed(&[10.0, 10.0], 0.0, 0.0))
    );
}

/// Validation refuses these in a document; in memory a pattern that could not
/// break a line draws it solid rather than making it vanish.
#[test]
fn a_pattern_that_could_not_break_a_line_draws_it_solid() {
    for pattern in [&[][..], &[0.0, 10.0], &[-5.0], &[f32::NAN]] {
        let frame = dashed(pattern, 0.0, 0.0);
        assert!(inked(&frame).iter().all(|&ink| ink), "{pattern:?}");
    }
}

/// The dashes belong to the outline: a line drawing itself on lays each one
/// down where it will stay, rather than sliding the pattern as it grows.
#[test]
fn a_trimmed_line_keeps_its_dashes_where_the_whole_line_has_them() {
    let whole = inked(&dashed(&[10.0, 10.0], 3.0, 0.0));
    let trimmed = inked(&dashed(&[10.0, 10.0], 3.0, 0.25));
    // A quarter of 120 is 30: the kept line starts at x = 70, sample 5.
    assert!(
        trimmed[..5].iter().all(|&ink| !ink),
        "nothing before the trim"
    );
    assert_eq!(trimmed[5..], whole[5..]);
}
