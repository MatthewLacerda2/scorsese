//! The per-pixel arithmetic of a lit layer: padding it, tinting a shadow and a
//! glow out of its blurred alpha, and stacking the three.
//!
//! All of it on premultiplied RGBA, which is what makes the stacking one line:
//! source-over in premultiplied form is `top + bottom · (1 − top alpha)` on
//! every channel alike, alpha included.

use crate::frame::{BYTES_PER_PIXEL, Resolution};

use super::{Cast, Halo};

/// `value · by / 255`, to nearest — the one rounding every product here uses,
/// so no channel drifts darker over a stack of three.
fn scale(value: u32, by: u32) -> u32 {
    (value * by + 127) / 255
}

/// `source` copied into the middle of a transparent raster `pad` pixels wider
/// on each side and `pad` taller on each end.
pub(super) fn pad(padded: &mut Vec<u8>, source: &[u8], resolution: Resolution, pad: (u32, u32)) {
    let (width, height) = (resolution.width() as usize, resolution.height() as usize);
    let (across, down) = (pad.0 as usize, pad.1 as usize);
    let row = width * BYTES_PER_PIXEL;
    let padded_row = (width + 2 * across) * BYTES_PER_PIXEL;
    padded.clear();
    padded.resize(padded_row * (height + 2 * down), 0);
    for (y, line) in source.chunks_exact(row).enumerate() {
        let start = (y + down) * padded_row + across * BYTES_PER_PIXEL;
        padded[start..start + row].copy_from_slice(line);
    }
}

/// The shadow into `lit`, which is still empty: `softened`'s alpha, moved by
/// the offset and tinted.
///
/// **Only the alpha is read.** A shadow is the shape of the layer and not its
/// colours — a red ball and a blue one cast the same shadow.
pub(super) fn shadow(lit: &mut [u8], softened: &[u8], resolution: Resolution, cast: Cast) {
    let (width, height) = (
        i64::from(resolution.width()),
        i64::from(resolution.height()),
    );
    let strength = (cast.strength * 255.0).round() as u32;
    let (dx, dy) = cast.offset;
    for (at, pixel) in lit.chunks_exact_mut(BYTES_PER_PIXEL).enumerate() {
        let at = at as i64;
        // The pixel whose shadow lands here is the one an offset behind it.
        let (x, y) = (at % width - dx, at / width - dy);
        if !(0..width).contains(&x) || !(0..height).contains(&y) {
            continue;
        }
        let alpha = u32::from(softened[(y * width + x) as usize * BYTES_PER_PIXEL + 3]);
        let alpha = scale(alpha, strength);
        for (channel, colour) in pixel.iter_mut().zip(cast.rgb) {
            *channel = scale(u32::from(colour), alpha) as u8;
        }
        pixel[3] = alpha as u8;
    }
}

/// The glow, from `spread` — the layer blurred by the glow's radius — laid over
/// whatever `lit` already holds.
///
/// In the layer's own colours, the spread pixel **is** the halo: its colour is
/// the neighbourhood's colours averaged, weighted by how solid each was, which
/// is exactly what light spilling off a two-colour icon looks like. The gain
/// multiplies it, alpha and colour together so the hue holds, until the alpha
/// reaches solid; past that the colour keeps brightening toward the layer's
/// own, and never past it, since a premultiplied channel cannot exceed its
/// alpha.
pub(super) fn glow(lit: &mut [u8], spread: &[u8], halo: Halo) {
    let gain = halo.gain;
    for (pixel, spread) in lit
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .zip(spread.chunks_exact(BYTES_PER_PIXEL))
    {
        let alpha = f32::from(spread[3]);
        if alpha == 0.0 {
            continue;
        }
        let lifted = (alpha * gain).min(255.0);
        let light: [u32; 4] = match halo.tint {
            None => {
                let lift = |channel: u8| (f32::from(channel) * gain).min(lifted).round() as u32;
                [
                    lift(spread[0]),
                    lift(spread[1]),
                    lift(spread[2]),
                    lifted.round() as u32,
                ]
            }
            Some([r, g, b, a]) => {
                let alpha = scale(lifted.round() as u32, u32::from(a));
                [
                    scale(u32::from(r), alpha),
                    scale(u32::from(g), alpha),
                    scale(u32::from(b), alpha),
                    alpha,
                ]
            }
        };
        stack(pixel, light);
    }
}

/// `top` over every pixel of `lit`.
pub(super) fn over(lit: &mut [u8], top: &[u8]) {
    for (pixel, top) in lit
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .zip(top.chunks_exact(BYTES_PER_PIXEL))
    {
        stack(pixel, [top[0], top[1], top[2], top[3]].map(u32::from));
    }
}

/// One premultiplied pixel over another, in place.
fn stack(bottom: &mut [u8], top: [u32; 4]) {
    let clear = 255 - top[3];
    for (channel, top) in bottom.iter_mut().zip(top) {
        *channel = (top + scale(u32::from(*channel), clear)).min(255) as u8;
    }
}
