//! Spacing an author wrote down, and whether it reaches the raster.
//!
//! Ordinary whitespace collapses, which is right for wrapped prose. `U+00A0`
//! is the character that opts out of exactly that, and it carries the Unicode
//! `White_Space` property — so every whitespace-shaped helper in the standard
//! library eats it by default. These assert it survives both halves of the
//! journey: the line breaker must not break at one, and the shaper must not
//! drop one for want of a glyph.

use scorsese_compositor::text::{self, Font, Slant, Style};
use scorsese_core::Rgba;

use crate::ink::{self, canvas, style};

/// Left-aligned and wide, so indentation shows up as ink starting further in
/// rather than as a centred block moving half as far.
fn flush_left() -> Style {
    Style {
        align: scorsese_core::TextAlign::Left,
        ..style(20.0, Rgba::WHITE)
    }
}

fn left_edge(text: &str) -> u32 {
    let mut frame = canvas();
    text::draw(&mut frame, text, Font::sans(), &flush_left());
    ink::bounds(&frame).expect("something was drawn").0
}

fn width_of(text: &str) -> u32 {
    let mut frame = canvas();
    text::draw(&mut frame, text, Font::sans(), &flush_left());
    let (left, _, right, _) = ink::bounds(&frame).expect("something was drawn");
    right - left
}

#[test]
fn a_leading_non_breaking_space_indents_the_line() {
    // The failure this replaces: `split_whitespace` produced no word for the
    // run of NBSP, so the indent vanished and the line sat flush left.
    let flush = left_edge("tick()");
    let indented = left_edge("\u{a0}\u{a0}\u{a0}\u{a0}tick()");
    assert!(
        indented > flush,
        "four non-breaking spaces must push the line right; flush {flush}, indented {indented}"
    );
}

#[test]
fn a_run_of_non_breaking_spaces_keeps_every_one_of_them() {
    // A column is only a column if the run does not collapse to one space.
    let one = width_of("a\u{a0}b");
    let many = width_of("a\u{a0}\u{a0}\u{a0}\u{a0}\u{a0}\u{a0}b");
    assert!(
        many > one,
        "six non-breaking spaces must set wider than one; one {one}, many {many}"
    );
}

#[test]
fn ordinary_spaces_still_collapse() {
    // Deliberately unchanged. Collapsing is right for wrapped prose, and this
    // is the assertion that says so out loud rather than leaving it to be
    // rediscovered as a regression.
    assert_eq!(width_of("a b"), width_of("a      b"));
}

/// The width of the whole set block, in a box too narrow for both words.
fn packed_width(text: &str) -> u32 {
    let narrow = Style {
        max_width: 60.0,
        ..flush_left()
    };
    let mut frame = canvas();
    text::draw(&mut frame, text, Font::sans(), &narrow);
    let (left, _, right, _) = ink::bounds(&frame).expect("something was drawn");
    right - left
}

#[test]
fn a_non_breaking_space_is_not_a_place_to_break() {
    // The other half of the character's meaning, read off the shape of the
    // block rather than off a line count. With an ordinary space the break
    // lands AT the space, so each line is one short word wide. With a
    // non-breaking space there is no break opportunity there at all, so the
    // pair is one word: it is split between characters instead, and the line
    // packs out to the box. A wider block is that difference.
    let broken = packed_width("aaa bbb");
    let joined = packed_width("aaa\u{a0}bbb");
    assert!(
        joined > broken,
        "a non-breaking space is not a break opportunity, so the line packs \
         to the box; broke to {broken}, packed to {joined}"
    );
}

/// The trap that makes the fix above look like it did not work.
///
/// A face with no `U+00A0` of its own shapes it to `.notdef`, which the shaper
/// drops **with its advance** — so the run is preserved through layout and
/// then set at zero width, and the symptom is identical to the bug. Asserted
/// per shipped family because coverage varies between them.
#[test]
fn every_shipped_face_sets_a_non_breaking_space_with_width() {
    for name in text::names() {
        let font = Font::shipped(name, None, Slant::Upright)
            .unwrap_or_else(|error| panic!("`{name}` does not resolve: {error}"));
        let mut tight = canvas();
        text::draw(&mut tight, "a\u{a0}b", &font, &flush_left());
        let mut loose = canvas();
        text::draw(
            &mut loose,
            "a\u{a0}\u{a0}\u{a0}\u{a0}b",
            &font,
            &flush_left(),
        );

        let (tl, _, tr, _) = ink::bounds(&tight).expect("something was drawn");
        let (ll, _, lr, _) = ink::bounds(&loose).expect("something was drawn");
        assert!(
            lr - ll > tr - tl,
            "`{name}` sets a non-breaking space at zero width — the glyph was \
             dropped with its advance"
        );
    }
}
