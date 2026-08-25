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

use skrifa::GlyphId;
use skrifa::color::{Brush, ColorGlyph, ColorPainter, CompositeMode, Transform as Paint};
use skrifa::outline::OutlinePen;
use skrifa::raw::types::BoundingBox;
use tiny_skia::{
    BlendMode, FillRule, Mask, Paint as Brushed, Path, PathBuilder, Pixmap, PixmapPaint, Rect,
    Transform,
};

use scorsese_core::Rgba;

use super::font::Face;

mod brush;

pub use brush::Unpaintable;
pub(in crate::text) use brush::palette;

use brush::{Palette, report, shader};

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
    // The walk reports in font units with y upwards, and a baseline is a row of
    // the raster with y downwards — so the scale that takes one to the other is
    // negative in y, and the origin lands where the pen is.
    let scale = face.scale();
    let base = Transform::from_row(scale, 0.0, 0.0, -scale, origin.0, origin.1);
    let size = (into.width(), into.height());
    let mut canvas = Canvas {
        into,
        layers: Vec::new(),
        transforms: vec![base],
        clips: Vec::new(),
        face,
        palette: Palette {
            entries: face.palette(),
            foreground,
        },
        size,
        said,
    };
    if let Err(error) = glyph.paint(face.location(), &mut canvas) {
        report(said, format!("a colour glyph its own font describes wrongly: {error}"));
    }
}

/// The raster a colour glyph is walked onto, and the stacks the walk keeps.
struct Canvas<'a, 'f> {
    into: &'a mut Pixmap,
    /// Layers opened by a compositing paint, innermost last. `None` is one that
    /// could not be allocated, which draws into whatever is beneath it rather
    /// than unbalancing the stack.
    layers: Vec<Option<Pixmap>>,
    /// The current matrix is the last, already concatenated. Never empty: the
    /// base transform below it is never popped.
    transforms: Vec<Transform>,
    /// The clip in force is the last, already intersected with every one
    /// outside it. Empty is no clip at all.
    clips: Vec<Mask>,
    face: &'a Face<'f>,
    palette: Palette<'a>,
    size: (u32, u32),
    said: &'a mut Vec<Unpaintable>,
}

impl Canvas<'_, '_> {
    fn matrix(&self) -> Transform {
        *self
            .transforms
            .last()
            .expect("the base transform is never popped")
    }

    /// Fills a path already in raster space, through whatever clip is in force,
    /// onto whichever layer is being drawn into.
    fn fill_path(&mut self, path: &Path, paint: &Brushed<'_>) {
        let Self {
            into,
            layers,
            clips,
            ..
        } = self;
        let target: &mut Pixmap = match layers.iter_mut().rev().flatten().next() {
            Some(layer) => layer,
            None => into,
        };
        // Non-zero winding, which is what the format specifies and what an
        // outline with a counter in it needs to keep the counter open.
        target.fill_path(
            path,
            paint,
            FillRule::Winding,
            Transform::identity(),
            clips.last(),
        );
    }

    /// One glyph's outline in raster space, ready to fill or to clip with.
    fn outline(&self, id: GlyphId) -> Option<Path> {
        let mut pen = Trace(PathBuilder::new());
        self.face.unscaled(id, &mut pen);
        pen.0.finish()?.transform(self.matrix())
    }

    /// Narrows the clip in force by `path`, which is already in raster space.
    fn narrow(&mut self, path: &Path) {
        let mut mask = match self.clips.last() {
            Some(outer) => outer.clone(),
            // No clip yet means everything is drawable, which is an all-white
            // mask rather than the all-black one a fresh `Mask` is.
            None => {
                let area = (self.size.0 as usize) * (self.size.1 as usize);
                let Some(open) = Mask::from_vec(vec![u8::MAX; area], into_size(self.size)) else {
                    return;
                };
                open
            }
        };
        mask.intersect_path(path, FillRule::Winding, true, Transform::identity());
        self.clips.push(mask);
    }

