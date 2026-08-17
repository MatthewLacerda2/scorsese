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

/// The arithmetic, on buffers small enough to work out by hand.
///
/// Here rather than in `tests/blurring.rs` because that is where the *effect*
/// is asserted — an edge comes out soft, a corner does not go dark — and an
/// effect is satisfied by more than one arithmetic. The three functions above
/// take a buffer and a radius and produce bytes, so the value to name is the
/// bytes; naming them needs `pass` itself, which nothing outside this file can
/// reach. The golden fixtures `blur_soft`, `blur_heavy` and `blur_alpha` are the
/// other half of this and cost a render each; these cost nothing and run on
/// every pull request.
#[cfg(test)]
mod tests {
    use super::*;

    /// One pixel: the value in red, a constant in green so a channel reading its
    /// neighbour's byte is visible, nothing in blue, opaque.
    fn pixel(value: u8) -> [u8; BYTES_PER_PIXEL] {
        [value, 200, 0, u8::MAX]
    }

    /// A line of those, one per value.
    fn line(values: &[u8]) -> Vec<u8> {
        values.iter().flat_map(|&v| pixel(v)).collect()
    }

    /// One channel of a line, back out.
    fn channel(bytes: &[u8], channel: usize) -> Vec<u8> {
        bytes
            .iter()
            .skip(channel)
            .step_by(BYTES_PER_PIXEL)
            .copied()
            .collect()
    }

    /// One pass, worked out on paper. Eight pixels stepping from 40 to 240 in
    /// the middle, at a radius of three — a seven-wide window, so each output is
    /// the seven pixels around it averaged, with everything off either end
    /// reading the pixel at that end.
    ///
    /// The windows left to right sum to 280, 480, 680, 880, 1080, 1280, 1480 and
    /// 1680. Over seven that is 40, 68.57, 97.14, 125.71, 154.29, 182.86, 211.43
    /// and 240 — and **to nearest**, which is what the bias added before the
    /// divide buys and what the row below says: 69 and not 68, 126 and not 125.
    #[test]
    fn one_pass_averages_a_window_and_rounds_to_nearest() {
        let source = line(&[40, 40, 40, 40, 240, 240, 240, 240]);
        let mut out = vec![0; source.len()];
        pass(&source, &mut out, 1, 8, 8, 1, 3);

        assert_eq!(channel(&out, 0), [40, 69, 97, 126, 154, 183, 211, 240]);
        assert_eq!(channel(&out, 1), [200; 8], "a flat channel comes out flat");
        assert_eq!(channel(&out, 2), [0; 8]);
        assert_eq!(channel(&out, 3), [u8::MAX; 8], "and opaque stays opaque");
    }

