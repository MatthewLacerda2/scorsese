//! The CPU compositor, on tiny-skia.
//!
//! CPU first because it is debuggable, platform-independent, and produces the
//! same pixels everywhere — which is what makes it the reference a GPU backend
//! can later be held to.

use scorsese_core::{Anchor, AnchorX, AnchorY};
use tiny_skia::{BlendMode, FilterQuality, PixmapMut, PixmapPaint, PixmapRef, Transform};

use crate::compose::{CompositeError, Compositor, Layer};
use crate::frame::{BYTES_PER_PIXEL, Frame};
use crate::grade;
use crate::properties::Properties;

/// Composites on the CPU.
#[derive(Debug, Default)]
pub struct CpuCompositor {
    scratch: Scratch,
}

/// The two copies of a layer this compositor may have to make, kept between
/// frames so a render allocates them once rather than eight megabytes thirty
/// times a second.
///
/// Two rather than one because a layer can need both, in this order: a graded
/// layer is graded in **straight** alpha — that is what the arithmetic is
/// written against — and premultiplying is the last thing that happens before
/// the rasteriser sees it. Grading a premultiplied pixel would scale the colour
/// by its own transparency and call the result a colour.
#[derive(Debug, Default)]
struct Scratch {
    /// The layer with its grade applied, still straight RGBA.
    graded: Vec<u8>,
    /// Colour channels multiplied by alpha, which is the form tiny-skia blends
    /// in.
    premultiplied: Vec<u8>,
}

