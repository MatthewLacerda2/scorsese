//! Drawing text into a frame.
//!
//! tiny-skia fills paths and does no text at all, so this is a glyph pipeline
//! of its own: `font` reads a face and its outlines, `layout` decides which
//! characters go on which line, and `draw` fills the result into a [`Frame`]
//! — through tiny-skia, so the compositor keeps one rasteriser rather than
//! two that could disagree. The crate that reads the faces is **skrifa**,
//! sized to the job; the reasoning is in `crates/compositor/Cargo.toml`.
//!
//! **Text is a layer like any other.** What this module produces is pixels on a
//! frame; that frame then goes through the same [`crate::compose`] path as a
//! decoded video frame, with the same `opacity`, `transform.position.*` and
//! `transform.scale.*` resolved from the same keyframe tracks. A title that
//! fades and slides costs nothing here — it is the existing properties acting
//! on a layer that happens to have letters in it, which is why text needs no
//! animatable properties of its own.
//!
//! **Everything is in pixels of the raster.** A [`Style`] here says 48 pixels,
//! not "a tenth of the frame": a raster is the only place a pixel means
//! anything, and the fraction a project actually stores
//! ([`scorsese_core::TextStyle`]) is turned into one of these where the render
//! resolution is known.

mod draw;
mod font;
mod layout;

pub use draw::draw_line;
pub use font::{Font, FontError};

use scorsese_core::{Rgba, TextAlign};

use crate::frame::Frame;

/// How a block of text is set, in pixels of the frame it goes on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Em size in pixels.
    pub size: f32,
    /// What colour the glyphs are drawn, alpha included.
    pub color: Rgba,
    /// Which edge of the wrapped block the lines line up against.
    pub align: TextAlign,
    /// Baseline to baseline, in pixels.
    pub line_height: f32,
    /// How wide the block may run before a line wraps, in pixels.
    pub max_width: f32,
}

/// Draws `text` into `frame`, wrapped to the style's width and **centred on
/// the frame** both ways.
///
/// Centred rather than placed, because placement already exists: a title is
/// moved with `transform.position.x` and `transform.position.y`, the same two
/// properties that move a video clip, keyframes and all. Giving text its own
/// notion of where it sits would be a second way to say the same thing, and the
/// two would disagree the first time someone animated one of them.
///
/// The frame is drawn onto as it arrives — cleared or not. A text layer starts
/// transparent so the tracks underneath show through everywhere the glyphs are
/// not.
pub fn draw(frame: &mut Frame, text: &str, font: &Font, style: &Style) {
    let resolution = frame.resolution();
    let (width, height) = (resolution.width() as f32, resolution.height() as f32);

    // Every guard below is a `max`, which is also how a NaN is caught: it
    // returns the other operand. A style that says nothing sane still draws
    // something rather than looping forever or vanishing.
    let size = style.size.max(1.0);
    let line_height = style.line_height.max(size);
    let max_width = if style.max_width.is_finite() && style.max_width > 0.0 {
        style.max_width
    } else {
        width
    };
    let rows = (height / line_height).floor().max(1.0) as usize;

    let face = font.at(size);
    let lines = layout::wrap(text, &face, max_width, rows);
    let (ascent, descent) = face.extents();
    let top = (height - line_height * lines.len() as f32) / 2.0;
    let box_left = (width - max_width) / 2.0;

    // One path for the whole block, filled once: the rasteriser is entered a
    // single time however many lines there are, and letters that overlap are
    // one shape rather than two blended over each other at the seam.
    let mut outlines = draw::Outlines::default();
    for (row, line) in lines.iter().enumerate() {
        let left = match style.align {
            TextAlign::Left => box_left,
            TextAlign::Center => (width - line.width) / 2.0,
            TextAlign::Right => box_left + max_width - line.width,
        };
        // The visual middle of a line of text sits `(ascent + descent) / 2`
        // above its baseline — descent being negative — so centring the block
        // means putting that midpoint, not the baseline, on the row's centre.
        // Baselines alone would hang every block a little low.
        let baseline = top + line_height * (row as f32 + 0.5) + (ascent + descent) / 2.0;
        draw::line_into(&mut outlines, &face, &line.text, (left, baseline));
    }
    draw::stamp(frame, outlines, style.color);
}
