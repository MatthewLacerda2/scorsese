//! Cards, and the band a block of text is set in.
//!
//! A card is a panel of colour with text on it, so what is asserted here is
//! that the panel goes exactly where it was asked to go, the text goes with
//! it, and neither touches the rest of the frame.

use scorsese_compositor::card::{self, Card};
use scorsese_compositor::text::{Band, Font};
use scorsese_compositor::Frame;
use scorsese_core::Rgba;

use crate::ink::{HEIGHT, WIDTH, canvas, pixel, style};

pub(crate) const PANEL: Rgba = Rgba::opaque(0x3c, 0x3c, 0x3c);

/// The bottom third, which is where a narration card sits.
pub(crate) fn foot() -> Band {
    Band {
        top: HEIGHT as f32 * 0.7,
        height: HEIGHT as f32 * 0.3,
    }
}

pub(crate) fn card_of(band: Band, background: Rgba, text: &str) -> Frame {
    let mut frame = canvas();
    card::draw(
        &mut frame,
        &Card {
            band,
            background,
            text,
            style: style(14.0, Rgba::WHITE),
        },
        Font::sans(),
    );
    frame
}

#[test]
fn a_card_paints_its_band_and_nothing_above_it() {
    let frame = card_of(foot(), PANEL, "");

    assert_eq!(
        pixel(&frame, WIDTH / 2, 10),
        (0, 0, 0, 0),
        "above the band the layer is still transparent, so the picture shows"
    );
    assert_eq!(
        pixel(&frame, WIDTH / 2, HEIGHT - 2),
        (PANEL.r, PANEL.g, PANEL.b, PANEL.a),
        "inside it, the panel"
    );
}

#[test]
fn the_band_reaches_the_edge_it_was_asked_for() {
    let frame = card_of(foot(), PANEL, "");
    let top_of_panel = (HEIGHT as f32 * 0.7) as u32;

    assert_eq!(pixel(&frame, 0, top_of_panel - 1).3, 0, "one row short");
    assert_eq!(pixel(&frame, 0, top_of_panel).3, PANEL.a, "the first row");
    assert_eq!(pixel(&frame, 0, HEIGHT - 1).3, PANEL.a, "and the last");
}

/// A translucent panel is a translucent *layer*: the alpha it was given is the
/// alpha it keeps, which is what lets a narration card be read over a shot.
#[test]
fn a_translucent_panel_keeps_its_alpha() {
    let veil = Rgba::new(0x14, 0x14, 0x14, 0xd8);
    let frame = card_of(foot(), veil, "");

    assert_eq!(
        pixel(&frame, WIDTH / 2, HEIGHT - 2),
        (veil.r, veil.g, veil.b, veil.a)
    );
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
