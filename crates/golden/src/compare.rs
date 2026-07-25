//! Measuring how far one frame is from another.
//!
//! Two numbers, because neither alone is enough:
//!
//! - **SSIM**, on the worst 8×8 block in the frame, catches *where* pixels are —
//!   a shape half a pixel out, an edge gone soft, a wrong patch in one corner.
//! - **Mean absolute error**, per colour channel, catches *what* pixels are — a
//!   wrong colour, a whole frame a few levels bright, a cut one frame early.
//!
//! Each of those is a failure the other waves through, and `tests/comparing.rs`
//! holds that claim to specific cases.

use std::ops::Range;

use serde::Deserialize;

use scorsese_render::{Frame, Resolution};

/// Channels compared for error: red, green, blue.
///
/// Alpha is skipped because a rendered frame is opaque everywhere, so including
/// it would only dilute the average with a constant.
const CHANNELS: usize = 3;
const BYTES_PER_PIXEL: usize = 4;

/// SSIM window. Eight is the conventional block size, and small enough that a
/// wrong corner cannot average itself away inside an otherwise-correct block.
const BLOCK: usize = 8;

/// SSIM's luminance stabiliser, `(K1 · 255)²` with **K1 = 0.1**.
///
/// The paper's K1 is 0.01, and the paper is about photographs. Its luminance
/// term is scale-relative, which on near-black flat content is savage: with the
/// standard constant a region moving from level 0 to level 1 scores 0.87. Our
/// fixtures are full of flat black — every gap in a timeline is a black frame —
/// so the standard value would fire on one level of encoder noise, and a gate
/// that cries wolf gets re-blessed reflexively until it means nothing.
///
/// K1 = 0.1 tolerates a couple of levels on flat content while leaving real
/// differences nowhere to hide: black against red still scores 0.10.
const C1: f64 = 650.25;

/// SSIM's contrast stabiliser, `(0.03 · 255)²` — the standard value, which
/// needs no adjusting because contrast is compared as a ratio of spreads.
const C2: f64 = 58.5225;

/// Rec. 601 luma weights, which is how the encoder's own 4:2:0 sees brightness.
const LUMA: [f64; CHANNELS] = [0.299, 0.587, 0.114];

/// How far apart two frames are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Difference {
    /// Structural similarity of the **worst** 8×8 block in the frame, on luma.
    /// 1.0 is identical.
    ///
    /// The worst block rather than the average: a wrong 4×4 patch in a 64×64
    /// frame moves the average by less than two parts in a thousand and would
    /// sail through, while the block containing it collapses.
    pub ssim: f64,
    /// Mean absolute channel difference over the whole frame, on the 0–255
    /// scale.
    pub mean_error: f64,
}

/// How much difference is acceptable.
///
/// Loose enough to survive a different platform's x264 — what we control is
/// frames, not bitstreams — and far tighter than any real regression. A wrong
/// colour, a cut one frame out, or a lost letterbox scores nowhere near these.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tolerance {
    #[serde(default = "default_min_ssim")]
    pub min_ssim: f64,
    #[serde(default = "default_max_mean_error")]
    pub max_mean_error: f64,
}

fn default_min_ssim() -> f64 {
    0.95
}

fn default_max_mean_error() -> f64 {
    2.0
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            min_ssim: default_min_ssim(),
            max_mean_error: default_max_mean_error(),
        }
    }
}

impl Tolerance {
    pub fn accepts(&self, difference: &Difference) -> bool {
        difference.ssim >= self.min_ssim && difference.mean_error <= self.max_mean_error
    }
}

/// Compares two frames of the same size.
pub fn compare(actual: &Frame, expected: &Frame) -> Result<Difference, SizeMismatch> {
    if actual.resolution() != expected.resolution() {
        return Err(SizeMismatch {
            actual: actual.resolution(),
            expected: expected.resolution(),
        });
    }
    let resolution = actual.resolution();
    let a = luma(actual.bytes());
    let b = luma(expected.bytes());
    Ok(Difference {
        ssim: worst_block_ssim(&a, &b, resolution),
        mean_error: mean_error(actual.bytes(), expected.bytes()),
    })
}

/// Two frames that are not even the same shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("frame is {actual} but its reference is {expected}")]
pub struct SizeMismatch {
    pub actual: Resolution,
    pub expected: Resolution,
}

/// One brightness value per pixel.
fn luma(pixels: &[u8]) -> Vec<f64> {
    pixels
        .chunks_exact(BYTES_PER_PIXEL)
        .map(|pixel| {
            (0..CHANNELS)
                .map(|channel| LUMA[channel] * f64::from(pixel[channel]))
                .sum()
        })
        .collect()
}

/// The lowest SSIM of any block in the frame.
fn worst_block_ssim(a: &[f64], b: &[f64], resolution: Resolution) -> f64 {
    let width = resolution.width() as usize;
    let height = resolution.height() as usize;
    let mut worst = f64::INFINITY;
    for top in (0..height).step_by(BLOCK) {
        for left in (0..width).step_by(BLOCK) {
            let rows = top..(top + BLOCK).min(height);
            let columns = left..(left + BLOCK).min(width);
            worst = worst.min(block_ssim(a, b, width, columns, rows));
        }
    }
    if worst.is_finite() { worst } else { 1.0 }
}

/// SSIM of one block: the luminance term times the contrast-and-structure term.
fn block_ssim(
    a: &[f64],
    b: &[f64],
    width: usize,
    columns: Range<usize>,
    rows: Range<usize>,
) -> f64 {
    let mut count = 0.0;
    let (mut sum_a, mut sum_b) = (0.0, 0.0);
    let (mut sum_aa, mut sum_bb, mut sum_ab) = (0.0, 0.0, 0.0);
    for row in rows {
        for column in columns.clone() {
            let at = row * width + column;
            let (va, vb) = (a[at], b[at]);
            count += 1.0;
            sum_a += va;
            sum_b += vb;
            sum_aa += va * va;
            sum_bb += vb * vb;
            sum_ab += va * vb;
        }
    }
    let mean_a = sum_a / count;
    let mean_b = sum_b / count;
    // Unbiased, as SSIM is conventionally defined. A single-pixel block has no
    // spread to estimate, so it divides by one and reports none.
    let divisor = if count > 1.0 { count - 1.0 } else { 1.0 };
    let variance_a = (sum_aa - count * mean_a * mean_a) / divisor;
    let variance_b = (sum_bb - count * mean_b * mean_b) / divisor;
    let covariance = (sum_ab - count * mean_a * mean_b) / divisor;

    let luminance = (2.0 * mean_a * mean_b + C1) / (mean_a * mean_a + mean_b * mean_b + C1);
    let structure = (2.0 * covariance + C2) / (variance_a + variance_b + C2);
    luminance * structure
}

fn mean_error(a: &[u8], b: &[u8]) -> f64 {
    let mut total: u64 = 0;
    let mut count: u64 = 0;
    for (pixel_a, pixel_b) in a
        .chunks_exact(BYTES_PER_PIXEL)
        .zip(b.chunks_exact(BYTES_PER_PIXEL))
    {
        for channel in 0..CHANNELS {
            total += u64::from(pixel_a[channel].abs_diff(pixel_b[channel]));
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}
