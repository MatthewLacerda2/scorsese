//! Changing the size of a picture that carries alpha, in our own arithmetic.
//!
//! A source with alpha used to be resampled by ffmpeg, between a `premultiply`
//! and an `unpremultiply` filter. That is compositing done by ffmpeg, and it
//! cost what the architecture says it would: FFmpeg 8 changed `premultiply`'s
//! 8-bit rounding, 6.1's swscale rounded alpha on the way into it, and three
//! page goldens that pass under CI's ffmpeg failed under Arch's (#868). The
//! decoder now hands such a source over at its own size and this module does
//! the resample, so what a transparent picture looks like at another size is
//! decided here, the same on every machine and every ffmpeg.
//!
//! **Premultiplied, for the reason the filter pair was there.** A fully
//! transparent pixel's colour means nothing and is usually black; averaged in
//! straight alpha it draws a dark rim round every edge. Weighting each colour
//! by its own coverage first makes that pixel weigh nothing.
//!
//! **Separable bicubic (Catmull-Rom), widened when shrinking.** Shrinking by
//! six — a page captured at 960×540 shown on a 160×90 raster — needs a filter
//! six times as wide or it aliases; growing needs only the four nearest. The
//! weights are polynomials in `f64`, and the passes add in a fixed order in
//! `f32`, so the same input gives the same bytes on any platform: nothing here
//! calls a library `sin` or `exp`.
//!
//! **Into a window.** The output is a rectangle of the picture as if it had
//! been scaled to `scaled`, starting at `offset` within it. That one shape is
//! all three fittings: `fit` is the whole of it, `fill` a centred window inside
//! a larger picture, and a padded fit a window larger than the picture, whose
//! outside is transparent.

use crate::{BYTES_PER_PIXEL, Frame, Resolution};

/// A resample from one size to another, with its weights worked out once.
///
/// Built per decoder rather than per frame: every frame of a source is the
/// same size, so the weights are too, and only the pixels change. It holds
/// nothing a resample writes, so one is shared by every thread fitting that
/// source's frames (#880), each through its own compositor's [`Scratch`] —
/// see [`crate::CpuCompositor::resample`].
#[derive(Debug, Clone)]
pub struct Resample {
    source: Resolution,
    window: Resolution,
    columns: Taps,
    rows: Taps,
}

/// What a resample writes on the way, kept by whoever runs it so a render
/// allocates it once per thread rather than once per frame. It grows to fit
/// the largest resample it has been through; a smaller one uses the front.
#[derive(Debug, Default)]
pub(crate) struct Scratch {
    /// The source with its colour weighted by its alpha, one row at a time.
    row: Vec<f32>,
    /// The horizontal pass: every source row, at the window's width.
    across: Vec<f32>,
    /// The vertical pass, one output row at a time.
    down: Vec<f32>,
}

impl Scratch {
    /// Sized for `resample`, keeping whatever is already allocated. Nothing
    /// needs clearing: every value read is written first in the same pass.
    fn fit(&mut self, resample: &Resample) {
        let (source, window) = (resample.source, resample.window);
        let grow = |buffer: &mut Vec<f32>, len: usize| {
            if buffer.len() < len {
                buffer.resize(len, 0.0);
            }
        };
        grow(&mut self.row, source.width() as usize * BYTES_PER_PIXEL);
        grow(
            &mut self.across,
            source.height() as usize * window.width() as usize * BYTES_PER_PIXEL,
        );
        grow(&mut self.down, window.width() as usize * BYTES_PER_PIXEL);
    }
}

impl Resample {
    /// A resample of a `source`-sized picture, scaled to `scaled`, of which
    /// the `window`-sized rectangle starting at `offset` is kept. An offset can
    /// be negative and the window larger than the picture: whatever falls
    /// outside it is transparent.
    pub fn new(
        source: Resolution,
        scaled: Resolution,
        window: Resolution,
        offset: (i64, i64),
    ) -> Self {
        let columns = Taps::new(source.width(), scaled.width(), window.width(), offset.0);
        let rows = Taps::new(source.height(), scaled.height(), window.height(), offset.1);
        Self {
            source,
            window,
            columns,
            rows,
        }
    }

    /// The size a frame going in must be.
    pub const fn source(&self) -> Resolution {
        self.source
    }

    /// The size a frame coming out is.
    pub const fn window(&self) -> Resolution {
        self.window
    }

    /// Resamples `from`, straight alpha, into `into`, straight alpha, writing
    /// on the way only into `scratch`. Its panics are
    /// [`crate::CpuCompositor::resample`]'s.
    pub(crate) fn apply(&self, from: &Frame, into: &mut Frame, scratch: &mut Scratch) {
        assert_eq!(
            from.resolution(),
            self.source,
            "resampled from the wrong size"
        );
        assert_eq!(
            into.resolution(),
            self.window,
            "resampled into the wrong size"
        );
        scratch.fit(self);
        let Scratch { row, across, down } = scratch;
        let width = self.window.width() as usize * BYTES_PER_PIXEL;
        let down = &mut down[..width];
        let stride = self.source.width() as usize * BYTES_PER_PIXEL;
        for (y, pixels) in from.bytes().chunks_exact(stride).enumerate() {
            if !self.rows.reads(y) {
                continue;
            }
            premultiply(pixels, row);
            let out = &mut across[y * width..(y + 1) * width];
            for (x, sum) in out.chunks_exact_mut(BYTES_PER_PIXEL).enumerate() {
                let (start, weights) = self.columns.of(x);
                accumulate(sum, weights, &row[start * BYTES_PER_PIXEL..]);
            }
        }
        // Row at a time rather than pixel at a time, so each tap is one
        // multiply-add over a contiguous row the compiler can vectorise. The
        // order each output value adds its taps in is the same either way.
        for (y, out) in into.bytes_mut().chunks_exact_mut(width).enumerate() {
            let (start, weights) = self.rows.of(y);
            down.fill(0.0);
            for (k, &weight) in weights.iter().enumerate() {
                let row = &across[(start + k) * width..(start + k + 1) * width];
                for (total, &value) in down.iter_mut().zip(row) {
                    *total += weight * value;
                }
            }
            for (sum, pixel) in down
                .chunks_exact(BYTES_PER_PIXEL)
                .zip(out.chunks_exact_mut(BYTES_PER_PIXEL))
            {
                unpremultiply(sum, pixel);
            }
        }
    }
}

