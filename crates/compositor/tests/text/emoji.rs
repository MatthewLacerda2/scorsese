//! Sequences: several characters that are one drawing.
//!
//! Its own file because this is the requirement that **half-works silently**.
//! 👍🏽 is a thumb plus a skin-tone modifier and 👨‍👩‍👧 is three people joined by
//! zero-width joiners; the font resolves each into one glyph, and it can only
//! do that if the whole sequence is shaped as one run. Split between the
//! characters and what appears is a thumb followed by a bare colour swatch, or
//! three separate people — output nobody would call an error and everybody
//! would call wrong.
//!
//! **So the measurement is width, not ink.** One glyph is about one em wide;
//! the broken version is two or three. Nothing else tells them apart from the
//! outside.

use scorsese_compositor::text::{self, Font};
use scorsese_core::Rgba;

use crate::ink::{bounds, canvas};

const SIZE: f32 = 32.0;

/// How wide the ink of one line is, drawn on its own.
fn drawn_width(content: &str) -> u32 {
    let mut frame = canvas();
    text::draw_line(
        &mut frame,
        content,
        Font::sans(),
        SIZE,
        Rgba::WHITE,
        (8.0, 80.0),
    );
    let (left, _, right, _) = bounds(&frame).unwrap_or_else(|| panic!("`{content}` drew nothing"));
    right - left + 1
}

/// The most solid colour on the frame, which for an emoji is the body of it.
fn body(content: &str) -> (u8, u8, u8) {
    let mut frame = canvas();
    text::draw_line(
        &mut frame,
        content,
        Font::sans(),
        SIZE,
        Rgba::WHITE,
        (8.0, 80.0),
    );
    let mut tally: std::collections::HashMap<(u8, u8, u8), usize> =
        std::collections::HashMap::new();
    for y in 0..frame.resolution().height() {
        for x in 0..frame.resolution().width() {
            let (r, g, b, a) = crate::ink::pixel(&frame, x, y);
            if a == u8::MAX {
                *tally.entry((r, g, b)).or_default() += 1;
            }
        }
    }
    tally
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(colour, _)| colour)
        .unwrap_or_else(|| panic!("`{content}` painted nothing solid"))
}

#[test]
fn a_skin_tone_is_one_glyph_and_not_a_hand_beside_a_swatch() {
    let plain = drawn_width("👍");
    let toned = drawn_width("👍🏽");
    assert!(
        toned.abs_diff(plain) <= 2,
        "👍🏽 must set as wide as 👍 — one glyph, not two. 👍 is {plain}px, \
         👍🏽 is {toned}px"
    );
    // And the pair really is two glyphs, so the comparison above has teeth: if
    // the modifier had been shaped on its own, `toned` would look like this.
    let pair = drawn_width("👍👍");
    assert!(
        pair > plain + plain / 2,
        "two thumbs should set nearly twice as wide: {pair}px against {plain}px"
    );
}

#[test]
fn the_skin_tone_actually_changes_the_colour() {
    // Same width is necessary and not sufficient: a modifier the shaper dropped
    // entirely would also measure right. The hand has to come out a different
    // colour, which is the whole of what the modifier says.
    assert_ne!(
        body("👍"),
        body("👍🏽"),
        "the modifier chooses a different drawing of the same hand"
    );
}

#[test]
fn a_joined_family_is_one_glyph_and_not_three_people() {
    let family = drawn_width("👨\u{200d}👩\u{200d}👧");
    let one = drawn_width("👨");
    let three = drawn_width("👨👩👧");
    assert!(
        family < three - one,
        "👨‍👩‍👧 is one ligature, so it must set far narrower than three separate \
         people: joined is {family}px, separate is {three}px"
    );
    assert!(
        family >= one,
        "…and it is still at least as wide as one person: {family}px against {one}px"
    );
}

#[test]
fn a_flag_is_one_drawing_rather_than_two_letters() {
    // A flag is a pair of regional indicators, ligated the same way. Split
    // between them, each draws the boxed letter that stands in for a country
    // nobody has a flag for — which is exactly the silent half-right output.
    let flag = drawn_width("🇧🇷");
    let one = drawn_width("🇧");
    assert!(
        flag < one * 2,
        "🇧🇷 must be one glyph rather than two letters: {flag}px against a pair \
         of {one}px letters"
    );
}

#[test]
fn a_keycap_takes_the_face_that_has_the_enclosing_mark() {
    // `1` is a character every text face has and `U+20E3` is one almost none
    // does. The cluster goes to the face that can set all of it, so a keycap
    // comes out as a keycap rather than as a bare digit.
    assert_ne!(
        drawn_width("1"),
        drawn_width("1\u{fe0f}\u{20e3}"),
        "the keycap is a different, wider drawing than the digit alone"
    );
    assert!(Font::sans().uncovered("1\u{fe0f}\u{20e3}").is_empty());
}
