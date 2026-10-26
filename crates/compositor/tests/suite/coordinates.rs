//! The coordinate ruler: where its lines land, and what it leaves alone.
//!
//! A ruler nobody can trust is worse than none, because a number read off a
//! wrong one is written into the document and believed. So what is asserted
//! here is the arithmetic — a line marking `0.3` is at three tenths of the
//! width, the middle one is the heavy one — and never how a glyph came out,
//! which belongs to the text pipeline and to the golden renders.

use scorsese_compositor::{Frame, Resolution, grid};
use scorsese_core::Rgba;

/// The frame everything below is drawn onto: mid-grey, so a pale rule and the
/// dark edging under it both show as a change.
const GREY: Rgba = Rgba::new(0x80, 0x80, 0x80, 0xff);

/// A raster big enough that a tenth of it is many pixels, and square so that
/// the two axes are read the same way.
const SIDE: u32 = 400;

fn grey() -> Frame {
    let mut frame = Frame::black(Resolution::new(SIDE, SIDE).expect("a square raster"));
    frame.fill(GREY);
    frame
}

/// Whether the pixel at `(x, y)` is still the grey it started as.
fn untouched(frame: &Frame, x: u32, y: u32) -> bool {
    let at = ((y * frame.resolution().width() + x) * 4) as usize;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2]) == (GREY.r, GREY.g, GREY.b)
}

/// How many pixels either side of `x` are marked, along row `y`.
///
/// A run rather than a single pixel: a rule is drawn with a dark edging around
/// a light core, and how wide the whole mark is is exactly what tells a heavy
/// line from an ordinary one.
fn run(frame: &Frame, y: u32, x: u32) -> u32 {
    let mut width = 0;
    for step in 0..20 {
        if !untouched(frame, x - step, y) {
            width += 1;
        }
        if step > 0 && !untouched(frame, x + step, y) {
            width += 1;
        }
    }
    width
}

/// The whole claim: the line labelled `0.3` is at three tenths across.
///
/// Sampled well away from the top and left edges, where the labels are, and
/// from the horizontal rules, which would answer for themselves.
#[test]
fn a_rule_lands_on_the_fraction_it_marks() {
    let mut frame = grey();
    grid::draw(&mut frame);

    for tenth in 1..10 {
        let x = SIDE * tenth / 10;
        assert!(
            !untouched(&frame, x, 148),
            "nothing is drawn at 0.{tenth} across"
        );
    }
    assert!(
        untouched(&frame, 140, 148),
        "the middle of a cell is ruled, and a cell has no line in it"
    );
}

/// Both axes, from the same drawing, in the same units.
#[test]
fn the_horizontal_rules_are_the_same_fractions_down() {
    let mut frame = grey();
    grid::draw(&mut frame);

    for tenth in 1..10 {
        let y = SIDE * tenth / 10;
        assert!(
            !untouched(&frame, 148, y),
            "nothing is drawn at 0.{tenth} down"
        );
    }
}

/// `0.5` is drawn heavier, which is what makes the middle of the frame findable
/// without counting lines.
#[test]
fn the_middle_rule_is_the_heavy_one() {
    let mut frame = grey();
    grid::draw(&mut frame);

    let middle = run(&frame, 148, SIDE / 2);
    let ordinary = run(&frame, 148, SIDE * 3 / 10);
    assert!(
        middle > ordinary,
        "0.5 came out {middle} pixels wide and 0.3 came out {ordinary} — the \
         heavy line is not heavier"
    );
}

/// Drawing is asked for, never assumed: an untouched frame is what every render
/// gets, and the ruler is furniture for looking at one.
#[test]
fn a_frame_nobody_ruled_is_the_frame_it_was() {
    let frame = grey();
    assert!(untouched(&frame, SIDE / 2, SIDE / 2));
    assert!(untouched(&frame, 0, 0));
}
