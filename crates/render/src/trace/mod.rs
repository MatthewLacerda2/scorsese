//! Tracing a picture into an SVG a page draws on, stroke by stroke (#999).
//!
//! A generated illustration arrives as pixels, and nothing draws pixels on.
//! This turns a flat picture — a character, a logo, a scanned drawing — into
//! paths, in an order and a shape that the kit's `kit.draw` draws as a hand
//! would. Free, local and deterministic: the same picture and the same choices
//! answer the same file.
//!
//! The steps, in order:
//!
//! 1. `palette` reduces the picture to at most the colours asked for.
//! 2. `regions` cuts it into regions of one colour each, absorbs the specks
//!    smaller than the detail asked for, and drops the background unless it is
//!    kept. No two regions overlap.
//! 3. A dark region is **ink**, a pen line, and `ink` cuts it into strands,
//!    one per edge, each drawn as a single pass along that edge. Every other
//!    region is a **colour**: its outline drawn in its own colour, then filled.
//! 4. `outline` traces each one with visioncortex — the engine of vtracer,
//!    MIT or Apache-2.0 — and `svg` writes the document in drawing order.
//!
//! **The drawing order is the document order**, because that is the order
//! `kit.draw` draws in. The lines first, top to bottom, each region's outside
//! edge before its holes; then the colours, largest first. Since nothing
//! overlaps, the order is free to be the one a hand would choose rather than
//! the back-to-front one a stack of shapes would force.
//!
//! **A photograph traces into a mosaic of blobs**: tracing finds edges between
//! flat colours, and a photograph has none. Nothing refuses one; the tool that
//! calls this says so instead.

mod ink;
mod outline;
mod palette;
mod picture;
mod regions;
mod svg;
#[cfg(test)]
mod tests;

pub use picture::{TraceError, Vectorized, read, vectorize};

use outline::Fit;
use regions::{Indexed, NONE, Region};

/// A colour this dark or darker is a pen line, not a fill.
const INK: f64 = 0.18;

/// What a person chooses about a tracing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tracing {
    /// At most this many colours, background included. Flat art needs as many
    /// as it has; more only traces shading.
    pub colours: usize,
    /// How much small detail survives.
    pub detail: Detail,
    /// Keep the background colour as a shape of its own, rather than leaving
    /// the board to show around the drawing.
    pub keep_background: bool,
}

impl Default for Tracing {
    fn default() -> Self {
        Self {
            colours: 8,
            detail: Detail::Medium,
            keep_background: false,
        }
    }
}

/// How much small detail survives a tracing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Detail {
    /// Bold shapes: small marks are dropped and curves smoothed.
    Low,
    /// What a flat illustration needs.
    #[default]
    Medium,
    /// Small marks kept and curves followed closely, at the cost of more paths.
    High,
}

impl Detail {
    /// `low`, `medium` or `high`.
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    /// A mark smaller than the picture's area divided by this is a speck.
    fn speck_divisor(self) -> usize {
        match self {
            Self::Low => 4_000,
            Self::Medium => 16_000,
            Self::High => 60_000,
        }
    }

    /// How closely an edge is followed.
    fn fit(self) -> Fit {
        match self {
            Self::Low => Fit {
                corner: 80.0,
                segment: 8.0,
                splice: 60.0,
            },
            Self::Medium => Fit {
                corner: 60.0,
                segment: 5.0,
                splice: 45.0,
            },
            Self::High => Fit {
                corner: 45.0,
                segment: 3.5,
                splice: 45.0,
            },
        }
    }
}

/// A traced picture: the SVG, and what is in it.
#[derive(Debug, Clone, PartialEq)]
pub struct Traced {
    /// The SVG document, its `viewBox` the picture's own pixels.
    pub svg: String,
    /// The picture's width, and the drawing's.
    pub width: usize,
    /// Its height.
    pub height: usize,
    /// The colours it is drawn in, as `#rrggbb`, most used first.
    pub colours: Vec<String>,
    /// Pen strokes: one per edge of a dark line.
    pub strokes: usize,
    /// Filled shapes of colour.
    pub shapes: usize,
}

/// Traces `rgba` — four bytes a pixel, `width` by `height` — into an SVG whose
/// ids all begin with `name`, so two drawings can share a page.
pub fn trace(rgba: &[u8], width: usize, height: usize, tracing: Tracing, name: &str) -> Traced {
    let palette = palette::choose(rgba, tracing.colours.clamp(2, 32));
    let mut picture = Indexed {
        width,
        height,
        cells: rgba
            .chunks_exact(4)
            .map(|pixel| match pixel[3] {
                0..128 => NONE,
                _ => {
                    let index = palette::nearest(&palette, [pixel[0], pixel[1], pixel[2]]);
                    u8::try_from(index).unwrap_or(NONE)
                }
            })
            .collect(),
    };
    picture.absorb_specks((width * height / tracing.detail.speck_divisor()).max(4));
    if !tracing.keep_background {
        picture.drop_background();
    }
    let (mut inks, mut fills): (Vec<Region>, Vec<Region>) = picture
        .regions()
        .into_iter()
        .partition(|region| palette::lightness(palette[usize::from(region.colour)]) <= INK);
    inks.sort_by_key(|region| (region.bounds[1], std::cmp::Reverse(region.pixels.len())));
    fills.sort_by_key(|region| (std::cmp::Reverse(region.pixels.len()), region.bounds[1]));
    let mut drawing = svg::Drawing::new(width, height, name);
    let fit = tracing.detail.fit();
    let mut strokes = 0;
    for region in &inks {
        for strand in ink::strands(region, width) {
            drawing.stroke(&strand, palette[usize::from(region.colour)], fit);
            strokes += 1;
        }
    }
    for region in &fills {
        drawing.shape(region, width, palette[usize::from(region.colour)], fit);
    }
    let mut used: Vec<u8> = inks
        .iter()
        .chain(&fills)
        .map(|region| region.colour)
        .collect();
    used.sort_unstable();
    used.dedup();
    Traced {
        svg: drawing.finish(),
        width,
        height,
        colours: used
            .into_iter()
            .map(|index| svg::hex(palette[usize::from(index)]))
            .collect(),
        strokes,
        shapes: fills.len(),
    }
}