    fn brushed(&mut self, brush: &Brush<'_>, over: Option<Paint>) -> Option<Brushed<'static>> {
        // A brush's own transform applies inside the current one: the gradient
        // is described in the space the shape is described in.
        let space = over.map_or_else(|| self.matrix(), |extra| self.matrix().pre_concat(affine(extra)));
        let shader = shader(brush, &self.palette, space, self.said)?;
        Some(Brushed {
            shader,
            anti_alias: true,
            ..Brushed::default()
        })
    }
}

impl ColorPainter for Canvas<'_, '_> {
    fn push_transform(&mut self, transform: Paint) {
        self.transforms.push(self.matrix().pre_concat(affine(transform)));
    }

    fn pop_transform(&mut self) {
        // The base transform stays: popping it would leave the walk with no
        // way back to the raster at all.
        if self.transforms.len() > 1 {
            self.transforms.pop();
        }
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        if let Some(path) = self.outline(glyph_id) {
            self.narrow(&path);
        }
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        let width = clip_box.x_max - clip_box.x_min;
        let height = clip_box.y_max - clip_box.y_min;
        let Some(rect) = Rect::from_xywh(clip_box.x_min, clip_box.y_min, width, height) else {
            return;
        };
        let Some(path) = PathBuilder::from_rect(rect).transform(self.matrix()) else {
            return;
        };
        self.narrow(&path);
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        // No shape of its own: this fills the clip in force, which is why the
        // rectangle is the whole raster.
        let Some(rect) = Rect::from_xywh(0.0, 0.0, self.size.0 as f32, self.size.1 as f32) else {
            return;
        };
        let Some(paint) = self.brushed(&brush, None) else {
            return;
        };
        self.fill_path(&PathBuilder::from_rect(rect), &paint);
    }

    fn fill_glyph(&mut self, glyph_id: GlyphId, brush_transform: Option<Paint>, brush: Brush<'_>) {
        // Overridden rather than left to the default, which would build a
        // full-raster mask for every fill. A glyph filled with its own brush is
        // what almost every layer of almost every emoji is.
        let Some(path) = self.outline(glyph_id) else {
            return;
        };
        let Some(paint) = self.brushed(&brush, brush_transform) else {
            return;
        };
        self.fill_path(&path, &paint);
    }

    fn push_layer(&mut self, _composite_mode: CompositeMode) {
        self.layers.push(Pixmap::new(self.size.0, self.size.1));
    }

    fn pop_layer_with_mode(&mut self, composite_mode: CompositeMode) {
        let Some(layer) = self.layers.pop() else {
            return;
        };
        let Some(layer) = layer else {
            return;
        };
        let paint = PixmapPaint {
            blend_mode: blend(composite_mode, self.said),
            ..PixmapPaint::default()
        };
        let Self { into, layers, .. } = self;
        let target: &mut Pixmap = match layers.iter_mut().rev().flatten().next() {
            Some(under) => under,
            None => into,
        };
        target.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
    }
}

/// skrifa's matrix in tiny-skia's row order. Both are the same affine map —
/// `x' = xx·x + xy·y + dx` — written with the terms in a different order.
fn affine(matrix: Paint) -> Transform {
    Transform::from_row(matrix.xx, matrix.yx, matrix.xy, matrix.yy, matrix.dx, matrix.dy)
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
            report(said, "a compositing mode this build does not know".to_owned());
            BlendMode::SourceOver
        }
    }
}

fn into_size(size: (u32, u32)) -> tiny_skia::IntSize {
    tiny_skia::IntSize::from_wh(size.0.max(1), size.1.max(1)).unwrap_or_else(|| {
        tiny_skia::IntSize::from_wh(1, 1).expect("a one-pixel size is legal")
    })
}

/// Collects an outline into a path, in the units it arrives in.
///
/// Unlike [`super::draw::Outlines`] it does not flip anything: a colour glyph
/// is walked under a matrix that already carries the flip, and doing it twice
/// would draw every emoji upside down.
struct Trace(PathBuilder);

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
