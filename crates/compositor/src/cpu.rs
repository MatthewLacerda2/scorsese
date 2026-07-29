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
    /// One with no scratch buffer yet. It grows to fit the first layer that
    /// needs premultiplying and is reused for every frame after that, so a
    /// compositor is worth keeping across a render rather than per frame.
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
    let transform = transform_of(layer, source_resolution, canvas.resolution());

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

/// Rest the layer centred on the canvas, scale and turn it about its own
/// centre, then offset by its position.
///
/// Written out as one matrix rather than composed from four, because the
/// composition order of scale-and-rotate-about-a-point is the classic place to
/// introduce a bug that only shows up off-centre. The mapping is
/// `p' = rest + position + c + R·S·(p − c)`: scale first, then turn, both about
/// the layer's own centre `c`, and everything else translates after. So the
/// linear part is
///
/// ```text
///           ⎡ sx·cos θ   −sy·sin θ ⎤
///   R · S = ⎣ sx·sin θ    sy·cos θ ⎦
/// ```
///
/// and the translation is `rest + position + c − (R·S)·c`.
///
/// **At θ = 0 this is exactly the matrix it always was** — `cos 0 = 1`,
/// `sin 0 = 0`, so it reduces to `tx = rest + c·(1 − sx) + position` — which is
/// what keeps every existing reference frame meaning what it meant. That is
/// worth more than brevity here: a golden render that shifted by a pixel would
/// be indistinguishable from this change being wrong.
///
/// **Positive is clockwise**, because the raster's y runs downward: `(1, 0)`
/// maps to `(cos θ, sin θ)`, and a positive y is further down the frame.
///
/// A layer the size of the canvas rests at the origin, so this is the same
/// matrix it always was — which is what keeps every existing reference frame
/// meaning what it meant. A layer that is *not* canvas-sized is what
/// [`scorsese_core::Fit::Native`] produces, and centred is where it rests: the
/// alternative, the top-left corner, is an arbitrary edge of a raster the
/// project is not supposed to know the size of.
fn transform_of(
    layer: &Layer<'_>,
    source: crate::frame::Resolution,
    canvas: crate::frame::Resolution,
) -> Transform {
    let scale_x = layer.properties.scale.0 as f32;
    let scale_y = layer.properties.scale.1 as f32;
    let centre_x = source.width() as f32 / 2.0;
    let centre_y = source.height() as f32 / 2.0;
    // Rounded to whole pixels: a layer an odd number of pixels narrower than the
    // canvas cannot sit exactly in the middle of it, and half a pixel out is a
    // bilinear smear across every edge in the layer. Crisp beats exact when the
    // difference is invisible and the cost is the whole layer going soft.
    let rest_x = ((canvas.width() as f32 - source.width() as f32) / 2.0).round();
    let rest_y = ((canvas.height() as f32 - source.height() as f32) / 2.0).round();
    let (sin, cos) = (layer.properties.rotation as f32).to_radians().sin_cos();
    // The linear part, R·S, in tiny-skia's row order: `from_row(sx, ky, kx, sy,
    // …)` maps `x' = sx·x + kx·y + tx` and `y' = ky·x + sy·y + ty`.
    let (a, kx) = (scale_x * cos, -scale_y * sin);
    let (ky, d) = (scale_x * sin, scale_y * cos);
    Transform::from_row(
        a,
        ky,
        kx,
        d,
        rest_x + centre_x - (a * centre_x + kx * centre_y) + layer.properties.position.0 as f32,
        rest_y + centre_y - (ky * centre_x + d * centre_y) + layer.properties.position.1 as f32,
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
