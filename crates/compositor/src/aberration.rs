//! The colour fringing every piece of glass has and no computed picture does.
//!
//! A lens does not bring every wavelength to the same place. Red lands a
//! fraction further from the optical axis than green and blue a fraction
//! nearer, so the three channels are recorded at very slightly different
//! magnifications — nothing at the centre of frame, worst at the edges. That
//! is **radial chromatic aberration**, and its total absence is one of the
//! quiet tells of an image that was computed rather than photographed. Grain
//! is the film; this is the lens.
//!
//! **One number**, deliberately, and the whole of what it means is a scale on
//! the vector from the layer's own centre: red is sampled at `(1 − s)` of that
//! distance and blue at `(1 + s)`, so red lands **outward** of where it was
//! and blue inward. That is the direction stated rather than discovered — the
//! visible signature is a warm fringe on the outer side of a bright edge and a
//! cool one on the inner side.
//!
//! **Green does not move.** It is the conventional choice and the cheap one:
//! two samples a pixel instead of three, and the channel carrying most of the
//! luma stays exactly where the rest of the compositor put it, so the picture
//! does not soften or shift as a whole — only its colours separate.
//!
//! **Bilinear, and it has to be.** The displacement here is a fraction of a
//! pixel over most of the raster, and nearest-neighbour sampling of a fraction
//! of a pixel is a staircase: the fringe would appear all at once along the
//! contour where the offset crosses a half pixel, which is exactly the
//! high-contrast edge this effect is most visible on.
//!
//! **Premultiplied pixels in, premultiplied pixels out**, for [`crate::blur`]'s
//! reason and one of its own. Averaging straight RGBA drags the colour stored
//! under transparent pixels into the visible edge; and because red and blue
//! arrive from pixels with alpha of their own, the only form in which "there is
//! no red over there" is a value rather than an unstated question is the one
//! where a transparent pixel contributes nothing.

use crate::frame::{BYTES_PER_PIXEL, Resolution};

/// The scale on the distance from the centre, from the fraction on the clip.
///
/// The clip's number is **a fraction of the layer's own height**, measured
/// where the layer's height is the thing to measure against: at the top and
/// bottom edges, half the height out from the centre. So `0.001` on a
/// 1080-tall layer moves red about a pixel there, and blue a pixel the other
/// way — which is why the factor below is two and not one.
///
/// **Under half a pixel anywhere is nothing**, and says so by returning zero,
/// so a keyframe track ramping up from `0.0` does not resample a whole layer to
/// move no pixel anywhere. "Anywhere" is the corner, which is the furthest any
/// pixel sits from the centre and so where the displacement is largest.
///
/// Negative is not aberration in the other direction — there being no such
/// thing to an author holding one number — and neither is a non-number.
pub(crate) fn spread(aberration: f64, resolution: Resolution) -> f64 {
    let scale = 2.0 * aberration;
    // NaN named rather than left to the comparison: it compares false against
    // everything, so without this it would fall through and be multiplied into
    // every coordinate on the raster instead of leaving by the same door every
    // other number that splits nothing leaves by.
    if scale.is_nan() || scale <= 0.0 {
        return 0.0;
    }
    let (width, height) = (
        f64::from(resolution.width()),
        f64::from(resolution.height()),
    );
    let corner = (width * width + height * height).sqrt() / 2.0;
    if scale * corner < 0.5 { 0.0 } else { scale }
}

/// Splits `source`'s channels into `out` and hands back the result.
///
/// Returns `source` untouched when there is nothing to do, so the caller can
/// use the answer unconditionally — the same contract [`crate::blur::into`]
/// has, and for the same two cases: no displacement worth making, and a buffer
/// whose length disagrees with the resolution it claims, which is refused a few
/// lines later where every other malformed layer is.
pub(crate) fn into<'a>(
    out: &'a mut Vec<u8>,
    source: &'a [u8],
    resolution: Resolution,
    spread: f64,
) -> &'a [u8] {
    let (width, height) = (resolution.width() as usize, resolution.height() as usize);
    if spread <= 0.0 || source.len() != width * height * BYTES_PER_PIXEL {
        return source;
    }
    out.clear();
    out.reserve(source.len());
    // The centre of the raster in the same coordinates a pixel's centre is
    // written in below, so a 1×1 layer is all centre and moves nothing.
    let (centre_x, centre_y) = (width as f64 / 2.0, height as f64 / 2.0);
    for (index, pixel) in source.chunks_exact(BYTES_PER_PIXEL).enumerate() {
        let (x, y) = (index % width, index / width);
        let (dx, dy) = (x as f64 + 0.5 - centre_x, y as f64 + 0.5 - centre_y);
        // The two ends of the split: red nearer the centre than the pixel it
        // lands on, blue further. So red comes out displaced **outward** and
        // blue inward — a warm fringe on the outer side of a bright edge and a
        // cool one on the inner side.
        let at = |scale: f64| (centre_x + dx * scale, centre_y + dy * scale);
        let alpha = pixel[3];
        let red = sample(source, resolution, at(1.0 - spread), 0);
        let blue = sample(source, resolution, at(1.0 + spread), 2);
        // Clamped to the alpha this pixel keeps. A premultiplied channel above
        // its own alpha is not a colour, and tiny-skia is entitled to make
        // nonsense of one; it happens wherever a channel is fetched from a
        // pixel more opaque than the one it lands on, which is every soft edge.
        out.push(red.min(alpha));
        out.push(pixel[1]);
        out.push(blue.min(alpha));
        out.push(alpha);
    }
    out.as_slice()
}

/// One channel, bilinearly, at a point in pixel-centre coordinates — so `0.5`
/// is the centre of the first pixel and the value there is exactly its own.
///
/// **Clamped at the edges**, like the blur's window: a sample off the raster
/// reads the nearest pixel on it. Letting it read black instead would draw a
/// dark rim round every strongly aberrated layer, which is a border rather than
/// a fringe.
fn sample(source: &[u8], resolution: Resolution, point: (f64, f64), channel: usize) -> u8 {
    let (width, height) = (resolution.width() as usize, resolution.height() as usize);
    let (x, y) = (point.0 - 0.5, point.1 - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let column = |at: f64| (at.max(0.0) as usize).min(width - 1);
    let row = |at: f64| (at.max(0.0) as usize).min(height - 1);
    let (left, right) = (column(x0), column(x0 + 1.0));
    let (top, bottom) = (row(y0), row(y0 + 1.0));
    let at = |c: usize, r: usize| f64::from(source[(r * width + c) * BYTES_PER_PIXEL + channel]);
    let upper = at(left, top) + (at(right, top) - at(left, top)) * fx;
    let lower = at(left, bottom) + (at(right, bottom) - at(left, bottom)) * fx;
    (upper + (lower - upper) * fy).round() as u8
}
