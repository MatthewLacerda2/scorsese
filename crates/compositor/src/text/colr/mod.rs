//! Filling a glyph that is a drawing rather than a shape.
//!
//! Most glyphs are one outline and one colour: the shape comes out of `glyf`,
//! [`super::draw`] fills it in whatever the style said, and that is the whole
//! of it. An emoji is not that. A `COLR` glyph is a little tree of paint
//! operations — clip to this outline, fill it with that gradient, lay the
//! result over what is underneath in that compositing mode — and the colours
//! belong to the font rather than to the caption.
//!
//! **skrifa walks the tree and tiny-skia does the drawing.** `skrifa::color`
//! hands this module a sequence of callbacks and every one of them lands on the
//! same rasteriser everything else in the crate uses, which is the invariant
//! [`super`] states: the compositor never grows a second answer to what a soft
//! edge looks like. The glyph is filled into a scratch pixmap and blended onto
//! the frame by [`crate::paint`], exactly as a path is.
//!
//! **A colour glyph is not tinted.** The style's colour reaches it only where
//! the font asks for it by name — palette index `0xFFFF`, which is how a face
//! draws a symbol meant to take the text's colour. A fire is the fire's
//! colours whatever colour the caption is, which is what somebody typing 🔥
//! into an amber title expects and the only reading that is not a guess.
//!
//! Everything the walk asks for that could not be drawn as asked comes back as
//! an [`Unpaintable`], and the render says so in its report. Silence is the bug
//! font fallback exists to end; reintroducing it one layer down would be
//! absurd.
//!
//! **Three files, by what each answers to.** [`brush`] is the colours — `CPAL`
//! and the gradients, said in tiny-skia's words. [`canvas`] is the stacks — the
//! transform, clip and layer discipline the callbacks are. What is left here is
//! the entry point and the two translations that are pure tables: skrifa's
//! matrix in tiny-skia's row order, and the format's compositing modes in
//! tiny-skia's names.

use skrifa::color::{ColorGlyph, CompositeMode, Transform as Paint};
use skrifa::outline::OutlinePen;
use tiny_skia::{BlendMode, Path, PathBuilder, Pixmap, Transform};

use scorsese_core::Rgba;

use super::font::Face;

mod brush;
mod canvas;

pub use brush::Unpaintable;
pub(in crate::text) use brush::palette;

use brush::report;
use canvas::Canvas;

/// Draws `glyph`'s colour layers into `into`, with the glyph's origin at
/// `origin` on the raster.
///
/// `foreground` is the style's own colour, which the glyph uses only where it
/// asks for it. Anything the walk could not honour is appended to `said`.
pub(super) fn paint(
    into: &mut Pixmap,
    face: &Face<'_>,
    glyph: &ColorGlyph<'_>,
    origin: (f32, f32),
    foreground: Rgba,
    said: &mut Vec<Unpaintable>,
) {
    let mut canvas = Canvas::new(into, face, origin, foreground, said);
    if let Err(error) = glyph.paint(face.location(), &mut canvas) {
        report(
            said,
            format!("a colour glyph its own font describes wrongly: {error}"),
        );
    }
}

/// skrifa's matrix in tiny-skia's row order. Both are the same affine map —
/// `x' = xx·x + xy·y + dx` — written with the terms in a different order.
fn affine(matrix: Paint) -> Transform {
    Transform::from_row(
        matrix.xx, matrix.yx, matrix.xy, matrix.yy, matrix.dx, matrix.dy,
    )
}

