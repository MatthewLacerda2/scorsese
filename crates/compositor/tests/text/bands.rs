//! The band a block of text is set in.
//!
//! `draw_in` is the generalisation `draw` became: a block is centred in a
//! strip of the frame and truncated to it. The whole frame is one such strip,
//! which is why a title is unchanged by any of this, and the foot of the frame
//! is another, which is where a narration card's words go.

use scorsese_compositor::Resolution;
use scorsese_compositor::text::{self, Band, Font};
use scorsese_core::Rgba;

use crate::cards::{PANEL, card_of, foot};
use crate::ink::{self, HEIGHT, WIDTH, canvas, style};

/// The text belongs to the band, not to the frame — otherwise a narration card
/// would print its words across the middle of the picture and the panel would
/// be an empty stripe underneath them.
#[test]
fn the_text_is_set_inside_the_band() {
    let frame = card_of(foot(), PANEL, "narration");
    // Only the glyphs are white; the panel is not.
    let white = frame
        .bytes()
        .chunks_exact(4)
        .enumerate()
        .filter(|(_, pixel)| pixel[0] > 200)
        .map(|(index, _)| index as u32 / WIDTH);
    let (first, last) = white.fold((HEIGHT, 0), |(lo, hi), row| (lo.min(row), hi.max(row)));

    assert!(
        first >= (HEIGHT as f32 * 0.7) as u32,
        "no ink above the band"
    );
    assert!(last < HEIGHT, "and none past the bottom of it");
}

/// White on gray stays white: the glyphs blend over the panel that is already
/// in the buffer rather than punching a hole in it.
#[test]
fn ink_over_a_panel_is_the_colour_it_asked_for() {
    let frame = card_of(foot(), PANEL, "narration");
    let brightest = frame
        .bytes()
        .chunks_exact(4)
        .map(|pixel| (pixel[0], pixel[1], pixel[2], pixel[3]))
        .max_by_key(|pixel| pixel.0)
        .expect("a frame has pixels");

    assert_eq!(brightest, (255, 255, 255, 255));
}

/// The whole frame is a band like any other, and text set in it lands exactly
/// where plain `draw` puts it — which is what makes this a generalisation of
/// the existing behaviour rather than a second way to draw.
#[test]
fn the_whole_frame_is_a_band_that_changes_nothing() {
    let content = "a sentence long enough to wrap at least once";
    let mut plain = canvas();
    text::draw(&mut plain, content, Font::sans(), &style(20.0, Rgba::WHITE));

    let mut banded = canvas();
    text::draw_in(
        &mut banded,
        content,
        Font::sans(),
        &style(20.0, Rgba::WHITE),
        Band::whole(Resolution::new(WIDTH, HEIGHT).expect("a legal raster")),
    );

    assert_eq!(plain.bytes(), banded.bytes());
}

/// A band truncates to itself. A prompt too long for the strip it is given
/// ends in an ellipsis, exactly as one too long for a frame does.
#[test]
fn a_band_truncates_to_its_own_height() {
    let long = "one two three four five six seven eight nine ten ".repeat(6);
    let long = long.as_str();
    let mut whole = canvas();
    text::draw(&mut whole, long, Font::sans(), &style(14.0, Rgba::WHITE));
    let mut banded = canvas();
    text::draw_in(
        &mut banded,
        long,
        Font::sans(),
        &style(14.0, Rgba::WHITE),
        foot(),
    );

    assert!(
        ink::lines(&banded) < ink::lines(&whole),
        "the band holds fewer lines than the frame does: {} vs {}",
        ink::lines(&banded),
        ink::lines(&whole)
    );
}
