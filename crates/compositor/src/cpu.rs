//! The CPU compositor, on tiny-skia.
//!
//! CPU first because it is debuggable, platform-independent, and produces the
//! same pixels everywhere — which is what makes it the reference a GPU backend
//! can later be held to.

use tiny_skia::{BlendMode, FilterQuality, PixmapMut, PixmapPaint, PixmapRef, Transform};

use crate::compose::{CompositeError, Compositor, Layer};
use crate::frame::{BYTES_PER_PIXEL, Frame};

/// Composites on the CPU.
#[derive(Debug, Default)]
pub struct CpuCompositor {
    /// A premultiplied copy of a layer whose alpha is not already solid. Kept
    /// between frames so a render allocates it once rather than per frame.
    scratch: Vec<u8>,
}

impl CpuCompositor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Compositor for CpuCompositor {
    fn composite(
        &mut self,
        canvas: &mut Frame,
        layers: &[Layer<'_>],
    ) -> Result<(), CompositeError> {
        canvas.fill_black();
        for layer in layers {
            if layer.properties.is_invisible() {
                continue;
            }
            if copied(canvas, layer) {
                continue;
            }
            draw(&mut self.scratch, canvas, layer)?;
        }
        Ok(())
    }
}

/// Copies a layer that would rasterise to exactly its own pixels.
///
/// Not only an optimisation: it means a render with no transforms produces
/// byte-identical output to one that never went through a compositor at all, so
/// the golden references for plain cuts keep meaning what they meant.
fn copied(canvas: &mut Frame, layer: &Layer<'_>) -> bool {
    let matches = layer.properties.is_identity()
        && layer.source.resolution() == canvas.resolution()
        && layer.source.is_opaque();
    if matches {
        canvas.bytes_mut().copy_from_slice(layer.source.bytes());
    }
    matches
}

fn draw(
    scratch: &mut Vec<u8>,
    canvas: &mut Frame,
    layer: &Layer<'_>,
) -> Result<(), CompositeError> {
    let source_resolution = layer.source.resolution();
    // An opaque frame is already in the form the rasteriser wants; anything
    // else has to be premultiplied first.
    let source_bytes: &[u8] = if layer.source.is_opaque() {
        layer.source.bytes()
    } else {
        premultiply_into(scratch, layer.source);
        scratch.as_slice()
    };
    let source = PixmapRef::from_bytes(
        source_bytes,
        source_resolution.width(),
        source_resolution.height(),
    )
    .ok_or(CompositeError::BadLayer {
        resolution: source_resolution,
        bytes: source_bytes.len(),
    })?;

    let paint = PixmapPaint {
        opacity: layer.properties.opacity.clamp(0.0, 1.0) as f32,
        // Bilinear: a scaled layer should not look like a mosaic, and anything
        // heavier buys nothing at the sizes a preview or a render works at.
        quality: FilterQuality::Bilinear,
        blend_mode: BlendMode::SourceOver,
    };
    let transform = transform_of(layer, source_resolution);

    let canvas_resolution = canvas.resolution();
    let bytes = canvas.byte_count();
    let mut destination = PixmapMut::from_bytes(
        canvas.bytes_mut(),
        canvas_resolution.width(),
        canvas_resolution.height(),
    )
    .ok_or(CompositeError::BadCanvas {
        resolution: canvas_resolution,
        bytes,
    })?;
    // The canvas began opaque and every blend here is source-over onto it, so
    // it stays opaque — which is why it needs no premultiplication in either
    // direction. Break that invariant and the colours come out wrong.
    destination.draw_pixmap(0, 0, source, &paint, transform, None);
    Ok(())
}

/// Scale about the layer's own centre, then offset.
///
/// Written out as one matrix rather than composed from three, because the
/// composition order of scale-about-a-point is the classic place to introduce a
/// bug that only shows up off-centre. Mapping `x' = sx·x + tx` directly, a
/// scale about centre `c` is `tx = c·(1 − sx)`, and position simply adds.
fn transform_of(layer: &Layer<'_>, source: crate::frame::Resolution) -> Transform {
    let scale_x = layer.properties.scale.0 as f32;
    let scale_y = layer.properties.scale.1 as f32;
    let centre_x = source.width() as f32 / 2.0;
    let centre_y = source.height() as f32 / 2.0;
    Transform::from_row(
        scale_x,
        0.0,
        0.0,
        scale_y,
        centre_x * (1.0 - scale_x) + layer.properties.position.0 as f32,
        centre_y * (1.0 - scale_y) + layer.properties.position.1 as f32,
    )
}

/// Multiplies colour channels by alpha, which is the form tiny-skia blends in.
fn premultiply_into(scratch: &mut Vec<u8>, source: &Frame) {
    scratch.clear();
    scratch.reserve(source.byte_count());
    for pixel in source.bytes().chunks_exact(BYTES_PER_PIXEL) {
        let alpha = u32::from(pixel[3]);
        let scale = |channel: u8| ((u32::from(channel) * alpha + 127) / 255) as u8;
        scratch.extend_from_slice(&[scale(pixel[0]), scale(pixel[1]), scale(pixel[2]), pixel[3]]);
    }
}
