//! `❤` and `❤️` are different pictures, and the second is the one a keyboard
//! sends.
//!
//! `U+FE0F` is the emoji presentation selector — *draw the one before me in
//! colour* — and it is what iOS and Android put after every emoji whose base
//! character predates emoji: `❤️ ☀️ ⚠️ ✔️ ▶️` and most of Miscellaneous
//! Symbols. Inter has a small black outline of each of those, so without the
//! selector being read the named face always wins and the author gets the
//! outline they did not choose. Nothing is dropped and nothing is reported,
//! which is why this is measured rather than looked at.
//!
//! **The measurement is chromatic pixels**: a pixel whose channels differ from
//! each other can only have come from a colour font, because the letters are
//! filled in one flat colour. Inked pixels are counted beside them so that a
//! selector that drew *nothing* could not pass for one that drew in colour.

use scorsese_compositor::text::{self, Font};
use scorsese_core::Rgba;

use crate::ink::{self, canvas};

const SIZE: f32 = 60.0;
const ORIGIN: (f32, f32) = (20.0, 120.0);

/// `(inked, chromatic)` for one string set in `sans` on a transparent frame.
fn drawn(content: &str) -> (usize, usize) {
    let mut frame = canvas();
    text::draw_line(&mut frame, content, Font::sans(), SIZE, Rgba::WHITE, ORIGIN);
    let (mut inked, mut chromatic) = (0, 0);
    for y in 0..frame.resolution().height() {
        for x in 0..frame.resolution().width() {
            let (r, g, b, a) = ink::pixel(&frame, x, y);
            if a == 0 {
                continue;
            }
            inked += 1;
            if r != g || g != b {
                chromatic += 1;
            }
        }
    }
    (inked, chromatic)
}

#[test]
fn the_emoji_selector_reaches_the_colour_face_and_no_selector_does_not() {
    // One test rather than two, because either half alone passes vacuously:
    // a chain that sent everything to the emoji face would satisfy the first
    // assertion, and one that read nothing satisfies the second.
    let (bare_inked, bare_chromatic) = drawn("\u{2764}");
    let (emoji_inked, emoji_chromatic) = drawn("\u{2764}\u{fe0f}");
    assert!(bare_inked > 0 && emoji_inked > 0, "both drew a heart");
    assert_eq!(
        bare_chromatic, 0,
        "`❤` with no selector is Inter's outline, filled in the caption's own \
         colour: {bare_chromatic} of {bare_inked} pixels were chromatic"
    );
    assert!(
        emoji_chromatic * 4 > emoji_inked,
        "`❤️` asked for colour, so most of it should be: {emoji_chromatic} of \
         {emoji_inked} pixels were chromatic"
    );
}

#[test]
fn the_text_selector_asks_the_other_way_and_is_read_too() {
    // The mirror of it, and the reason it is here: a selector read in one
    // direction and ignored in the other is a half-rule that reads as a bug.
    // Inter has the outline, so `❤︎` gets Inter — which is also what the bare
    // character gets, so the assertion is that the two are the same picture
    // while `❤️` is a different one.
    assert_eq!(drawn("\u{2764}\u{fe0e}"), drawn("\u{2764}"));
    assert_ne!(drawn("\u{2764}\u{fe0e}"), drawn("\u{2764}\u{fe0f}"));
}

#[test]
fn what_a_phone_keyboard_sends_comes_out_in_colour() {
    // The block this is actually about: `❤️ ☀️ ⚠️ ▶️ ⬆️`. Every one of these
    // has an outline in Inter *and* a drawing in Noto, which is the only case
    // the selector decides — `✔️` and `⭐` are in Noto alone, so they were
    // already reaching colour before anything read a selector and prove
    // nothing here.
    for base in ['\u{2764}', '\u{2600}', '\u{26a0}', '\u{25b6}', '\u{2b06}'] {
        let bare = drawn(&base.to_string());
        let selected = drawn(&format!("{base}\u{fe0f}"));
        assert_eq!(
            bare.1, 0,
            "U+{:04X} bare is the named face's outline",
            base as u32
        );
        assert!(
            selected.1 > 0,
            "U+{:04X} with U+FE0F is the colour drawing: {} of {} pixels were \
             chromatic",
            base as u32,
            selected.1,
            selected.0
        );
    }
}

#[test]
fn a_selector_on_a_character_only_the_fallback_has_changes_nothing() {
    // `⭐` is in Noto and not in Inter, so there is one face on offer and the
    // selector has nothing to choose between. Both presentations draw it, and
    // a text presentation that refused rather than drew would be the silence
    // the whole chain exists to end.
    let colour = drawn("\u{2b50}");
    assert!(colour.1 > 0, "the star is a colour drawing");
    assert_eq!(drawn("\u{2b50}\u{fe0f}"), colour);
    assert_eq!(drawn("\u{2b50}\u{fe0e}"), colour);
}
