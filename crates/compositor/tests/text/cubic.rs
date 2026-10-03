//! A font drawn in cubic curves — the outlines of most `.otf` files people own.
//!
//! Every face shipped, and every other test font, is TrueType: quadratic
//! segments only. A CFF face is the one thing that reaches the pen's cubic
//! segment, so without this file a drawing that dropped every cubic would pass
//! the whole suite while a user's own `.otf` set letters with no curves in them
//! (#671). Why this face rather than another is in `tests/fonts/README.md`.

use scorsese_compositor::text::{self, Font};
use scorsese_core::Rgba;

use crate::ink::{self, canvas};

/// Source Serif 4 Regular as Adobe builds it: `CFF ` outlines, no `glyf`.
const CFF: &[u8] = include_bytes!("../fonts/SourceSerif4-Regular.otf");

/// The face's own x-height, in thousandths of the em (`OS/2.sxHeight`).
const X_HEIGHT: f32 = 0.475;

/// A round `o` is nothing but curves. Drawn whole, its ink is about an
/// x-height tall and its middle is the counter, empty; a drawing missing its
/// cubics has no ink at all, or a bowl with nothing round to hold the hole.
#[test]
fn a_cubic_outline_draws_a_round_letter_with_its_counter() {
    let font = Font::from_bytes(CFF, None).expect("a static CFF face parses");
    let size = 120.0;
    let mut frame = canvas();
    text::draw_line(&mut frame, "o", &font, size, Rgba::WHITE, (100.0, 140.0));

    let (left, top, right, bottom) = ink::bounds(&frame).expect("the curves drew ink");
    let height = (bottom - top + 1) as f32;
    let expected = X_HEIGHT * size;
    assert!(
        (expected * 0.95..=expected * 1.1).contains(&height),
        "an `o` stands about an x-height tall ({expected}px, with overshoot); found {height}"
    );
    assert!(
        right - left > 30,
        "the bowl is round, not a sliver; found {} wide",
        right - left
    );
    let centre = ink::pixel(&frame, (left + right) / 2, (top + bottom) / 2);
    assert_eq!(
        centre.3, 0,
        "the counter is empty: the bowl closes around a hole"
    );
    assert!(
        ink::count(&frame) > 600,
        "the bowl itself is solid ink, not a hairline outline"
    );
}
