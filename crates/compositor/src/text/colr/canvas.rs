//! The raster a colour glyph is walked onto, and the stacks the walk keeps.
//!
//! `skrifa::color::ColorPainter` is a sequence of callbacks — push a
//! transform, clip to this glyph, fill with that brush, open a layer, close it
//! in that compositing mode — and this is the end of them. Everything about it
//! is a **stack discipline**: what is pushed comes off in the reverse order,
//! and what is in force is the last thing pushed, already combined with
//! everything outside it.
//!
//! That is why the transform stack holds matrices already concatenated and the
//! clip stack holds masks already intersected. The alternative — keeping the
//! pieces and combining them at each fill — is the same answer computed once
//! per fill instead of once per push, and it is the version that gets the order
//! wrong, because the order is then written out at every use rather than once
//! here.
//!
//! **Nothing here may unbalance a stack.** A pop that finds nothing, a layer
//! that could not be allocated, a clip box with no area: each of those is a
//! thing this cannot draw, and every one of them leaves the stacks exactly as
//! deep as the walk believes they are. A painter that dropped a push would
//! draw the *rest* of the glyph wrong, which is a far worse failure than the
//! layer it could not make.

use skrifa::GlyphId;
use skrifa::color::{Brush, ColorPainter, CompositeMode, Transform as Paint};
use skrifa::raw::types::BoundingBox;
use tiny_skia::{
    FillRule, Mask, Paint as Brushed, Path, PathBuilder, Pixmap, PixmapPaint, Rect, Transform,
};

use scorsese_core::Rgba;

use crate::text::font::Face;

use super::brush::{Palette, Unpaintable, shader};
use super::{Trace, affine, blend};

/// The raster a colour glyph is walked onto, and the stacks the walk keeps.
pub(super) struct Canvas<'a, 'f> {
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

impl<'a, 'f> Canvas<'a, 'f> {
    /// A canvas over `into`, with the glyph's origin at `origin` on the raster
    /// and `foreground` standing for the colour the caption is set in.
    ///
    /// The walk reports in font units with y upwards, and a baseline is a row
    /// of the raster with y downwards — so the scale that takes one to the
    /// other is negative in y, and the origin lands where the pen is. That is
    /// the base transform, and it is the one thing on the stack that is never
    /// popped.
    pub(super) fn new(
        into: &'a mut Pixmap,
        face: &'a Face<'f>,
        origin: (f32, f32),
        foreground: Rgba,
        said: &'a mut Vec<Unpaintable>,
    ) -> Self {
        let scale = face.scale();
        let base = Transform::from_row(scale, 0.0, 0.0, -scale, origin.0, origin.1);
        let size = (into.width(), into.height());
        Self {
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
        }
    }

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
        let mut pen = Trace::default();
        self.face.unscaled(id, &mut pen);
        pen.finish()?.transform(self.matrix())
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
        let space = over.map_or_else(
            || self.matrix(),
            |extra| self.matrix().pre_concat(affine(extra)),
        );
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
        self.transforms
            .push(self.matrix().pre_concat(affine(transform)));
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

/// A raster size tiny-skia will accept, since a `Mask` cannot be zero in either
/// direction and a frame in principle can.
fn into_size(size: (u32, u32)) -> tiny_skia::IntSize {
    tiny_skia::IntSize::from_wh(size.0.max(1), size.1.max(1))
        .unwrap_or_else(|| tiny_skia::IntSize::from_wh(1, 1).expect("a one-pixel size is legal"))
}

#[cfg(test)]
mod tests;
