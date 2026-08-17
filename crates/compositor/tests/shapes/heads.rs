//! The head: how far back it reaches, how wide it spreads, and which way it
//! aims.
//!
//! Every assertion here is a *measurement*, because a head the wrong size or
//! aimed the wrong way still puts ink where a sampled pixel would look for it.
//!
//! **And nothing here runs across the frame.** On a horizontal arrow the
//! direction's `y` is zero, so half of the head's arithmetic is multiplied by
//! nothing and any mistake in that half is invisible. A vertical arrow reads the
//! other half; a diagonal reads both at once.

use scorsese_compositor::Frame;
use scorsese_compositor::shape::{Arrow, Border, Figure, Outline, draw};
use scorsese_core::{Curve, Heads};

use crate::extent::{assert_coverage, assert_extent, coverage};
use crate::{BLUE, frame};

/// Thick enough that a head is many pixels wider than the line it caps, so a
/// pixel of rasterising noise is a fraction of a percent of the ink.
const WIDTH: f32 = 8.0;

/// A head reaches four widths back from the tip and spreads 1.6 either side, so
/// at this thickness it is 32 pixels long and 25.6 across at the base.
const LENGTH: f64 = 4.0 * WIDTH as f64;
const SPREAD: f64 = 1.6 * WIDTH as f64;

/// The ink a head adds over the line it caps: its own triangle, less the part of
/// it the stroke had already covered. Half the stroke's thickness is 4, which the
/// head's taper reaches 10 pixels back from the tip — so the overlap is a
/// tapering wedge for those 10 and the full 8 across for the remaining 22.
const HEAD: f64 = SPREAD * LENGTH - (0.4 * 10.0 * 10.0 + 8.0 * 22.0);

/// A hundred and twenty pixels of straight line, eight thick.
const LINE: f64 = 120.0 * WIDTH as f64;

/// A line falling on whole pixels either side, plus a curve's worth for the
/// head's two sloping edges.
const SLACK: f64 = 3.0;

#[test]
fn a_head_reaches_four_widths_back_and_spreads_sixteen_tenths_either_side() {
    let plain = down(Heads::None);
    assert_extent(
        &plain,
        (96, 40, 103, 159),
        "a plain line is its own stroke and nothing else",
    );
    assert_coverage(&plain, LINE, SLACK, "120 down by 8 across");

    let headed = down(Heads::End);
    assert_extent(
        &headed,
        (87, 40, 112, 159),
        "a head spreads 12.8 either side of the line it caps",
    );
    assert_coverage(&headed, LINE + HEAD, SLACK, "the line, and one head on it");
}

/// A head at the start points *back the way the line came*, so on a diagonal it
/// lies along the run exactly as the one at the end does and adds exactly the
/// same ink. That identity is the assertion: it is a statement about the two
/// heads being one head twice, which no measurement of a single one could make,
/// and it is what a half-reversed direction breaks.
#[test]
fn the_head_at_the_start_is_the_one_at_the_end_reversed() {
    let plain = coverage(&across(Heads::None));
    let one = coverage(&across(Heads::End)) - plain;
    let two = coverage(&across(Heads::Both)) - plain;

    assert!(
        (one - HEAD).abs() <= SLACK,
        "one head adds {HEAD} pixels' worth of ink, found {one}"
    );
    assert!(
        (two - 2.0 * one).abs() <= SLACK,
        "two heads are twice one: {two} against {one}"
    );
    assert_extent(
        &across(Heads::Both),
        (47, 47, 152, 152),
        "and both lie along the run rather than out to the side of it",
    );
}

/// Down the middle of the frame, so the direction is `(0, 1)` and every `y` in
/// the head's arithmetic is load-bearing.
fn down(heads: Heads) -> Frame {
    drawn((100.0, 40.0), (100.0, 160.0), heads)
}

/// Corner to corner, so neither component is zero and the two heads are mirror
/// images of one another.
fn across(heads: Heads) -> Frame {
    drawn((50.0, 50.0), (150.0, 150.0), heads)
}

fn drawn(from: (f32, f32), to: (f32, f32), heads: Heads) -> Frame {
    let mut frame = frame();
    draw(
        &mut frame,
        &Figure {
            outline: Outline::Arrow(Arrow {
                from,
                to,
                curve: Curve::Straight,
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
