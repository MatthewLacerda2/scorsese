//! Text longer than the box it was given: where it breaks.

use scorsese_compositor::text::{self, Font, Style};
use scorsese_core::Rgba;

use crate::ink::{self, WIDTH, canvas, style};

/// Long enough to wrap several times at the sizes below, and made of ordinary
/// words so the breaks land between them.
pub(crate) const SENTENCE: &str =
    "The quick brown fox jumps over the lazy dog and keeps going well past the frame";

#[test]
fn a_line_too_long_for_the_box_wraps_instead_of_running_off() {
    let mut frame = canvas();
    text::draw(
        &mut frame,
        SENTENCE,
        Font::sans(),
        &style(20.0, Rgba::WHITE),
    );

    assert!(ink::lines(&frame) > 1, "a sentence this long takes lines");
    let (left, _, right, _) = ink::bounds(&frame).expect("something was drawn");
    assert!(
        right < WIDTH && left > 0,
        "wrapped text stays inside the frame; found {left}..{right}"
    );
}

#[test]
fn a_narrower_box_takes_more_lines() {
    let wide = style(20.0, Rgba::WHITE);
    let narrow = Style {
        max_width: wide.max_width / 2.0,
        ..wide
    };

    let mut loose = canvas();
    text::draw(&mut loose, SENTENCE, Font::sans(), &wide);
    let mut tight = canvas();
    text::draw(&mut tight, SENTENCE, Font::sans(), &narrow);

    assert!(
        ink::lines(&tight) > ink::lines(&loose),
        "halving the wrap width has to cost lines: {} then {}",
        ink::lines(&loose),
        ink::lines(&tight)
    );
    let (left, _, right, _) = ink::bounds(&tight).expect("something was drawn");
    assert!(
        f32::from(u16::try_from(right - left).expect("a raster this size")) <= narrow.max_width,
        "the narrower box is what the text was held to; found {left}..{right}"
    );
}

/// A word with nothing to break at is broken between characters rather than
/// left to run off the side. A URL is the case that bites.
#[test]
fn an_unbreakable_word_is_broken_anyway() {
    let mut frame = canvas();
    text::draw(
        &mut frame,
        "Supercalifragilisticexpialidocious",
        Font::sans(),
        &style(28.0, Rgba::WHITE),
    );

    assert!(ink::lines(&frame) > 1, "one long word still has to wrap");
    let (_, _, right, _) = ink::bounds(&frame).expect("something was drawn");
    assert!(right < WIDTH, "and stays on the frame; found {right}");
}

#[test]
fn a_newline_in_the_content_is_a_line_break() {
    let mut broken = canvas();
    text::draw(
        &mut broken,
        "one\ntwo",
        Font::sans(),
        &style(24.0, Rgba::WHITE),
    );
    let mut joined = canvas();
    text::draw(
        &mut joined,
        "one two",
        Font::sans(),
        &style(24.0, Rgba::WHITE),
    );

    assert_eq!(ink::lines(&broken), 2, "the author's own break is kept");
    assert_eq!(ink::lines(&joined), 1, "and is the only reason to break");
}

/// Alignment is a property like any other, and the three settings have to
/// actually differ.
#[test]
fn the_three_alignments_put_a_short_line_in_three_places() {
    let left_edge = |align| {
        let mut frame = canvas();
        text::draw(
            &mut frame,
            "hi",
            Font::sans(),
            &Style {
                align,
                ..style(24.0, Rgba::WHITE)
            },
        );
        ink::bounds(&frame).expect("something was drawn").0
    };

    let (left, centre, right) = (
        left_edge(scorsese_core::TextAlign::Left),
        left_edge(scorsese_core::TextAlign::Center),
        left_edge(scorsese_core::TextAlign::Right),
    );
    assert!(
        left < centre && centre < right,
        "left, centred and right are three distinct places; found {left}, {centre}, {right}"
    );
}

/// A line of emoji is held to the box exactly as a line of words is.
///
/// The wrapper asks the chain for its widths, so a glyph the named face never
/// had is as wide to the measuring as it is on the frame. Measured against the
/// named face alone, every one of these would be nothing at all — the line
/// would never look too long, would never be broken, and would be drawn running
/// straight out of the box the document asked for. That is the one way
/// measuring and drawing can still disagree once there is more than one face,
/// and it is silent: no error, just a title over the edge of the picture.
#[test]
fn a_line_of_emoji_is_wrapped_to_the_width_it_is_drawn_at() {
    let boxed = Style {
        max_width: 100.0,
        line_height: 40.0,
        ..style(24.0, Rgba::WHITE)
    };
    let mut frame = canvas();
    text::draw(&mut frame, "🔥🔥🔥🔥🔥🔥", Font::sans(), &boxed);

    assert!(
        ink::lines(&frame) > 1,
        "six fires are far wider than a 100px box, so they have to break"
    );
    let (left, _, right, _) = ink::bounds(&frame).expect("the fires were drawn");
    assert!(
        f32::from(u16::try_from(right - left).expect("a raster this size")) <= boxed.max_width,
        "what was drawn has to be what was measured: {left}..{right} in a \
         {}px box",
        boxed.max_width
    );
}
