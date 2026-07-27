//! Cards: a panel of colour with a block of text set in it.
//!
//! This is what a slug card is made of, and it is deliberately almost nothing:
//! [`Frame::fill_rows`] and then [`text::draw_in`], into the same [`Band`], in
//! that order. There is no card *path* through the compositor — what comes out
//! is an ordinary layer, composited, transformed and faded by exactly the code
//! a decoded video frame goes through.
//!
//! What a card *says* is not decided here. This crate draws the words it is
//! given at the size it is given them; which words a sketch clip deserves, and
//! what fraction of the frame a narration band is, belong to `scorsese-render`,
//! where the assets table and the render's raster are both known.

use scorsese_core::Rgba;

use crate::frame::Frame;
use crate::text::{self, Band, Font, Style};

/// A panel of colour with text set in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Card<'a> {
    /// Which strip of the frame the panel covers, and the strip the text is
    /// centred in — one value, so the two cannot disagree.
    pub band: Band,
    /// What colour the panel is, alpha included. A translucent panel is a
    /// translucent *layer*: the picture below shows through it once the
    /// compositor puts the two together, which is what makes a narration card
    /// readable over the shot it narrates.
    pub background: Rgba,
    /// The words on the card. Newlines are honoured and everything else wraps.
    pub text: &'a str,
    /// How those words are set — size, colour, alignment, wrap column.
    pub style: Style,
}

/// Draws `card` into `frame`: the panel first, the text over it.
///
/// The rest of the frame is left exactly as it arrived, so a card drawn into a
/// transparent buffer contributes its band and nothing else.
pub fn draw(frame: &mut Frame, card: &Card<'_>, font: &Font) {
    frame.fill_rows(card.band.rows(frame.resolution()), card.background);
    text::draw_in(frame, card.text, font, &card.style, card.band);
}
