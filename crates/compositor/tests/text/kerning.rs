//! Whether the pen's step from one glyph to the next depends on the pair.
//!
//! Which is all kerning is. The shapes it produces are the golden fixtures'
//! business — a picture is the only honest reference for a letterform — but
//! *that* the spacing responds to what follows a glyph is a fact about
//! numbers, and a fact a test can hold on its own.

use scorsese_compositor::text::{self, Font};
use scorsese_core::Rgba;

use crate::ink::{self, canvas};

/// Big enough that a pair adjustment of a few hundredths of an em is several
/// whole pixels, so nothing here rests on a single row of antialiasing.
const SIZE: f32 = 96.0;

/// The pen starts well inside the frame so that no glyph is clipped by an
/// edge, which would put the ink bound somewhere the letter did not decide.
const ORIGIN: (f32, f32) = (20.0, 140.0);

/// How far right the ink of `text` reaches, set from a fixed origin.
fn right_edge(text: &str) -> f32 {
    let mut frame = canvas();
    text::draw_line(&mut frame, text, Font::sans(), SIZE, Rgba::WHITE, ORIGIN);
    let (_, _, right, _) = ink::bounds(&frame).expect("these letters draw something");
    right as f32
}

/// How far the pen moved over the first glyph of `pair`, whose second glyph is
/// `second` set on its own.
///
/// The two frames end with the same letter, so its own ink is the same width
/// in both and cancels: what is left is the step the first glyph took, its
/// pair adjustment included.
fn step(pair: &str, second: &str) -> f32 {
    right_edge(pair) - right_edge(second)
}

/// `AV` is the pair every specimen sheet opens with: the two diagonals nest,
/// and a face that says so spaces them tighter than it spaces `A` before a
/// letter it has nothing to say about. Without kerning both steps are one
/// advance width and this reads equal.
#[test]
fn a_face_spaces_a_pair_by_what_follows_it() {
    let kerned = step("AV", "V");
    let plain = step("AH", "H");
    assert!(
        kerned < plain - 1.0,
        "`AV` should set tighter than `AH`; steps were {kerned} and {plain}"
    );
}

/// The same, in the other direction and in the other shipped face — a title in
/// serif is kerned because the face is read for it, not because sans was
/// special-cased somewhere.
#[test]
fn the_serif_face_kerns_too() {
    let right_edge = |text: &str| {
        let mut frame = canvas();
        text::draw_line(&mut frame, text, Font::serif(), SIZE, Rgba::WHITE, ORIGIN);
        let (_, _, right, _) = ink::bounds(&frame).expect("these letters draw something");
        right as f32
    };
    let kerned = right_edge("Yo") - right_edge("o");
    let plain = right_edge("Ho") - right_edge("o");
    assert!(
        kerned < plain - 1.0,
        "`Yo` should set tighter than `Ho`; steps were {kerned} and {plain}"
    );
}

/// A shaper hands back `.notdef` — a hollow box in most faces — for a
/// character the font cannot set, and printing that would be the renderer
/// guessing out loud. Taking no width and drawing nothing is what "this face
/// cannot say that" has looked like here since text was first drawn; shaping
/// is no reason for it to start printing boxes.
#[test]
fn a_character_the_face_cannot_set_still_draws_nothing() {
    let mut frame = canvas();
    text::draw_line(
        &mut frame,
        "日本語",
        Font::sans(),
        SIZE,
        Rgba::WHITE,
        ORIGIN,
    );
    assert_eq!(
        ink::count(&frame),
        0,
        "a face without these glyphs should set them as nothing, not as boxes"
    );
}
