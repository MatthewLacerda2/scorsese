//! Putting glyphs on the raster.
//!
//! Outlines in, filled pixels out — through **tiny-skia**, the same rasteriser
//! every other layer in this crate goes through. A glyph is a path like any
//! other path, so there is one anti-aliaser in the compositor rather than two
//! that could disagree, and text inherits whatever platform behaviour the
//! existing golden references already pin.
//!
//! Being the crate's one way onto the raster is also why [`fill_rects`] is
//! here rather than beside the module that wants it: a rectangle is a path
//! like a glyph is, and drawing one anywhere else would be a second
//! anti-aliaser and a second blend to keep in step with this one.

use skrifa::outline::OutlinePen;
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Transform};

use scorsese_core::Rgba;

use crate::frame::{BYTES_PER_PIXEL, Frame};

use super::font::{Face, Font};
use super::shape::Shaped;

/// Draws one line of `text` into `frame`, in `color`, at `size` pixels per em.
///
/// **The primitive everything else here is built from.** `origin` is where the
/// pen starts: `x` is the left edge of the first glyph's advance and `y` is the
/// **baseline** — the line the letters sit on, not the top of them. Baselines
/// rather than bounding boxes because that is the one horizontal that stays put
/// as the characters change: two lines share a baseline whether or not either
/// happens to contain a `g`.
///
/// Nothing is wrapped and nothing is clipped to a box; a glyph falling off the
/// edge of the frame is simply not drawn. Deciding what fits is the wrapping
/// layer's job, one level up.
pub fn draw_line(
    frame: &mut Frame,
    text: &str,
    font: &Font,
    size: f32,
    color: Rgba,
    origin: (f32, f32),
) {
    let face = font.at(size.max(1.0));
    let mut path = Outlines::default();
    line_into(&mut path, &face, &face.shape(text), origin);
    stamp(frame, path, color);
}

/// Draws several unrelated lines in one pass of the rasteriser.
///
/// [`draw_line`] for each of them would be correct and slow: every call fills
/// its own full-frame pixmap, so a ruler's two dozen labels would allocate and
/// walk the raster two dozen times. They share a face, a size and a colour —
/// which is exactly when the glyphs can go into one path — so the cost is one
/// pass however many runs there are.
pub(crate) fn draw_runs<S: AsRef<str>>(
    frame: &mut Frame,
    font: &Font,
    size: f32,
    color: Rgba,
    runs: &[(S, (f32, f32))],
) {
    let face = font.at(size.max(1.0));
    let mut path = Outlines::default();
    for (text, origin) in runs {
        line_into(&mut path, &face, &face.shape(text.as_ref()), *origin);
    }
    stamp(frame, path, color);
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

/// Traces an already-shaped run into `path`, with the run starting at
/// `origin`.
///
/// Shaped rather than plain text because where a glyph goes was decided
/// upstream — the shaper applied the face's kerning, and a run that was
/// measured to fit a line has to be drawn as the same run it was measured as.
pub(super) fn line_into(path: &mut Outlines, face: &Face<'_>, shaped: &Shaped, origin: (f32, f32)) {
    let (left, baseline) = origin;
    for glyph in &shaped.glyphs {
        // The run's offsets are font-space — y upwards — and a baseline is a
        // row of the raster, so a glyph lifted off the baseline moves up the
        // frame, which is towards zero.
        path.place(left + glyph.at.0, baseline - glyph.at.1);
        face.outline(glyph.id, path);
    }
}

/// Collects glyph outlines into one tiny-skia path, flipping them onto the
/// raster as they arrive.
///
/// One path for a whole block rather than one per glyph: the rasteriser is then
/// entered once, and overlapping letters — an italic `f` reaching into the next
/// one — are filled as a single shape instead of being blended over each other
/// twice at the seam.
#[derive(Default)]
pub(super) struct Outlines {
    builder: PathBuilder,
    /// Where the current glyph's origin sits on the raster. Outlines arrive in
    /// font space with y upwards, so `y` is subtracted rather than added.
    at: (f32, f32),
}

impl Outlines {
    /// Moves the origin the next glyph's outline is relative to.
    pub(super) fn place(&mut self, x: f32, baseline: f32) {
        self.at = (x, baseline);
    }

    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (self.at.0 + x, self.at.1 - y)
    }
}

impl OutlinePen for Outlines {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (cx, cy) = self.point(cx, cy);
        let (x, y) = self.point(x, y);
        self.builder.quad_to(cx, cy, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (cx0, cy0) = self.point(cx0, cy0);
        let (cx1, cy1) = self.point(cx1, cy1);
        let (x, y) = self.point(x, y);
        self.builder.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

/// Fills the collected outlines onto the frame in one pass.
///
/// The fill happens in a scratch pixmap because tiny-skia blends in
/// **premultiplied** alpha and a [`Frame`] carries **straight** alpha. Over a
/// transparent start the two differ everywhere the glyph edge is soft, so the
/// pixels are converted on the way back rather than written through and left
/// subtly dark at every antialiased edge.
pub(super) fn stamp(frame: &mut Frame, outlines: Outlines, color: Rgba) {
    let Some(path) = outlines.builder.finish() else {
        return;
    };
    fill(frame, &path, color);
}

/// Fills one finished path onto the frame — the step [`stamp`] and
/// [`fill_rects`] share, so glyphs and rectangles cannot come out blended
/// differently.
fn fill(frame: &mut Frame, path: &Path, color: Rgba) {
    let resolution = frame.resolution();
    let Some(mut pixmap) = Pixmap::new(resolution.width(), resolution.height()) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    // Anti-aliased, because a title with stepped edges is the one thing that
    // makes a render look amateur at any resolution.
    paint.anti_alias = true;
    pixmap.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);

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
/// Over the transparent frame a text layer starts as, this is a copy. It is
/// written as the general case anyway because the frame is not always empty —
/// a slug card draws its prompt over a background it has already filled — and
/// a copy there would punch a hole in what is underneath at every soft edge.
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
