//! Putting glyphs on the raster.
//!
//! Outlines in, filled pixels out. A glyph is a path like any other path, so
//! it goes onto the raster through [`crate::paint`] — the crate's one
//! rasteriser and one blend, shared with the shapes and the ruler, so there is
//! never a second answer to what a soft edge looks like.
//!
//! What is left here is the part that is actually about type: turning skrifa's
//! outlines, which arrive in font space with y upwards, into one tiny-skia path
//! sitting the right way up on the raster.

use skrifa::outline::OutlinePen;
use tiny_skia::PathBuilder;

use scorsese_core::Rgba;

use crate::frame::Frame;
use crate::paint;

use super::Edge;
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
    stamp(frame, path, color, None);
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
    stamp(frame, path, color, None);
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

/// Fills the collected outlines onto the frame in one pass — the whole block
/// as a single shape, so letters that overlap are blended once rather than
/// twice at the seam.
///
/// **An `edge` goes down first, and the fill covers half of it.** tiny-skia
/// strokes centred on a path, so a rim that shows `width` pixels outside the
/// letterform is a stroke of twice that with the glyph filled over the inner
/// half — which is what makes this a rim *added* to the letter rather than a
/// line eating into it. Two passes rather than one because they are two
/// colours; the whole block goes down in each, so an overlapping pair of
/// letters is still one shape and the seam between them is not drawn twice.
///
/// Round joins, and that is not a finish. A mitred one spikes wherever the
/// outline turns a sharp corner — the apex of an `A`, the vertices of a `W`,
/// every serif — so a rim meant to follow the letter would instead grow horns
/// off it.
pub(super) fn stamp(frame: &mut Frame, outlines: Outlines, color: Rgba, edge: Option<Edge>) {
    let Some(path) = outlines.builder.finish() else {
        return;
    };
    if let Some(edge) = edge {
        paint::stroke_round(frame, &path, edge.color, edge.width * 2.0);
    }
    paint::fill(frame, &path, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::frame::{BYTES_PER_PIXEL, Resolution};

    /// Room for two short runs well clear of each other and of every edge.
    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 120;

    /// A transparent frame, which is what a text layer starts as.
    fn canvas() -> Frame {
        let mut frame = Frame::black(Resolution::new(WIDTH, HEIGHT).expect("a legal raster"));
        frame.fill_transparent();
        frame
    }

    /// How many pixels carry any ink at all.
    fn inked(frame: &Frame) -> usize {
        frame
            .bytes()
            .chunks_exact(BYTES_PER_PIXEL)
            .filter(|pixel| pixel[3] > 0)
            .count()
    }

    /// The one claim [`draw_runs`] makes: it is [`draw_line`] for each run, done
    /// in a single pass of the rasteriser rather than one pass per run.
    ///
    /// So the assertion is the whole raster, byte for byte, against the runs
    /// drawn one at a time — not "some ink arrived". Two runs that do not
    /// overlap are the case where the two paths must agree exactly: every
    /// inked pixel is touched once either way, so a difference anywhere is a
    /// glyph the batch put somewhere else, drew at another size or colour, or
    /// did not draw at all.
    #[test]
    fn runs_drawn_in_one_pass_land_where_the_lines_do() {
        const SIZE: f32 = 24.0;
        let runs = [("Left", (12.0, 40.0)), ("Right", (160.0, 96.0))];
        let font = Font::sans();

        let mut batched = canvas();
        draw_runs(&mut batched, font, SIZE, Rgba::WHITE, &runs);

        let mut one_at_a_time = canvas();
        for (text, origin) in runs {
            draw_line(&mut one_at_a_time, text, font, SIZE, Rgba::WHITE, origin);
        }

        assert!(
            inked(&one_at_a_time) > 100,
            "the primitive itself drew the two runs"
        );
        assert_eq!(
            inked(&batched),
            inked(&one_at_a_time),
            "one pass over two runs inks as many pixels as two passes do"
        );
        assert!(
            batched.bytes() == one_at_a_time.bytes(),
            "one pass over two runs is the same raster as a pass each"
        );
    }

    /// And every run is drawn, not just the first: dropping either one changes
    /// the picture, so the batch cannot be satisfied by half of its argument.
    #[test]
    fn every_run_reaches_the_raster() {
        const SIZE: f32 = 24.0;
        let runs = [("Left", (12.0, 40.0)), ("Right", (160.0, 96.0))];
        let font = Font::sans();

        let mut both = canvas();
        draw_runs(&mut both, font, SIZE, Rgba::WHITE, &runs);

        for run in &runs {
            let mut alone = canvas();
            draw_runs(
                &mut alone,
                font,
                SIZE,
                Rgba::WHITE,
                std::slice::from_ref(run),
            );
            assert!(
                inked(&alone) > 0 && inked(&alone) < inked(&both),
                "`{}` is one of two runs, so it inks fewer pixels than the pair",
                run.0
            );
        }
    }
}