impl CpuCompositor {
    /// One with no scratch buffers yet. They grow to fit the first layer that
    /// needs each and are reused for every frame after that, so a compositor is
    /// worth keeping across a render rather than per frame.
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
    scratch: &mut Scratch,
    canvas: &mut Frame,
    layer: &Layer<'_>,
) -> Result<(), CompositeError> {
    let source_resolution = layer.source.resolution();
    // Destructured so the two buffers can be borrowed at once: the graded copy
    // is read while the premultiplied one is written.
    let Scratch {
        graded,
        premultiplied,
    } = scratch;
    // The grade runs on the layer's own pixels, before anything below moves
    // them: a vignette is measured from this rectangle's centre, and a
    // saturation is about these pixels rather than the canvas they land on.
    let straight: &[u8] = if layer.properties.grade.is_neutral() {
        layer.source.bytes()
    } else {
        grade::into(graded, layer.source, layer.properties.grade);
        graded.as_slice()
    };
    // An opaque frame is already in the form the rasteriser wants; anything
    // else has to be premultiplied first.
    let source_bytes: &[u8] = if layer.source.is_opaque() {
        straight
    } else {
        premultiply_into(premultiplied, straight);
        premultiplied.as_slice()
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
    let transform = transform_of(
        &layer.properties,
        layer.anchor,
        source_resolution,
        canvas.resolution(),
    );

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
/// **Position is a fraction of the canvas, and this is where it becomes
/// pixels** — x against the canvas width, y against its height. Resolution is
/// a render setting, so a layer nudged by a count of pixels would sit
/// somewhere else the moment the same project was delivered at another size.
/// Each axis resolves against its own dimension rather than both against the
/// height: placement is what position is mostly used for, and `0.5` in x
/// reaching the edge of a 16:9 frame reads more plainly than a diagonal that
/// keeps its angle across aspect ratios.
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
/// **A flip needs nothing here, and that is the point of it.** Turning the
/// layer about one of its own axes squashes it along the other by `cos θ`, and
/// `S` is already a scale about the layer's own centre — so a flip arrives
/// folded into `sx` and `sy` by [`Properties::effective_scale`] and this
/// matrix is the matrix it always was. Folding it into `S` rather than
/// applying it after `R` is also what makes the axis the *layer's* and not the
/// frame's: a layer already turned 30° flips about its own edge, which is the
/// only reading of "its vertical axis" that survives a rotation. At `180°` the
/// factor is `−1`, so the same matrix mirrors about the same centre, which is
/// exactly what the back of a card is — there is no backface branch because
/// there is nothing for one to do.
///
/// [`Properties::effective_scale`]: crate::Properties::effective_scale
///
/// **The anchor decides where the layer rests**, and nothing else about it. A
/// centred anchor is the arithmetic this always did; `left` rests the layer's
/// left edge on the frame's, `right` rests its right edge on the frame's right.
/// That makes a positive offset mean "further in" on both sides, so the same
/// number is the same margin whichever edge it is measured from — which is what
/// lets a layout be flipped by changing one word.
///
/// A layer the size of the canvas rests at the origin whatever its anchor,
/// since every edge already meets the matching one. That is what keeps every
/// existing reference frame meaning what it meant — and it is a fact about the
/// **geometry**, not about which fit mode produced it. An anchor is a no-op
/// exactly when there is no spare: always for `fill`, which covers the raster
/// by construction, and for a `native` or `fit` layer only when its aspect
/// happens to match the render's. A letterboxed `fit` — 1920×1080 inside
/// 1920×1440 — has 360 pixels of spare, and an anchor rests the picture on the
/// edge of it. That case reaches here only because the decode stage hands over
/// the fitted picture's own rectangle rather than padding it out to the raster
/// first; padded, the spare is inside the layer's alpha where this cannot see
/// it.
pub(crate) fn transform_of(
    properties: &Properties,
    anchor: Anchor,
    source: crate::frame::Resolution,
    canvas: crate::frame::Resolution,
) -> Transform {
    // Resolved in f64 and cast once: the flip's cosine is the one factor here a
    // single-precision cosine could round visibly, and near edge-on it is the
    // difference between a sliver and a smear.
    let (scale_x, scale_y) = properties.effective_scale();
    let (scale_x, scale_y) = (scale_x as f32, scale_y as f32);
    let centre_x = source.width() as f32 / 2.0;
    let centre_y = source.height() as f32 / 2.0;
    // Rounded to whole pixels: a layer an odd number of pixels narrower than the
    // canvas cannot sit exactly in the middle of it, and half a pixel out is a
    // bilinear smear across every edge in the layer. Crisp beats exact when the
    // difference is invisible and the cost is the whole layer going soft. Only
    // the centred case can land on a half pixel; the edges are exact.
    let spare_x = canvas.width() as f32 - source.width() as f32;
    let spare_y = canvas.height() as f32 - source.height() as f32;
    let rest_x = match anchor.x {
        AnchorX::Left => 0.0,
        AnchorX::Center => (spare_x / 2.0).round(),
        AnchorX::Right => spare_x,
    };
    let rest_y = match anchor.y {
        AnchorY::Top => 0.0,
        AnchorY::Center => (spare_y / 2.0).round(),
        AnchorY::Bottom => spare_y,
    };
    // Not rounded, unlike the resting place above: a fraction of the raster
    // lands between pixels far more often than it lands on one, and a slow
    // drift stepping a whole pixel at a time is a worse artefact than the
    // bilinear softness that rounding avoids. Resting is static, where the
    // softness buys nothing; a position is usually going somewhere.
    // Negated on the far edges, so "further in" is positive on both sides and
    // the same number is the same margin whichever edge it was measured from.
    // Without this, flipping a layout would mean negating every offset, which
    // is the hand arithmetic anchors exist to remove.
    let inward_x = if anchor.x == AnchorX::Right {
        -1.0
    } else {
        1.0
    };
    let inward_y = if anchor.y == AnchorY::Bottom {
        -1.0
    } else {
        1.0
    };
    let offset_x = properties.position.0 as f32 * canvas.width() as f32 * inward_x;
    let offset_y = properties.position.1 as f32 * canvas.height() as f32 * inward_y;
    let (sin, cos) = (properties.rotation as f32).to_radians().sin_cos();
    // The linear part, R·S, in tiny-skia's row order: `from_row(sx, ky, kx, sy,
    // …)` maps `x' = sx·x + kx·y + tx` and `y' = ky·x + sy·y + ty`.
    let (a, kx) = (scale_x * cos, -scale_y * sin);
    let (ky, d) = (scale_x * sin, scale_y * cos);
    Transform::from_row(
        a,
        ky,
        kx,
        d,
        rest_x + centre_x - (a * centre_x + kx * centre_y) + offset_x,
        rest_y + centre_y - (ky * centre_x + d * centre_y) + offset_y,
    )
}

/// Multiplies colour channels by alpha, which is the form tiny-skia blends in.
fn premultiply_into(scratch: &mut Vec<u8>, source: &[u8]) {
    scratch.clear();
    scratch.reserve(source.len());
    for pixel in source.chunks_exact(BYTES_PER_PIXEL) {
        let alpha = u32::from(pixel[3]);
        let scale = |channel: u8| ((u32::from(channel) * alpha + 127) / 255) as u8;
        scratch.extend_from_slice(&[scale(pixel[0]), scale(pixel[1]), scale(pixel[2]), pixel[3]]);
    }
}