/// The format's compositing modes in tiny-skia's names. Every one the format
/// defines has an equivalent; only a file naming one that does not exist
/// reaches the last arm.
fn blend(mode: CompositeMode, said: &mut Vec<Unpaintable>) -> BlendMode {
    match mode {
        CompositeMode::Clear => BlendMode::Clear,
        CompositeMode::Src => BlendMode::Source,
        CompositeMode::Dest => BlendMode::Destination,
        CompositeMode::SrcOver => BlendMode::SourceOver,
        CompositeMode::DestOver => BlendMode::DestinationOver,
        CompositeMode::SrcIn => BlendMode::SourceIn,
        CompositeMode::DestIn => BlendMode::DestinationIn,
        CompositeMode::SrcOut => BlendMode::SourceOut,
        CompositeMode::DestOut => BlendMode::DestinationOut,
        CompositeMode::SrcAtop => BlendMode::SourceAtop,
        CompositeMode::DestAtop => BlendMode::DestinationAtop,
        CompositeMode::Xor => BlendMode::Xor,
        CompositeMode::Plus => BlendMode::Plus,
        CompositeMode::Screen => BlendMode::Screen,
        CompositeMode::Overlay => BlendMode::Overlay,
        CompositeMode::Darken => BlendMode::Darken,
        CompositeMode::Lighten => BlendMode::Lighten,
        CompositeMode::ColorDodge => BlendMode::ColorDodge,
        CompositeMode::ColorBurn => BlendMode::ColorBurn,
        CompositeMode::HardLight => BlendMode::HardLight,
        CompositeMode::SoftLight => BlendMode::SoftLight,
        CompositeMode::Difference => BlendMode::Difference,
        CompositeMode::Exclusion => BlendMode::Exclusion,
        CompositeMode::Multiply => BlendMode::Multiply,
        CompositeMode::HslHue => BlendMode::Hue,
        CompositeMode::HslSaturation => BlendMode::Saturation,
        CompositeMode::HslColor => BlendMode::Color,
        CompositeMode::HslLuminosity => BlendMode::Luminosity,
        _ => {
            report(
                said,
                "a compositing mode this build does not know".to_owned(),
            );
            BlendMode::SourceOver
        }
    }
}

/// Collects an outline into a path, in the units it arrives in.
///
/// Unlike [`super::draw::Outlines`] it does not flip anything: a colour glyph
/// is walked under a matrix that already carries the flip, and doing it twice
/// would draw every emoji upside down.
#[derive(Default)]
struct Trace(PathBuilder);

impl Trace {
    /// The path traced so far, or `None` for an outline with nothing in it —
    /// which is what an empty glyph, or one the face could not draw, leaves.
    fn finish(self) -> Option<Path> {
        self.0.finish()
    }
}