    /// The two directions are one function, and this is the sentence that says
    /// so: a vertical pass over a buffer's transpose is the transpose of a
    /// horizontal pass over the buffer.
    ///
    /// Nothing in it depends on the numbers. What it pins is that `line_step`
    /// and `step` are the *whole* difference between the two directions — an
    /// arithmetic mistake in either is invisible along a row, where both are one
    /// or zero, and changes the answer down a column, where neither is.
    #[test]
    fn a_column_is_a_row_of_the_transpose() {
        const W: usize = 4;
        const H: usize = 3;
        let grid = [[10, 20, 30, 40], [50, 60, 70, 80], [90, 100, 110, 120]];

        let mut rows = Vec::new();
        for row in &grid {
            rows.extend(line(row));
        }
        let mut columns = Vec::new();
        for x in 0..W {
            for row in &grid {
                columns.extend(pixel(row[x]));
            }
        }

        // A row is `W` pixels one apart, `W` apart; a column of the transpose is
        // `W` pixels `H` apart, one apart — the two calls `into` itself makes.
        let mut blurred_rows = vec![0; rows.len()];
        pass(&rows, &mut blurred_rows, H, W, W, 1, 1);
        let mut blurred_columns = vec![0; columns.len()];
        pass(&columns, &mut blurred_columns, H, W, 1, H, 1);

        for y in 0..H {
            for x in 0..W {
                let along = (y * W + x) * BYTES_PER_PIXEL;
                let down = (x * H + y) * BYTES_PER_PIXEL;
                assert_eq!(
                    blurred_columns[down..down + BYTES_PER_PIXEL],
                    blurred_rows[along..along + BYTES_PER_PIXEL],
                    "pixel ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn half_a_pixel_is_the_narrowest_blur_there_is() {
        // Exactly half a pixel, and exactly is the word: 0.5/64 is representable
        // in binary and 64 is a power of two, so this is the boundary itself
        // rather than a number near it. Rounded and not truncated, so the
        // narrowest blur that does anything is the narrowest one asked for.
        assert_eq!(radius(0.5 / 64.0, 64), 1);
        assert_eq!(radius(0.49 / 64.0, 64), 0, "anything narrower is nothing");
        assert_eq!(radius(0.0, 64), 0, "and so is nothing");
        assert_eq!(radius(-1.0, 64), 0, "a negative number is not a blur");
        assert_eq!(radius(f64::NAN, 64), 0, "nor is a non-number");
        assert_eq!(radius(0.1, 64), 6, "a tenth of the height, in its pixels");
        assert_eq!(radius(4.0, 64), 64, "never wider than the layer is tall");
    }

    #[test]
    fn nothing_to_do_is_no_work_at_all() {
        let resolution = Resolution::new(8, 2).expect("a legal raster");
        let source = vec![7; 8 * 2 * BYTES_PER_PIXEL];
        let mut buffers = Buffers::default();

        assert_eq!(into(&mut buffers, &source, resolution, 0), &source[..]);
        assert!(
            buffers.back.is_empty(),
            "a zero radius runs no pass and allocates nothing"
        );

        // A buffer whose length disagrees with the resolution it claims is not
        // this function's to report — it is refused a few lines later, where
        // every other malformed layer is. So it comes back untouched rather than
        // being indexed off its end.
        let short = vec![7; 8 * BYTES_PER_PIXEL];
        assert_eq!(into(&mut buffers, &short, resolution, 3), &short[..]);
        assert!(buffers.back.is_empty());
    }

    /// How far a blur reaches, which is the only thing that says how many passes
    /// ran.
    ///
    /// Sixteen pixels stepping from 40 to 240 between 7 and 8, radius one, and
    /// two identical rows — a column of one value averages to itself, so the
    /// vertical passes are the identity here and what is left is the horizontal
    /// one, three times over.
    ///
    /// One pass reaches `radius` pixels either side of an edge; three reach
    /// `3·radius` and not a pixel further. So 4 is still the 40 it started as, 5
    /// has moved, 10 has moved, and 11 is back to a flat 240. A symmetric window
    /// also makes the ramp between them **antisymmetric**: two pixels the same
    /// distance either side of the edge sum to 40 + 240, which no partial number
    /// of passes and no off-centre window satisfies.
    #[test]
    fn three_passes_reach_exactly_three_radii() {
        let row = [
            40, 40, 40, 40, 40, 40, 40, 40, 240, 240, 240, 240, 240, 240, 240, 240,
        ];
        let mut source = line(&row);
        source.extend(line(&row));
        let resolution = Resolution::new(16, 2).expect("a legal raster");
        let mut buffers = Buffers::default();

        let blurred = into(&mut buffers, &source, resolution, 1).to_vec();
        let (top, bottom) = blurred.split_at(16 * BYTES_PER_PIXEL);
        assert_eq!(top, bottom, "the vertical passes moved nothing");

        let soft = channel(top, 0);
        assert_eq!(soft[4], 40, "three radii short of the edge, still flat");
        assert_eq!(soft[11], 240, "and three radii past it, flat again");
        assert!(soft[5] > 40 && soft[10] < 240, "found {soft:?}");
        for distance in 0..4 {
            let (left, right) = (soft[7 - distance], soft[8 + distance]);
            assert_eq!(
                u32::from(left) + u32::from(right),
                280,
                "{distance} either side of the edge: {left} and {right}"
            );
        }
    }
}
