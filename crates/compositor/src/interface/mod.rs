//! The few marks a feed's interface is drawn from, for the preview that lays
//! one over a finished frame (#1010): filled and outlined rounded boxes, dots
//! and rings, a symbol placed by its corner, a fade, and one picture laid over
//! another.
//!
//! Pixels throughout, as [`crate::shape`]'s figures are: which controls a
//! platform shows, and where, is `scorsese-render`'s, which knows the
//! placement and the raster. Every mark goes through `paint`, so a
//! soft edge here is the soft edge everything else draws.

#[cfg(test)]
mod tests;

use scorsese_core::Rgba;
use tiny_skia::{PathBuilder, Rect};

use crate::area::Area;
use crate::frame::{BYTES_PER_PIXEL, Frame};
use crate::icon::Icon;
use crate::paint;
use crate::text::Font;

/// Fills `area` with its corners rounded to `radius`. An area with no inside
/// draws nothing.
pub fn rounded(frame: &mut Frame, area: Area, radius: f32, color: Rgba) {
    if let Some(path) = rounded_path(area, radius) {
        paint::fill(frame, &path, color);
    }
}

/// Strokes the outline of `area`, its corners rounded to `radius`, in a line
/// `width` pixels thick centred on the edge.
pub fn outlined(frame: &mut Frame, area: Area, radius: f32, (color, width): (Rgba, f32)) {
    if let Some(path) = rounded_path(area, radius) {
        paint::stroke(frame, &path, color, width, None);
    }
}

/// Fills a circle of `radius` about `centre`.
pub fn disc(frame: &mut Frame, centre: (f32, f32), radius: f32, color: Rgba) {
    if let Some(path) = PathBuilder::from_circle(centre.0, centre.1, radius) {
        paint::fill(frame, &path, color);
    }
}

/// Strokes a circle of `radius` about `centre`, `width` pixels thick.
pub fn ring(frame: &mut Frame, centre: (f32, f32), radius: f32, (color, width): (Rgba, f32)) {
    if let Some(path) = PathBuilder::from_circle(centre.0, centre.1, radius) {
        paint::stroke(frame, &path, color, width, None);
    }
}

/// Draws `icon` in a square of `size` pixels whose top-left corner is `at`,
/// its line `width` pixels thick.
pub fn symbol(
    frame: &mut Frame,
    icon: &Icon,
    at: (f32, f32),
    size: f32,
    (color, width): (Rgba, f32),
) {
    crate::icon::draw::draw_at(frame, icon, at, size, (color, width));
}

/// How wide one line of `text` sets at `size`, so a mark can end at a known
/// edge or be centred.
pub fn line_width(font: &Font, text: &str, size: f32) -> f32 {
    crate::text::width(font, text, size)
}

/// Fills the rows from `top`, `height` pixels down, with a colour going from
/// `from` at the top to `to` at the bottom: the shade a feed lays under its
/// controls so they read over any picture. Written straight into the frame,
/// so it is drawn first, onto a transparent one.
pub fn fade(frame: &mut Frame, top: f32, height: f32, (from, to): (Rgba, Rgba)) {
    let resolution = frame.resolution();
    let (width, rows) = (resolution.width() as usize, resolution.height());
    if !height.is_finite() || height <= 0.0 {
        return;
    }
    let first = top.max(0.0).floor() as u32;
    let last = ((top + height).ceil().max(0.0) as u32).min(rows);
    let row_bytes = width * BYTES_PER_PIXEL;
    for row in first..last {
        let along = ((row as f32 + 0.5 - top) / height).clamp(0.0, 1.0);
        let mix =
            |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * along).round() as u8;
        let color = [
            mix(from.r, to.r),
            mix(from.g, to.g),
            mix(from.b, to.b),
            mix(from.a, to.a),
        ];
        let start = row as usize * row_bytes;
        for pixel in frame.bytes_mut()[start..start + row_bytes].chunks_exact_mut(BYTES_PER_PIXEL) {
            pixel.copy_from_slice(&color);
        }
    }
}

/// Lays `layer` over `frame`, source-over, wherever it is not transparent.
/// The two must be the same size; a layer of another size lays nothing.
pub fn over(frame: &mut Frame, layer: &Frame) {
    if frame.resolution() == layer.resolution() {
        paint::blend_frame(frame, layer, 1.0);
    }
}

/// A rounded rectangle's path, or `None` for one with no inside.
fn rounded_path(area: Area, radius: f32) -> Option<tiny_skia::Path> {
    let rect = Rect::from_xywh(area.left, area.top, area.width, area.height)?;
    let radius = radius.clamp(0.0, area.width.min(area.height) / 2.0);
    let mut builder = PathBuilder::new();
    crate::shape::closed::rounded(&mut builder, rect, radius);
    builder.finish()
}
