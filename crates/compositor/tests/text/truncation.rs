//! Text longer than the frame is tall: where it stops, and how it says so.

use scorsese_compositor::text::{self, Font, Style};
use scorsese_compositor::{Frame, Resolution};
use scorsese_core::{Rgba, TextAlign};

use crate::ink::{self, HEIGHT, canvas, style};
use crate::wrapping::SENTENCE;

/// A frame with room for exactly one line, so anything after the first is
/// dropped and the ellipsis has to appear.
fn one_line_frame() -> Frame {
    let mut frame = Frame::black(Resolution::new(200, 40).expect("a legal raster"));
    frame.fill_transparent();
    frame
}

fn narrow() -> Style {
    Style {
        anchor: scorsese_core::Anchor::default(),
        size: 20.0,
        color: Rgba::WHITE,
        align: TextAlign::Center,
        line_height: 30.0,
        max_width: 90.0,
    }
}

fn drawn(text: &str) -> Frame {
    let mut frame = one_line_frame();
    text::draw(&mut frame, text, Font::sans(), &narrow());
    frame
}

/// More text than the frame is tall stops inside the picture rather than
/// running off the bottom of it, which would look like a render bug.
#[test]
fn text_taller_than_the_frame_stops_inside_it() {
    let mut frame = canvas();
    text::draw(
        &mut frame,
        &format!("{SENTENCE} {SENTENCE} {SENTENCE} {SENTENCE}"),
        Font::sans(),
        &style(24.0, Rgba::WHITE),
    );

    let (_, top, _, bottom) = ink::bounds(&frame).expect("something was drawn");
    assert!(
        bottom < HEIGHT && top > 0,
        "truncated text is fully on the frame; found {top}..{bottom}"
    );
}

/// The ellipsis is what tells a viewer there was more. Without it, truncation
/// reads as a sentence that simply ended where the author meant it to.
///
/// Asserted by pixels rather than by inspecting a private layout: the frame
/// drawn from the long string has to be the frame drawn from the surviving
/// line with an ellipsis on it, exactly.
#[test]
fn what_does_not_fit_is_replaced_by_an_ellipsis() {
    let truncated = drawn("alpha beta gamma");
    assert_eq!(
        ink::lines(&truncated),
        1,
        "a frame this short holds one line"
    );
    assert_eq!(
        truncated.bytes(),
        drawn("alpha…").bytes(),
        "the surviving line ends in an ellipsis, not in mid-sentence"
    );
    assert_ne!(
        truncated.bytes(),
        drawn("alpha").bytes(),
        "and the ellipsis is really drawn, not merely intended"
    );
}

/// The ellipsis is fitted to the box, not appended past the edge of it.
#[test]
fn the_ellipsis_stays_inside_the_wrap_width() {
    let truncated = drawn("alphabetagamma delta");
    let (left, _, right, _) = ink::bounds(&truncated).expect("something was drawn");
    assert!(
        f32::from(u16::try_from(right - left).expect("a raster this size")) <= narrow().max_width,
        "an ellipsised line is still held to the wrap width; found {left}..{right}"
    );
}