impl OutlinePen for Trace {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.quad_to(cx, cy, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.0.close();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use skrifa::color::CompositeMode as Mode;

    use super::*;

    /// Every compositing mode the format defines, and the tiny-skia blend each
    /// one is.
    ///
    /// Written out rather than derived from [`blend`]: a table generated from
    /// the code would agree with it after a mistake exactly as well as before
    /// one. This is the specification's list, in the specification's order —
    /// `COLR`'s composite modes are numbered 0 to 27 — so a reader can check it
    /// against the document rather than against us.
    const TABLE: [(Mode, BlendMode); 28] = [
        (Mode::Clear, BlendMode::Clear),
        (Mode::Src, BlendMode::Source),
        (Mode::Dest, BlendMode::Destination),
        (Mode::SrcOver, BlendMode::SourceOver),
        (Mode::DestOver, BlendMode::DestinationOver),
        (Mode::SrcIn, BlendMode::SourceIn),
        (Mode::DestIn, BlendMode::DestinationIn),
        (Mode::SrcOut, BlendMode::SourceOut),
        (Mode::DestOut, BlendMode::DestinationOut),
        (Mode::SrcAtop, BlendMode::SourceAtop),
        (Mode::DestAtop, BlendMode::DestinationAtop),
        (Mode::Xor, BlendMode::Xor),
        (Mode::Plus, BlendMode::Plus),
        (Mode::Screen, BlendMode::Screen),
        (Mode::Overlay, BlendMode::Overlay),
        (Mode::Darken, BlendMode::Darken),
        (Mode::Lighten, BlendMode::Lighten),
        (Mode::ColorDodge, BlendMode::ColorDodge),
        (Mode::ColorBurn, BlendMode::ColorBurn),
        (Mode::HardLight, BlendMode::HardLight),
        (Mode::SoftLight, BlendMode::SoftLight),
        (Mode::Difference, BlendMode::Difference),
        (Mode::Exclusion, BlendMode::Exclusion),
        (Mode::Multiply, BlendMode::Multiply),
        (Mode::HslHue, BlendMode::Hue),
        (Mode::HslSaturation, BlendMode::Saturation),
        (Mode::HslColor, BlendMode::Color),
        (Mode::HslLuminosity, BlendMode::Luminosity),
    ];

    #[test]
    fn every_mode_the_format_defines_maps_to_the_blend_that_names_it() {
        let mut said = Vec::new();
        for (mode, blends_as) in TABLE {
            assert_eq!(blend(mode, &mut said), blends_as, "{mode:?}");
        }
        assert!(
            said.is_empty(),
            "every mode the format defines has an equivalent, so none of them \
             is unpaintable: {said:?}"
        );
    }

    #[test]
    fn no_two_modes_are_the_same_blend() {
        // What the table above is for. A translation that sent half the modes
        // to `SourceOver` would still produce a plausible coloured emoji, and
        // every end-to-end assertion in the crate would go on passing — so the
        // claim that has to be made explicitly is that the modes are told
        // apart at all.
        let blends: HashSet<String> = TABLE.iter().map(|(_, to)| format!("{to:?}")).collect();
        assert_eq!(blends.len(), TABLE.len());
    }

    #[test]
    fn a_mode_this_build_does_not_know_is_said_out_loud_and_drawn_over() {
        // Only a malformed file reaches this, and it is drawn rather than
        // dropped — but never silently, which is the rule the whole module is
        // downstream of.
        let mut said = Vec::new();
        assert_eq!(blend(Mode::Unknown, &mut said), BlendMode::SourceOver);
        assert_eq!(said.len(), 1, "and it said so: {said:?}");
    }

    #[test]
    fn a_matrix_keeps_the_map_it_describes_through_the_change_of_order() {
        // The one place a transposition would hide: both libraries list six
        // terms and neither lists them in the other's order. Six different
        // numbers and a point that is neither symmetric nor at the origin, so
        // no swapped pair can come out right by coincidence.
        let matrix = Paint {
            xx: 2.0,
            yx: 3.0,
            xy: 5.0,
            yy: 7.0,
            dx: 11.0,
            dy: 13.0,
        };
        let mut mapped = [tiny_skia::Point::from_xy(1.0, 10.0)];
        affine(matrix).map_points(&mut mapped);
        assert_eq!(
            mapped[0],
            tiny_skia::Point::from_xy(2.0 + 5.0 * 10.0 + 11.0, 3.0 + 7.0 * 10.0 + 13.0),
            "x' = xx·x + xy·y + dx and y' = yx·x + yy·y + dy"
        );
    }

    #[test]
    fn the_pen_traces_every_kind_of_segment_and_flips_nothing() {
        // Each callback is its own mutation: a `quad_to` that dropped its
        // control point or a `close` that did nothing would change a glyph's
        // shape without changing where it sits, which is what the pixel
        // assertions elsewhere measure.
        let mut pen = Trace::default();
        pen.move_to(10.0, 20.0);
        pen.line_to(30.0, 20.0);
        pen.quad_to(40.0, 40.0, 30.0, 60.0);
        pen.curve_to(20.0, 70.0, 15.0, 80.0, 10.0, 90.0);
        pen.close();
        let path = pen.finish().expect("four segments and a close are a path");
        assert_eq!(path.segments().count(), 5, "one verb per callback");
        let bounds = path.bounds();
        // y arrives 20 and stays 20: the flip lives in the matrix the walk
        // runs under, and doing it here as well would draw every emoji upside
        // down.
        assert_eq!((bounds.left(), bounds.top()), (10.0, 20.0));
        assert!(bounds.right() >= 30.0 && bounds.bottom() >= 90.0);
    }

    #[test]
    fn an_outline_with_nothing_in_it_is_no_path_at_all() {
        // What a glyph the face could not draw leaves, and the reason every
        // caller of `outline` is written to skip rather than to fill.
        assert!(Trace::default().finish().is_none());
    }
}