/// One source row with its colour weighted by alpha, in 0..=255 units.
fn premultiply(pixels: &[u8], into: &mut [f32]) {
    for (pixel, out) in pixels
        .chunks_exact(BYTES_PER_PIXEL)
        .zip(into.chunks_exact_mut(BYTES_PER_PIXEL))
    {
        let alpha = f32::from(pixel[3]);
        let coverage = alpha * (1.0 / 255.0);
        for channel in 0..3 {
            out[channel] = f32::from(pixel[channel]) * coverage;
        }
        out[3] = alpha;
    }
}

/// Adds a weighted run of premultiplied pixels into `sum`.
fn accumulate(sum: &mut [f32], weights: &[f32], pixels: &[f32]) {
    let mut total = [0.0_f32; BYTES_PER_PIXEL];
    for (&weight, pixel) in weights.iter().zip(pixels.chunks_exact(BYTES_PER_PIXEL)) {
        for channel in 0..BYTES_PER_PIXEL {
            total[channel] += weight * pixel[channel];
        }
    }
    sum.copy_from_slice(&total);
}

/// Back to straight alpha, clamped: a bicubic overshoots at a hard edge, and
/// a colour can only be as bright as white.
///
/// Rounded by adding a half and truncating, which is what `round` does for a
/// value already clamped non-negative — without the library call `round` is
/// on a baseline x86-64, three of them a pixel.
fn unpremultiply(sum: &[f32], into: &mut [u8]) {
    let alpha = sum[3].clamp(0.0, 255.0);
    let level = (alpha + 0.5) as u8;
    if level == 0 {
        into.fill(0);
        return;
    }
    let scale = 255.0 / alpha;
    for channel in 0..3 {
        into[channel] = ((sum[channel] * scale).clamp(0.0, 255.0) + 0.5) as u8;
    }
    into[3] = level;
}

/// The weights along one axis: for each output pixel, the first source pixel
/// it reads and how much of each it takes.
#[derive(Debug, Clone)]
struct Taps {
    spans: Vec<(usize, usize, usize)>,
    weights: Vec<f32>,
    /// The source pixels any output reads, as a range — rows outside it are
    /// skipped by the horizontal pass, which is what a `fill` crop saves.
    read: std::ops::Range<usize>,
}

impl Taps {
    /// `source` pixels scaled to `scaled`, of which `window` are kept from
    /// `offset` on. An output pixel past either end of the picture reads
    /// nothing, and so comes out transparent.
    fn new(source: u32, scaled: u32, window: u32, offset: i64) -> Self {
        let ratio = f64::from(source) / f64::from(scaled);
        let widen = ratio.max(1.0);
        let support = 2.0 * widen;
        let mut spans = Vec::with_capacity(window as usize);
        let mut weights = Vec::new();
        let (mut lowest, mut highest) = (usize::MAX, 0);
        for x in 0..i64::from(window) {
            let at = x + offset;
            if at < 0 || at >= i64::from(scaled) {
                spans.push((0, 0, 0));
                continue;
            }
            let centre = (at as f64 + 0.5) * ratio;
            let first = (centre - support + 0.5).floor().max(0.0) as usize;
            let last = ((centre + support + 0.5).floor() as usize).min(source as usize);
            let raw: Vec<f64> = (first..last)
                .map(|i| catmull_rom((i as f64 + 0.5 - centre) / widen))
                .collect();
            let total: f64 = raw.iter().sum();
            spans.push((first, weights.len(), raw.len()));
            weights.extend(raw.iter().map(|weight| (weight / total) as f32));
            (lowest, highest) = (lowest.min(first), highest.max(last));
        }
        Self {
            spans,
            weights,
            read: lowest..highest,
        }
    }

    /// The first source pixel output `x` reads, and its weights.
    fn of(&self, x: usize) -> (usize, &[f32]) {
        let (first, at, count) = self.spans[x];
        (first, &self.weights[at..at + count])
    }

    /// Whether any output pixel reads source pixel `i`.
    fn reads(&self, i: usize) -> bool {
        self.read.contains(&i)
    }
}

/// The Catmull-Rom cubic (`a = -0.5`): exact at whole pixels, so a picture
/// scaled by one comes out as it went in.
fn catmull_rom(x: f64) -> f64 {
    let x = x.abs();
    if x < 1.0 {
        (1.5 * x - 2.5) * x * x + 1.0
    } else if x < 2.0 {
        ((-0.5 * x + 2.5) * x - 4.0) * x + 2.0
    } else {
        0.0
    }
}
