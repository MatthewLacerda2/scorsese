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
//!
//! **Some glyphs are drawings rather than shapes**, and those take the other
//! road: an emoji carries its own colours in layers and cannot be a stretch of
//! one path filled in one colour. [`super::colr`] walks it onto a scratch
//! raster, and the two halves of a block — the letters as a path, the drawings
//! as pixels — meet at [`Ink::stamp`].

use skrifa::outline::OutlinePen;
use tiny_skia::{PathBuilder, Pixmap};

use scorsese_core::Rgba;

use crate::frame::{Frame, Resolution};
use crate::paint;

use super::Edge;
use super::colr::{self, Unpaintable};
use super::font::{Faces, Font};
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
///
/// What comes back is whatever a colour glyph on the line asked for and could
/// not be given — empty for every line of letters, which is almost all of them.
pub fn draw_line(
    frame: &mut Frame,
    text: &str,
    font: &Font,
    size: f32,
    color: Rgba,
    origin: (f32, f32),
) -> Vec<Unpaintable> {
    let faces = font.faces(size.max(1.0));
    let mut ink = Ink::new(frame.resolution(), color, None);
    ink.line(&faces, &faces.shape(text), origin);
    ink.stamp(frame)
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
) -> Vec<Unpaintable> {
    let faces = font.faces(size.max(1.0));
    let mut ink = Ink::new(frame.resolution(), color, None);
    for (text, origin) in runs {
        ink.line(&faces, &faces.shape(text.as_ref()), *origin);
    }
    ink.stamp(frame)
}

/// What a block of text draws, collected before any of it reaches the frame.
///
/// Two halves that cannot be one. The letters are outlines filled in a single
/// colour, so they are one path entered into the rasteriser once — overlapping
/// letters blend at their seam rather than over each other. A colour glyph is
/// its own layered drawing in its own colours, so it is walked onto a scratch
/// raster instead, and only where a block actually has one: the pixmap is a
/// full frame of memory and a caption with no emoji in it never allocates one.
pub(super) struct Ink {
    outlines: Outlines,
    /// The raster a colour glyph would be walked onto, kept so that making one
    /// stays the first colour glyph's business rather than every block's.
    resolution: Resolution,
    /// The colour glyphs so far, `None` until the block turns out to have one.
    colours: Option<Pixmap>,
    /// What the letters are filled in, and what a colour glyph gets when it
    /// asks for the text's own colour by name.
    colour: Rgba,
    /// The rim grown off the letterforms, if the style asked for one.
    ///
    /// **The letters only.** A rim is a stroke of the path being filled, and a
    /// colour glyph is not that path — it is a drawing of its own, in its own
    /// colours, with no single outline to grow anything off. So a 🔥 in a
    /// captioned line is drawn as itself while the letters beside it keep
    /// their rim, which is also what it should look like: an emoji is already
    /// its own high-contrast shape and a rim round it would read as a sticker
    /// border rather than as legibility.
    edge: Option<Edge>,
    said: Vec<Unpaintable>,
}

impl Ink {
    pub(super) fn new(resolution: Resolution, colour: Rgba, edge: Option<Edge>) -> Self {
        Self {
            outlines: Outlines::default(),
            resolution,
            colours: None,
            colour,
            edge,
            said: Vec::new(),
        }
    }

    /// Traces an already-shaped run, with the run starting at `origin`.
    ///
    /// Shaped rather than plain text because where a glyph goes was decided
    /// upstream — the shaper applied the face's kerning, and a run that was
    /// measured to fit a line has to be drawn as the same run it was measured
    /// as. Which face each glyph came from was decided there too.
    pub(super) fn line(&mut self, faces: &Faces<'_>, shaped: &Shaped, origin: (f32, f32)) {
        let (left, baseline) = origin;
        for glyph in &shaped.glyphs {
            // The run's offsets are font-space — y upwards — and a baseline is
            // a row of the raster, so a glyph lifted off the baseline moves up
            // the frame, which is towards zero.
            let at = (left + glyph.at.0, baseline - glyph.at.1);
            let face = faces.face(glyph.face);
            match face.colour(glyph.id) {
                Some(drawing) => {
                    if self.colours.is_none() {
                        self.colours =
                            Pixmap::new(self.resolution.width(), self.resolution.height());
                    }
                    if let Some(pixmap) = self.colours.as_mut() {
                        colr::paint(pixmap, face, &drawing, at, self.colour, &mut self.said);
                    }
                }
                None => {
                    self.outlines.place(at.0, at.1);
                    face.outline(glyph.id, &mut self.outlines);
                }
            }
        }
    }

    /// Fills everything collected onto the frame and hands back what could not
    /// be drawn as the font asked.
    ///
    /// Letters first and drawings over them. They do not overlap in any real
    /// caption — an advance is an advance — so this is an order rather than a
    /// decision, and it is the one that puts an emoji in front of a letter it
    /// was placed on top of instead of behind it.
    pub(super) fn stamp(self, frame: &mut Frame) -> Vec<Unpaintable> {
        stamp(frame, self.outlines, self.colour, self.edge);
        if let Some(pixmap) = &self.colours {
            paint::blend_pixmap(frame, pixmap);
        }
        self.said
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
fn stamp(frame: &mut Frame, outlines: Outlines, color: Rgba, edge: Option<Edge>) {
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
