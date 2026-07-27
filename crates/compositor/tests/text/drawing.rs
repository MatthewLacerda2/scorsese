//! The primitive: a string, at a size, in a colour, at a position.

use scorsese_compositor::text::{self, Font};
use scorsese_core::Rgba;

use crate::ink::{self, HEIGHT, WIDTH, canvas, style};

#[test]
fn a_string_marks_the_frame() {
    let mut frame = canvas();
    text::draw(
        &mut frame,
        "Chapter One",
        Font::sans(),
        &style(28.0, Rgba::WHITE),
    );

    assert!(ink::count(&frame) > 100, "a title should cover real area");
    let (left, top, right, bottom) = ink::bounds(&frame).expect("something was drawn");
    assert!(
        right < WIDTH && bottom < HEIGHT,
        "the ink stays on the frame"
    );
    assert!(
        left > 0 && top > 0,
        "the ink is inset, not against the edge"
    );
}

#[test]
fn an_empty_string_draws_nothing() {
    let mut frame = canvas();
    text::draw(&mut frame, "", Font::sans(), &style(28.0, Rgba::WHITE));
    assert_eq!(ink::count(&frame), 0, "nothing to say, nothing to draw");
}

/// The generality rule made testable: the colour drawn is the colour the
/// document asked for, and nothing here has an opinion about which it is.
#[test]
fn the_colour_asked_for_is_the_colour_drawn() {
    for colour in [
        Rgba::opaque(255, 0, 0),
        Rgba::opaque(0, 128, 255),
        Rgba::opaque(12, 200, 60),
    ] {
        let mut frame = canvas();
        text::draw(&mut frame, "Colour", Font::sans(), &style(40.0, colour));
        let solid = ink::most_solid(&frame);
        assert_eq!(
            (solid.0, solid.1, solid.2),
            (colour.r, colour.g, colour.b),
            "a fully covered pixel is the colour asked for, not a stand-in"
        );
        assert_eq!(solid.3, u8::MAX, "an opaque colour draws opaque glyphs");
    }
}

/// A see-through caption is a colour with alpha in it, not a second mechanism.
#[test]
fn alpha_in_the_colour_carries_through_to_the_pixels() {
    let mut frame = canvas();
    text::draw(
        &mut frame,
        "Faint",
        Font::sans(),
        &style(40.0, Rgba::new(255, 255, 255, 128)),
    );
    let solid = ink::most_solid(&frame);
    assert!(
        (120..=136).contains(&solid.3),
        "half-transparent text draws at about half alpha, found {}",
        solid.3
    );
}

/// The primitive positions by baseline, which is the horizontal that stays put
/// as the characters change.
#[test]
fn the_primitive_sits_the_text_on_the_baseline_it_is_given() {
    let baseline = 120.0;
    let mut frame = canvas();
    text::draw_line(
        &mut frame,
        "Hello",
        Font::sans(),
        30.0,
        Rgba::WHITE,
        (40.0, baseline),
    );

    let (left, _, _, bottom) = ink::bounds(&frame).expect("something was drawn");
    assert!(
        (118..=121).contains(&bottom),
        "no letter of `Hello` descends, so the ink ends at the baseline; found {bottom}"
    );
    assert!(
        (38..=44).contains(&left),
        "the pen starts where it was put; found {left}"
    );
}

/// Size is a property, so asking for more of it gets more of it.
#[test]
fn a_bigger_size_draws_bigger_text() {
    let mut small = canvas();
    text::draw(&mut small, "Size", Font::sans(), &style(16.0, Rgba::WHITE));
    let mut large = canvas();
    text::draw(&mut large, "Size", Font::sans(), &style(48.0, Rgba::WHITE));

    let height = |frame| {
        let (_, top, _, bottom) = ink::bounds(frame).expect("something was drawn");
        bottom - top
    };
    assert!(
        height(&large) > height(&small) * 2,
        "tripling the size roughly triples the letters"
    );
}
