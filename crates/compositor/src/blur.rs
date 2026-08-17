//! Softening a layer's own pixels, before anything places them.
//!
//! One number on a clip: `0.0` leaves it alone, small softens it, large takes
//! it to mush. The unit is **a fraction of the layer's own height**, resolved
//! to pixels here where the resolution is finally known — the same split every
//! other fractional measurement in this crate uses, and what keeps the same
//! number meaning the same softness at 1080p and at 4K.
//!
//! **A separable box blur, three times.** A box blur is a running average over
//! a window, so with a sliding sum each output pixel costs one addition and one
//! subtraction *regardless of how wide the window is* — a heavy blur costs
//! exactly what a light one does, which is the difference between a knob
//! somebody turns and one nobody turns past 2. Separable means the two
//! dimensions are done one after the other rather than as a square kernel, so
//! the cost is `2·n` reads a pixel instead of `n²`. Three of those in
//! succession converge on a Gaussian closely enough that nobody can tell the
//! two apart, which is why it is what everyone ships.
//!
//! **Premultiplied pixels in, premultiplied pixels out.** This is not a detail
//! of the arithmetic below, it is a requirement on the caller: averaging
//! straight RGBA drags the colour of fully transparent pixels into the visible
//! edge, and every layer with alpha comes out ringed in whatever colour its
//! transparent surround happened to be stored as — usually black. The
//! neighbourhood being averaged has to be one where a transparent pixel
//! contributes nothing, and premultiplied is exactly that form.
//!
//! **Clamped at the edges.** A pixel near the border averages a window that
//! runs off the layer, and the pixels off the end read as the nearest one on
//! it. The alternative — shrinking the window — makes the divisor vary along
//! the row for no visible benefit, and treating off-layer as black would draw
//! a dark border around every blurred layer.

use crate::frame::{BYTES_PER_PIXEL, Resolution};

/// How many horizontal-then-vertical rounds are run. Three is the number that
/// makes a box blur indistinguishable from a Gaussian.
const PASSES: usize = 3;

/// The two buffers a separable blur ping-pongs between.
///
/// Kept by the compositor between frames for the same reason the grade's
/// scratch is: at 1080p one of these is 8 MB, and allocating a pair of them
/// thirty times a second is pure churn.
#[derive(Debug, Default)]
pub(crate) struct Buffers {
    /// Where a horizontal pass writes, and what the vertical pass that follows
    /// it reads.
    front: Vec<u8>,
    /// Where a vertical pass writes — and so, after the last of them, the
    /// blurred layer.
    back: Vec<u8>,
}

/// The blur in pixels, from the fraction on the clip and the layer's height.
///
/// **Under half a pixel is nothing**, and says so by returning zero, so a
/// keyframe track ramping up from `0.0` does not run six passes over a whole
/// layer to move no pixel anywhere. Rounded rather than truncated, so the
/// smallest blur that means anything is the smallest one that is asked for.
///
/// Never wider than the layer is tall: past that the window is entirely edge
/// clamp and the picture has already gone to a single average colour, so a
/// larger number costs work to change nothing.
pub(crate) fn radius(blur: f64, height: u32) -> usize {
    let pixels = blur * f64::from(height);
    // NaN named rather than left to the comparison: it compares false against
    // everything, so without this it would fall through to a cast instead of
    // out of the door with every other number that softens nothing.
    if pixels.is_nan() || pixels < 0.5 {
        return 0;
    }
    (pixels.round() as usize).min(height as usize)
}

/// Blurs `source` into the buffers and hands back the result.
///
/// Returns `source` untouched when there is nothing to do, so the caller can
/// use the answer unconditionally: a zero radius, and a buffer whose length
/// disagrees with the resolution it claims — which is not this function's to
/// report, and is refused a few lines later where every other malformed layer
/// is.
pub(crate) fn into<'a>(
    buffers: &'a mut Buffers,
    source: &'a [u8],
    resolution: Resolution,
    radius: usize,
) -> &'a [u8] {
    let (width, height) = (resolution.width() as usize, resolution.height() as usize);
    if radius == 0 || source.len() != width * height * BYTES_PER_PIXEL {
        return source;
    }
    {
        let Buffers { front, back } = &mut *buffers;
        front.clear();
        front.resize(source.len(), 0);
        back.clear();
        back.resize(source.len(), 0);
        for round in 0..PASSES {
            // Every round after the first reads what the last vertical pass
            // left behind, which is the whole of the ping-pong.
            let input: &[u8] = if round == 0 { source } else { back };
            // A row is `width` pixels one after another; rows are `width`
            // pixels apart.
            pass(input, front, height, width, width, 1, radius);
            // A column is `height` pixels `width` apart; columns are one pixel
            // apart.
            pass(front, back, width, height, 1, width, radius);
        }
    }
    &buffers.back
}

/// One running-average pass along one axis.
///
/// The axis is described rather than branched on: pixel `j` of line `i` sits at
/// `i · line_step + j · step`, which is a row when `step` is one and a column
/// when it is the width. One function means the horizontal and vertical halves
/// cannot drift apart, and the vertical one is where an off-by-one would be
/// hardest to see.
fn pass(
    source: &[u8],
    out: &mut [u8],
    lines: usize,
    length: usize,
    line_step: usize,
    step: usize,
    radius: usize,
) {
    let window = (2 * radius + 1) as u32;
    // Added before the divide, so the average rounds to nearest rather than
    // always down — which over six passes would drag the whole layer darker.
    let bias = window / 2;
    let last = length - 1;
    for line in 0..lines {
        let base = line * line_step;
        // The clamp that makes the edges behave: an index off either end of the
        // line reads the pixel at that end. `min` covers the far edge, and the
        // `saturating_sub` below covers the near one.
        let at = |j: usize| (base + j.min(last) * step) * BYTES_PER_PIXEL;

        // The window at the first pixel: `radius + 1` copies of the pixel at
        // the edge, standing in for everything off the start of the line, then
        // the `radius` real ones after it.
        let mut sum = [0_u32; BYTES_PER_PIXEL];
        for (channel, total) in sum.iter_mut().enumerate() {
            *total = u32::from(source[at(0) + channel]) * (radius as u32 + 1);
        }
        for j in 1..=radius {
            let entering = at(j);
            for (channel, total) in sum.iter_mut().enumerate() {
                *total += u32::from(source[entering + channel]);
            }
        }

        for j in 0..length {
            let write = (base + j * step) * BYTES_PER_PIXEL;
            // Slide to the next pixel's window: the one arriving at the far end
            // in, the one leaving the near end out.
            let entering = at(j + radius + 1);
            let leaving = at(j.saturating_sub(radius));
            for (channel, total) in sum.iter_mut().enumerate() {
                out[write + channel] = ((*total + bias) / window) as u8;
                *total += u32::from(source[entering + channel]);
                *total -= u32::from(source[leaving + channel]);
            }
        }
    }
}
