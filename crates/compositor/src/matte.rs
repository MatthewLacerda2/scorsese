//! A track matte: the mask one layer is drawn through, cut from another
//! layer's picture.
//!
//! The matte layer is composited on its own onto a transparent raster the size
//! of the canvas, exactly as it would have landed on the frame, and the alpha
//! of what is left there becomes a [`Mask`] the masked layer is then drawn
//! through. Nothing here knows what the matte *is* — a rectangle, a title, a
//! group — or which stages it went through on the way; that is the point of
//! drawing it rather than describing it.
//!
//! Beside `light` rather than inside it, and after it: `light` is the last of
//! a layer's **own** stages and ends in the layer's own pixels, where a matte
//! needs the canvas — the matte layer and the masked one each have a transform,
//! and the only place the two meet is the frame they both land on.

use tiny_skia::Mask;

use crate::compose::{CompositeError, Matte};
use crate::frame::{BYTES_PER_PIXEL, Frame, Resolution};

/// The raster a matte is drawn onto and the mask made from it, kept between
/// frames for the reason every scratch buffer is: a canvas-sized allocation
/// per matted layer per frame is megabytes of churn.
#[derive(Debug, Default)]
pub(crate) struct Buffers {
    /// Where the matte layer is composited, transparent everywhere else.
    drawn: Option<Frame>,
    /// Its alpha, and nothing else — what the masked layer is drawn through.
    mask: Option<Mask>,
}

impl Buffers {
    /// Draws `matte` with `draw` onto a transparent canvas of `resolution` and
    /// hands back its alpha as a mask, inverted if the matte says so.
    ///
    /// `draw` is the compositor's own per-layer drawing, lent in so this
    /// module owns the buffers and the reading of alpha and nothing about how
    /// a layer is drawn. It is not called for an invisible matte: nothing is
    /// what it would have drawn, and the mask is all zero (all through, once
    /// inverted) without it.
    pub(crate) fn cut(
        &mut self,
        matte: &Matte<'_>,
        resolution: Resolution,
        draw: impl FnOnce(&mut Frame, &crate::Layer<'_>) -> Result<(), CompositeError>,
    ) -> Result<&Mask, CompositeError> {
        let drawn = match &mut self.drawn {
            Some(frame) if frame.resolution() == resolution => frame,
            slot => slot.insert(Frame::black(resolution)),
        };
        drawn.fill_transparent();
        if !matte.layer.properties.is_invisible() {
            draw(drawn, &matte.layer)?;
        }
        let fits = self.mask.as_ref().is_some_and(|mask| {
            mask.width() == resolution.width() && mask.height() == resolution.height()
        });
        if !fits {
            self.mask = Some(Mask::new(resolution.width(), resolution.height()).ok_or(
                CompositeError::BadCanvas {
                    resolution,
                    bytes: drawn.byte_count(),
                },
            )?);
        }
        let mask = self.mask.as_mut().expect("sized just above");
        alpha_into(mask.data_mut(), drawn.bytes(), matte.invert);
        Ok(mask)
    }
}

/// Copies each pixel's alpha out of RGBA `pixels` into `mask`, or its
/// complement when `invert`.
///
/// Alpha only, and the whole of it: a half-transparent edge of the matte is a
/// half-revealed edge of the layer, which is how a matte's own `blur` becomes
/// a feathered reveal with no property of its own.
fn alpha_into(mask: &mut [u8], pixels: &[u8], invert: bool) {
    for (coverage, pixel) in mask.iter_mut().zip(pixels.chunks_exact(BYTES_PER_PIXEL)) {
        *coverage = if invert { 255 - pixel[3] } else { pixel[3] };
    }
}

#[cfg(test)]
mod tests {
    use scorsese_core::Rgba;

    use super::{Buffers, alpha_into};
    use crate::compose::Matte;
    use crate::frame::{Frame, Resolution};

    /// Cuts an opaque matte at `width` × `height`, and answers where the
    /// raster it was drawn on and the mask it became live — the two buffers
    /// this type exists to keep. The drawing itself checks it was handed a
    /// raster of the size asked for.
    fn cut_at(buffers: &mut Buffers, width: u32, height: u32) -> (*const u8, *const u8) {
        let resolution = Resolution::new(width, height).expect("a legal raster");
        let source = Frame::black(resolution);
        let matte = Matte {
            layer: crate::Layer::plain(&source),
            invert: false,
        };
        let mut drawn_at = std::ptr::null();
        let mask = buffers
            .cut(&matte, resolution, |drawn, _| {
                assert_eq!(
                    drawn.resolution(),
                    resolution,
                    "drawn at the size asked for"
                );
                drawn.fill(Rgba::WHITE);
                drawn_at = drawn.bytes().as_ptr();
                Ok(())
            })
            .expect("cuts");
        assert_eq!((mask.width(), mask.height()), (width, height));
        assert!(mask.data().iter().all(|&a| a == 255), "the matte's alpha");
        (drawn_at, mask.data().as_ptr())
    }

    /// Reuse is the whole reason `Buffers` exists: a second matte at the same
    /// size lands in the same two allocations rather than two new ones.
    #[test]
    fn a_second_cut_at_the_same_size_reuses_both_buffers() {
        let mut buffers = Buffers::default();
        let first = cut_at(&mut buffers, 16, 8);
        assert_eq!(cut_at(&mut buffers, 16, 8), first);
    }

    /// And a buffer is never reused at the wrong size — changing one
    /// dimension at a time, so neither is enough on its own to pass for a fit.
    #[test]
    fn a_cut_at_another_size_draws_and_masks_at_that_size() {
        let mut buffers = Buffers::default();
        cut_at(&mut buffers, 16, 8);
        cut_at(&mut buffers, 16, 4);
        cut_at(&mut buffers, 8, 4);
    }

    #[test]
    fn the_mask_is_the_alpha_and_inverting_complements_it() {
        let pixels = [9, 9, 9, 0, 9, 9, 9, 128, 9, 9, 9, 255];
        let mut mask = [7; 3];
        alpha_into(&mut mask, &pixels, false);
        assert_eq!(mask, [0, 128, 255]);
        alpha_into(&mut mask, &pixels, true);
        assert_eq!(mask, [255, 127, 0]);
    }
}
