//! What happens to a character the face a document named cannot draw.
//!
//! The claim under test is narrow and load-bearing: the fallback adds the
//! missing glyph and changes **nothing else**. So every assertion here is a
//! measurement of the letters around it — where they start, where they end, how
//! tall the line is — against the same caption without the emoji in it.

use scorsese_compositor::text::{self, Font};
use scorsese_core::Rgba;

use crate::ink::{self, bounds, canvas, style};

const SIZE: f32 = 24.0;
const ORIGIN: (f32, f32) = (10.0, 60.0);

/// The box the **white** ink occupies — the letters, and never a colour glyph,
/// which is drawn in the font's own colours. `(left, top, right, bottom)`.
fn letters(frame: &scorsese_compositor::Frame) -> (u32, u32, u32, u32) {
    let resolution = frame.resolution();
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for y in 0..resolution.height() {
        for x in 0..resolution.width() {
            let (r, g, b, a) = ink::pixel(frame, x, y);
            // A letter is the style's colour at whatever coverage the edge has;
            // an emoji is never neutral grey at full strength.
            if a == 0 || r != g || g != b {
                continue;
            }
            found = Some(match found {
                None => (x, y, x, y),
                Some((l, t, rr, bb)) => (l.min(x), t.min(y), rr.max(x), bb.max(y)),
            });
        }
    }
    found.expect("the letters were drawn")
}

fn drawn(content: &str) -> scorsese_compositor::Frame {
    let mut frame = canvas();
    text::draw_line(&mut frame, content, Font::sans(), SIZE, Rgba::WHITE, ORIGIN);
    frame
}

#[test]
fn the_emoji_is_drawn_rather_than_dropped() {
    let plain = drawn("Ship it");
    let with_fire = drawn("Ship it 🔥");
    let (_, _, plain_right, _) = bounds(&plain).expect("`Ship it` drew something");
    let (_, _, fire_right, _) = bounds(&with_fire).expect("`Ship it 🔥` drew something");
    assert!(
        fire_right > plain_right + 10,
        "the fire should add most of an em to the line: `Ship it` ends at \
         {plain_right}, `Ship it 🔥` at {fire_right}"
    );
}

#[test]
fn the_letters_keep_the_width_and_the_place_they_had() {
    // The whole promise of a fallback: the caption is the caption it was, plus
    // a glyph. A run boundary that lost an advance, or a scale worked out from
    // the wrong face, would move these.
    assert_eq!(
        letters(&drawn("Ship it")),
        letters(&drawn("Ship it 🔥")),
        "adding an emoji must not move a single letter"
    );
}

#[test]
fn the_fire_is_drawn_in_its_own_colours() {
    let frame = drawn("Ship it 🔥");
    let coloured = (0..frame.resolution().height())
        .flat_map(|y| (0..frame.resolution().width()).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let (r, g, b, a) = ink::pixel(&frame, *x, *y);
            a > 200
                && (u32::from(r).abs_diff(u32::from(g)) > 40
                    || u32::from(g).abs_diff(u32::from(b)) > 40)
        })
        .count();
    assert!(
        coloured > 40,
        "the fire is orange and yellow, so a solidly non-grey area should have \
         been painted; found {coloured} such pixels"
    );
}

#[test]
fn a_caption_is_no_taller_for_having_an_emoji_in_it() {
    // Line height and the baseline come from the named face and from nothing
    // else, so this is the criterion stated as a caption: the words sit where
    // they sat. It holds *structurally* — a chain is built from the named font
    // and never from the content, so the extents of a caption with an emoji in
    // it and one without are the same values — and the unit test beside
    // `Faces::extents` is what holds that mechanism still. What this one adds
    // is the promise in the form somebody would check it in.
    let block = |content: &str| {
        let mut frame = canvas();
        text::draw(&mut frame, content, Font::sans(), &style(SIZE, Rgba::WHITE));
        let (_, top, _, bottom) = letters(&frame);
        (top, bottom)
    };
    assert_eq!(block("Ship it"), block("Ship it 🔥"));
}

#[test]
fn wrapping_breaks_a_caption_in_the_same_place() {
    // The emoji goes on the last line of both, so the words above it break
    // exactly as they did. A width measured against the named face alone would
    // have counted the emoji as nothing at all.
    const WORDS: &str = "one two three four five six seven eight";
    let wrapped = |content: &str| {
        let mut frame = canvas();
        text::draw(&mut frame, content, Font::sans(), &style(20.0, Rgba::WHITE));
        (ink::lines(&frame), letters(&frame).0)
    };
    let (rows, left) = wrapped(WORDS);
    assert!(
        rows > 1,
        "the sentence has to wrap for this to test anything"
    );
    assert_eq!(wrapped(&format!("{WORDS} 🔥")), (rows, left));
}

#[test]
fn a_character_no_face_covers_still_draws_nothing() {
    // Unchanged behaviour, and it has to stay unchanged: the chain ran out, so
    // the character is dropped with its advance and `check` is what says so.
    let mut frame = canvas();
    text::draw_line(
        &mut frame,
        "\u{10ffff}",
        Font::sans(),
        SIZE,
        Rgba::WHITE,
        ORIGIN,
    );
    assert!(
        ink::bounds(&frame).is_none(),
        "nothing in the chain can draw it"
    );
    assert_eq!(Font::sans().uncovered("abc \u{10ffff}"), vec!['\u{10ffff}']);
}

#[test]
fn what_the_chain_covers_is_no_longer_reported_as_missing() {
    // `check` reports what vanishes from the frame. An emoji does not vanish
    // any more, so reporting it would be an objection to something correct.
    assert!(Font::sans().uncovered("Ship it 🔥").is_empty());
    assert!(
        Font::serif()
            .uncovered("👍🏽 👨\u{200d}👩\u{200d}👧 🇧🇷")
            .is_empty()
    );
}

#[test]
fn everything_noto_asks_for_can_actually_be_painted() {
    // The reporting path exists because COLRv1 can describe things tiny-skia
    // might not have. This says the face scorsese actually ships never needs it
    // — so a report with anything in it means a real gap rather than noise.
    let mut frame = canvas();
    let said = text::draw_line(
        &mut frame,
        "🔥 👍🏽 👨\u{200d}👩\u{200d}👧 🇧🇷 🎉 ❤️ 🌍 ⭐ 🚀 😀",
        Font::sans(),
        16.0,
        Rgba::WHITE,
        (4.0, 100.0),
    );
    assert_eq!(
        said,
        Vec::new(),
        "nothing in Noto Color Emoji is unpaintable"
    );
}
