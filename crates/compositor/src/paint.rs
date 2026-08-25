//! The crate's one way onto a raster.
//!
//! Everything drawn here — glyphs, gridlines, the boxes and circles of a
//! diagram — is a tiny-skia path, filled or stroked by these two functions and
//! blended back onto the [`Frame`] by the one below them. That is the whole
//! reason the module exists: a second rasteriser, or a second blend, would be
//! two answers to *what does a soft edge look like* that could drift apart, and
//! the golden gate compares pixels.
//!
//! It lived under `text` while glyphs were the only thing being drawn, which is
//! also where its `fill_rects` sat despite belonging to a ruler. Shapes made
//! that home wrong rather than merely odd — a rectangle asset reaching into the
//! text module to be drawn describes nothing true about either.
//!
//! **The scratch pixmap is not an optimisation to remove.** tiny-skia blends in
//! premultiplied alpha and a [`Frame`] carries straight alpha; over the
//! transparent start a drawn layer has, the two differ everywhere an edge is
//! soft. Writing through would leave every anti-aliased boundary subtly dark.

use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

use scorsese_core::Rgba;

use crate::frame::{BYTES_PER_PIXEL, Frame};

/// Fills one finished path onto the frame.
pub(crate) fn fill(frame: &mut Frame, path: &Path, color: Rgba) {
    onto(frame, color, |pixmap, paint| {
        pixmap.fill_path(path, paint, FillRule::Winding, Transform::identity(), None);
    });
}

/// Strokes one finished path onto the frame, `width` pixels thick.
///
/// The line straddles the path — half either side — which is tiny-skia's
/// behaviour and every drawing program's, and is what keeps a shape's stated
/// size the size of the shape rather than of its ink.
///
/// Square ends and mitred corners, which is what a box, an ellipse and an
/// arrow are drawn with. [`stroke_round`] is the other one.
pub(crate) fn stroke(frame: &mut Frame, path: &Path, color: Rgba, width: f32) {
    lined(frame, path, color, width, LineCap::Butt, LineJoin::Miter);
}

/// The same, with the line's ends and corners rounded off.
///
/// An icon's and a caption's rim, and for the same reason in both: a mitred
/// corner spikes wherever the outline turns sharply. Lucide draws every one of
/// its symbols with `stroke-linecap: round` and `stroke-linejoin: round`, and
/// those are not a finish applied afterwards — they are part of the drawing; a
/// stroke that stopped square would give every icon in the set blunt ends and
/// spiked corners, which is a different set of icons. A rim round type meets
/// the apex of an `A` and the vertices of a `W`, and a mitred join there grows
/// horns off the letter instead of following it.
pub(crate) fn stroke_round(frame: &mut Frame, path: &Path, color: Rgba, width: f32) {
    lined(frame, path, color, width, LineCap::Round, LineJoin::Round);
}

/// Strokes with the ends and corners the caller asks for.
///
/// A width that is zero, negative or not a number draws nothing. There is no
/// sensible line to invent for any of them, and a default would be a border
/// nobody asked for on a shape that said it had none.
fn lined(frame: &mut Frame, path: &Path, color: Rgba, width: f32, cap: LineCap, join: LineJoin) {
    if !width.is_finite() || width <= 0.0 {
        return;
    }
    let stroke = Stroke {
        width,
        line_cap: cap,
        line_join: join,
        ..Stroke::default()
    };
    onto(frame, color, |pixmap, paint| {
        pixmap.stroke_path(path, paint, &stroke, Transform::identity(), None);
    });
}

/// Fills axis-aligned rectangles — `(x, y, width, height)` in pixels — onto
/// `frame` in `color`, all of them in one pass.
///
/// Anti-aliased and blended like everything else here, which is what a
/// gridline needs: a line at a tenth of a 1280-pixel raster falls at 128.0 for
/// one tenth and 384.0 for another, and one snapped to whole pixels would sit
/// at a slightly different fraction than the one it claims to mark.
pub(crate) fn fill_rects(frame: &mut Frame, rects: &[(f32, f32, f32, f32)], color: Rgba) {
    let mut builder = PathBuilder::new();
    for &(x, y, width, height) in rects {
        if let Some(rect) = Rect::from_xywh(x, y, width, height) {
            builder.push_rect(rect);
        }
    }
    if let Some(path) = builder.finish() {
        fill(frame, &path, color);
    }
}

/// Rasterises into a scratch pixmap and blends the result onto the frame.
///
/// `draw` is handed the pixmap and a paint already set to `color` and
/// anti-aliased — anti-aliased because a stepped edge is the one thing that
/// makes a render look amateur at any resolution.
fn onto(frame: &mut Frame, color: Rgba, draw: impl FnOnce(&mut Pixmap, &Paint)) {
    let resolution = frame.resolution();
    let Some(mut pixmap) = Pixmap::new(resolution.width(), resolution.height()) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;
    draw(&mut pixmap, &paint);

    for (pixel, drawn) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .zip(pixmap.pixels())
    {
        if drawn.alpha() == 0 {
            continue;
        }
        let straight = drawn.demultiply();
        blend(
            pixel,
            [
                straight.red(),
                straight.green(),
                straight.blue(),
                straight.alpha(),
            ],
        );
    }
}

/// Source-over of one straight-alpha pixel onto another.
///
/// Over the transparent frame a drawn layer starts as, this is a copy. It is
/// written as the general case anyway because the frame is not always empty —
/// a slug card draws its prompt over a background it has already filled, and a
/// shape draws its border over its own fill — and a copy there would punch a
/// hole in what is underneath at every soft edge.
fn blend(pixel: &mut [u8], source: [u8; 4]) {
    let source_alpha = f32::from(source[3]) / 255.0;
    let kept = f32::from(pixel[3]) / 255.0 * (1.0 - source_alpha);
    let out_alpha = source_alpha + kept;
    if out_alpha <= 0.0 {
        return;
    }
    for (channel, &value) in source[..3].iter().enumerate() {
        let mixed =
            (f32::from(value) * source_alpha + f32::from(pixel[channel]) * kept) / out_alpha;
        pixel[channel] = round(mixed);
    }
    pixel[3] = round(out_alpha * 255.0);
}

/// Rounds a channel back to a byte, clamped — floating point drift near the
/// ends must not wrap a solid pixel round to transparent.
fn round(value: f32) -> u8 {
    (value + 0.5).clamp(0.0, 255.0) as u8
}
