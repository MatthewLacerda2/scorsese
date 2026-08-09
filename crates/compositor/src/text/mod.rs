//! Drawing text into a frame.
//!
//! tiny-skia fills paths and does no text at all, so this is a glyph pipeline
//! of its own: `font` reads a face, `layout` decides which characters go on
//! which line, `shape` turns each line into the glyphs that set it — kerning
//! and all — and `draw` fills those outlines into a [`Frame`] through
//! tiny-skia, so the compositor keeps one rasteriser rather than two that
//! could disagree. Two crates read the face: **skrifa** for outlines and
//! **HarfRust** for shaping; the reasoning for both is in
//! `crates/compositor/Cargo.toml`.
//!
//! **Text is a layer like any other.** What this module produces is pixels on a
//! frame; that frame then goes through the same [`Compositor`](crate::Compositor)
//! path as a decoded video frame, with the same `opacity`, `transform.position.*`
//! and `transform.scale.*` resolved from the same keyframe tracks. A title that
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
mod shape;

pub use draw::draw_line;
pub(crate) use draw::{draw_runs, fill_rects};
pub use font::{Font, FontError, SHIPPED_WEIGHT, Shipped};

use std::ops::Range;

use scorsese_core::{Anchor, AnchorX, AnchorY, Rgba, TextAlign};

use crate::frame::{Frame, Resolution};

/// The horizontal strip of a frame a block of text is set in.
///
/// A block is centred in its band, so the whole frame is one band — what a
/// title gets — and the foot of the frame is another, which is what a
/// narration card sits in. Nothing else about the drawing changes: the same
/// wrapping, the same truncation, the same horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    /// Pixels from the top of the frame down to the top of the strip.
    pub top: f32,
    /// How tall the strip is, in pixels. What is set in it is truncated to
    /// this rather than to the frame, so a band cannot spill out of itself.
    pub height: f32,
}

impl Band {
    /// The whole of a frame, which is what plain [`draw`] sets text in.
    pub fn whole(resolution: Resolution) -> Self {
        Self {
            top: 0.0,
            height: resolution.height() as f32,
        }
    }

    /// The rows of `resolution` this band covers — what fills the panel of a
    /// card behind the text. Rounded outwards and clamped, so a band never
    /// leaves an unpainted row between itself and the edge it reaches.
    pub(crate) fn rows(self, resolution: Resolution) -> Range<u32> {
        let height = f64::from(resolution.height());
        let first = f64::from(self.top).clamp(0.0, height) as u32;
        let last = (f64::from(self.top) + f64::from(self.height)).clamp(0.0, height);
        first..(last.ceil() as u32).max(first)
    }
}

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
    /// Which edges of the frame the block's own edges are measured from.
    ///
    /// A text layer is drawn into a frame the size of the whole raster, so
    /// unlike a `native` picture there is no smaller rectangle for the
    /// compositor to rest — the anchor has to reach the *layout*. What it
    /// anchors is the **wrapped block at `max_width`**, not the longest line:
    /// left-anchored text could plausibly mean either, and only the first keeps
    /// a column's left edge still when the wording changes.
    pub anchor: Anchor,
}

/// How wide one line of `text` sets at `size`, in pixels.
///
/// Shaped rather than counted, so the answer includes the face's kerning and
/// is the width the same line would actually be drawn at. What it is for is
/// placing a run against a right-hand edge — [`draw_line`] starts at a left
/// edge, and something that has to end at a known x has to know how long it
/// is first.
pub(crate) fn width(font: &Font, text: &str, size: f32) -> f32 {
    font.at(size.max(1.0)).shape(text).width
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
    draw_in(frame, text, font, style, Band::whole(frame.resolution()));
}

/// [`draw`], into one band of the frame rather than the whole of it.
///
/// The block is centred in the band both ways and truncated to it, so a card's
/// panel and the text on it are described by the same value and cannot drift
/// apart. `draw` is this with the band being the entire frame.
pub fn draw_in(frame: &mut Frame, text: &str, font: &Font, style: &Style, band: Band) {
    let resolution = frame.resolution();
    let width = resolution.width() as f32;
    let height = band.height.max(1.0);

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
    let block_height = line_height * lines.len() as f32;
    let top = match style.anchor.y {
        AnchorY::Top => band.top,
        AnchorY::Center => band.top + (height - block_height) / 2.0,
        AnchorY::Bottom => band.top + height - block_height,
    };
    let box_left = match style.anchor.x {
        AnchorX::Left => 0.0,
        AnchorX::Center => (width - max_width) / 2.0,
        AnchorX::Right => width - max_width,
    };

    // One path for the whole block, filled once: the rasteriser is entered a
    // single time however many lines there are, and letters that overlap are
    // one shape rather than two blended over each other at the seam.
    let mut outlines = draw::Outlines::default();
    for (row, line) in lines.iter().enumerate() {
        // `align` places a line inside the block; the anchor placed the block
        // inside the frame. Centring a line has to be centring it in the
        // *block*, not in the frame, or an anchored column's lines would drift
        // back to the middle of the picture one at a time.
        let left = match style.align {
            TextAlign::Left => box_left,
            TextAlign::Center => box_left + (max_width - line.shaped.width) / 2.0,
            TextAlign::Right => box_left + max_width - line.shaped.width,
        };
        // The visual middle of a line of text sits `(ascent + descent) / 2`
        // above its baseline — descent being negative — so centring the block
        // means putting that midpoint, not the baseline, on the row's centre.
        // Baselines alone would hang every block a little low.
        let baseline = top + line_height * (row as f32 + 0.5) + (ascent + descent) / 2.0;
        draw::line_into(&mut outlines, &face, &line.shaped, (left, baseline));
    }
    draw::stamp(frame, outlines, style.color);
}
