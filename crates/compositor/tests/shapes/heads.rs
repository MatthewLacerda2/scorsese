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
use scorsese_core::{Curve, Heads, Rgba};

use crate::extent::{assert_coverage, assert_extent, coverage, row};
use crate::{BLUE, frame};

/// Thick enough that a head is many pixels wider than the line it caps, so a
/// pixel of rasterising noise is a fraction of a percent of the ink.
const WIDTH: f32 = 8.0;

/// A head reaches four widths back from the tip and spreads 1.6 either side, so
/// at this thickness it is 32 pixels long and 25.6 across at the base.
const LENGTH: f64 = 4.0 * WIDTH as f64;
const SPREAD: f64 = 1.6 * WIDTH as f64;

/// The ink a head adds over a plain line: its own triangle, less the stretch of
/// line it stands in for. The line stops inside the head (#608), so the whole
/// triangle is ink and the head's full length of line is not — 409.6 less 256 at
/// this thickness, so a head adds 153.6. A line still stroked to the tip would
/// add 193.6 instead, and fail this by far more than the slack.
const HEAD: f64 = SPREAD * LENGTH - WIDTH as f64 * LENGTH;

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
    let pointed = across(Heads::Both);
    let bare = coverage(&across(Heads::None));
    let one = coverage(&across(Heads::End)) - bare;
    let two = coverage(&pointed) - bare;

    // A pixel more slack than down the frame: on the diagonal the head's base
    // and the line's square end inside it are anti-aliased edges too, and they
    // round to 156.7 against 153.6. The line stroked to the tip would be 40 out.
    assert!(
        (one - HEAD).abs() <= SLACK + 1.0,
        "one head adds {HEAD} pixels' worth of ink, found {one}"
    );
    assert!(
        (two - 2.0 * one).abs() <= SLACK,
        "two heads are twice one: {two} against {one} ({HEAD} each)"
    );
    assert_extent(
        &pointed,
        (50, 50, 149, 149),
        "and both end on their tips, along the run rather than out to its side",
    );
}

/// The tip is a point. Two pixels back from it a head is a pixel and a half
/// across, so the column there holds a sliver of ink, not the line's full eight
/// — which is what it held while the line's square end ran under the tip and
/// stood clear of both sloping sides (#608).
#[test]
fn the_line_stops_inside_the_head_so_the_tip_is_a_point() {
    let headed = down(Heads::End);
    let (tip, line) = (row(&headed, 158), row(&headed, 100));
    assert!(tip <= 2.0, "two back from the tip it is {tip} wide");
    assert!((line - f64::from(WIDTH)).abs() <= 0.5, "the line is {line}");
}

/// The line runs one width into the head and stops: far enough that the two
/// overlap rather than meeting edge to edge, which would leave an anti-aliased
/// seam across the line, and no further. Half see-through, the overlap is where
/// the ink lands twice — so down the middle it is a band exactly one width long,
/// just inside the head's base, and nowhere else.
#[test]
fn the_line_overlaps_the_head_by_one_width() {
    let glass = drawn_in(
        (100.0, 40.0),
        (100.0, 160.0),
        Heads::End,
        Rgba::new(0x00, 0x00, 0xff, 0x80),
    );
    let twice: Vec<u32> = (0..crate::SIDE)
        .filter(|&y| crate::at(&glass, 100, y).3 > 0xa0)
        .collect();
    let base = 160 - LENGTH as u32;
    let band: Vec<u32> = (base..base + WIDTH as u32).collect();
    assert_eq!(twice, band, "inked twice: the width behind the head's base");
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
    drawn_in(from, to, heads, BLUE)
}

fn drawn_in(from: (f32, f32), to: (f32, f32), heads: Heads, color: Rgba) -> Frame {
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
                color,
                width: WIDTH,
            }),
        },
    );
    frame
}
