//! The colour half of a tape: luma and two colour differences, and the smear.
//!
//! VHS recorded the picture's brightness at one bandwidth and its colour at a
//! small fraction of it. Everything characteristic about the colour of a tape
//! follows from that one fact — the smear sideways, the fringes that lag behind
//! an edge, and the green and magenta that a bad tape goes, because the errors
//! land on the **colour-difference axes** rather than on red, green and blue
//! independently. So this module's whole job is to get a pixel onto those axes,
//! do the damage there, and bring it back.
//!
//! **`Y`, `B − Y`, `R − Y`**, which is the plain colour-difference basis rather
//! than any broadcast scaling of it: no offsets, no headroom, no clipping to a
//! studio range. The scaling exists so a signal fits in a wire, and there is no
//! wire here — what matters is that the basis is linear, exactly invertible,
//! and separates *how bright* from *what colour*, which this one is and does.
//! `Y` uses the same Rec.709 weights [`crate::grade`] does, because a green
//! field and a blue one are not equally bright and an average of three channels
//! says they are.
//!
//! **Premultiplied throughout**, like everything else this stage touches: the
//! transform above it is linear, so a premultiplied pixel converts and comes
//! back exactly as a straight one would, and a transparent pixel contributes
//! nothing to the smear instead of contributing whatever colour it was stored
//! as.

/// Rec.709 luma weights — the same ones [`crate::grade`] uses, because there is
/// one answer in this crate to what the brightness of a pixel is.
const LUMA: (f64, f64, f64) = (0.2126, 0.7152, 0.0722);

/// The widest smear, as a fraction of the layer's width, at `chroma_bleed` of
/// `1.0`.
///
/// A twelfth of the picture is far past a tape and well into a fault, which is
/// where the top of a `0.0..=1.0` range belongs: `1.0` should be the heaviest
/// anybody would ask for rather than the heaviest the arithmetic can express.
/// Ordinary use is a quarter of it.
const WIDEST: f64 = 0.08;

/// One pixel on the axes this module works in, premultiplied.
///
/// `alpha` rides along because every value here is premultiplied by it: it is
/// what turns the luma back into the `0.0..=1.0` brightness a grain field wants
/// to be weighted by, and what the channels are clamped to on the way out.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Sample {
    /// Brightness, premultiplied, in `0.0..=255.0`.
    pub(crate) luma: f64,
    /// Blue minus luma, premultiplied.
    pub(crate) cb: f64,
    /// Red minus luma, premultiplied.
    pub(crate) cr: f64,
    /// The pixel's own coverage, `0.0..=255.0`.
    pub(crate) alpha: f64,
}

impl Sample {
    /// The pixel's brightness as the `0.0..=1.0` number a grain field weights
    /// itself by — which is a *straight* value, so the premultiplication has to
    /// come back out first. A pixel nothing covers is not dark, it is absent,
    /// and gets no noise at all.
    pub(crate) fn brightness(&self) -> f64 {
        if self.alpha <= 0.0 {
            return 0.0;
        }
        (self.luma / self.alpha).clamp(0.0, 1.0)
    }
}

/// One premultiplied RGBA pixel, split onto the axes.
pub(crate) fn split(pixel: &[u8]) -> Sample {
    let (r, g, b) = (
        f64::from(pixel[0]),
        f64::from(pixel[1]),
        f64::from(pixel[2]),
    );
    let luma = LUMA.0 * r + LUMA.1 * g + LUMA.2 * b;
    Sample {
        luma,
        cb: b - luma,
        cr: r - luma,
        alpha: f64::from(pixel[3]),
    }
}

/// The inverse, written into `out` as three bytes and then the alpha.
///
/// **Clamped to the alpha the pixel keeps.** A premultiplied channel above its
/// own alpha is not a colour, and tiny-skia is entitled to make nonsense of
/// one — the same clamp [`crate::aberration`] applies for the same reason, and
/// here it is the smear and the snow that can push a channel past it.
pub(crate) fn join(out: &mut Vec<u8>, luma: f64, cb: f64, cr: f64, alpha: f64) {
    // Only the correction is divided by green's weight, not the luma with it:
    // `Y = 0.2126(Y + cr) + 0.7152G + 0.0722(Y + cb)` rearranges to this, and
    // the version that divides the whole numerator is a picture that goes
    // green the moment the colour is taken out of it.
    let green = luma - (LUMA.0 * cr + LUMA.2 * cb) / LUMA.1;
    let channel = |value: f64| value.clamp(0.0, alpha).round() as u8;
    out.push(channel(luma + cr));
    out.push(channel(green));
    out.push(channel(luma + cb));
    out.push(alpha.round() as u8);
}

/// How many pixels of a row one chroma sample is smeared across, from the
/// fraction on the clip and the layer's width.
///
/// **One is no smear**, which is what a window of a single pixel means, so a
/// keyframe track ramping up from `0.0` does not run a moving average over a
/// whole raster to leave every pixel where it was. Rounded rather than
/// truncated, so the narrowest smear that means anything is the narrowest one
/// asked for; a NaN is named rather than left to the comparison, since it
/// compares false against everything and would otherwise fall through to the
/// cast.
pub(crate) fn window(bleed: f64, width: usize) -> usize {
    let pixels = bleed.clamp(0.0, 1.0) * WIDEST * width as f64;
    if pixels.is_nan() || pixels < 1.5 {
        return 1;
    }
    (pixels.round() as usize).min(width)
}

/// Smears one row's colour differences into `out`, leaving its luma alone.
///
/// **Trailing, not centred**, and that is the whole of what makes it read as a
/// tape rather than as a blur. The colour arrived *late*: each pixel's chroma
/// is the average of the window ending at it, so colour runs on past the right
/// side of an edge and the left side of one stays clean. A centred window would
/// fringe both sides equally, which is what a lens does — that effect already
/// exists next door and this is not it.
///
/// A running sum, so a wide smear costs exactly what a narrow one does, and the
/// window is clamped at the left edge the way [`crate::blur`]'s is: off the
/// raster reads the first pixel on it, rather than a colour nobody put there.
pub(crate) fn bleed(row: &[Sample], out: &mut [(f64, f64)], window: usize) {
    let first = row.first().copied().unwrap_or_default();
    let count = window as f64;
    // The window ending at pixel zero is entirely edge clamp, so it starts as
    // the first pixel repeated across the whole of it.
    let (mut cb, mut cr) = (first.cb * count, first.cr * count);
    for (x, out) in out.iter_mut().enumerate() {
        // Unconditionally, including at the first pixel — where the arriving
        // and the leaving sample are both the clamped first one, so the sum is
        // the window it started as. A guard there would be a branch nothing can
        // ever take a different answer from, which is a line no test can pin.
        let leaving = row[x.saturating_sub(window)];
        cb += row[x].cb - leaving.cb;
        cr += row[x].cr - leaving.cr;
        *out = (cb / count, cr / count);
    }
}

/// The threshold arithmetic, which nothing outside this file can reach.
///
/// Here rather than in `tests/taping/` for [`crate::aberration`]'s reason: that
/// is where the *effect* is asserted — colour runs rightward and not leftward,
/// the brightness survives — and an effect is satisfied by more than one
/// arithmetic. The number this hands back is the whole of what the clip's
/// `chroma_bleed` means, so the value to name is the number.
#[cfg(test)]
mod tests {
    use super::*;

    /// A twelfth of the width at `1.0`, and proportionally below it: `0.08`
    /// times the width, rounded. The factor is what decides how wide the
    /// heaviest smear anybody can ask for is, and nothing about the shape of
    /// the picture would say if it changed.
    #[test]
    fn the_window_is_a_fraction_of_the_width() {
        assert_eq!(window(1.0, 1000), 80);
        assert_eq!(window(0.5, 1000), 40);
        assert_eq!(window(1.0, 64), 5, "5.12 rounds down");
        assert_eq!(window(1.0, 80), 6, "6.4 rounds down");
        assert_eq!(window(1.0, 95), 8, "7.6 rounds up");
    }

    /// **Under a pixel and a half is no smear**, and a window of one *is* no
    /// smear — a mean over a single pixel is that pixel. So the smallest window
    /// this ever returns above one is two, and a keyframe track ramping up from
    /// zero runs no moving average until it would move something.
    #[test]
    fn nothing_worth_smearing_is_a_window_of_one() {
        assert_eq!(window(0.0, 1000), 1, "nothing is nothing");
        assert_eq!(window(-1.0, 1000), 1, "and so is a negative");
        assert_eq!(window(f64::NAN, 1000), 1, "and a non-number");
        // 0.018 × 0.08 × 1000 is 1.44, under the bar; 0.019 is 1.52, over it.
        assert_eq!(window(0.018, 1000), 1);
        assert_eq!(window(0.019, 1000), 2);
        // And the bar itself, exactly: a 25-wide raster puts `0.75 × 0.08 × 25`
        // at precisely `1.5` in binary, with nothing rounded on the way. A pixel
        // and a half **is** a smear — the narrowest one that does anything is
        // the narrowest one asked for, which is the same claim `blur::radius`
        // and `aberration::spread` each make at their own half pixel.
        assert_eq!(window(0.75, 25), 2);
    }

    /// Clamped at the top as well as the bottom, so a keyframe track that
    /// overshoots past `1.0` asks for the heaviest smear rather than one wider
    /// than the picture.
    #[test]
    fn past_the_top_of_the_range_is_the_top_of_the_range() {
        assert_eq!(window(4.0, 1000), window(1.0, 1000));
    }

    /// The window ending at the first pixel is entirely off the raster, so it
    /// reads that pixel repeated — which is what keeps the left edge of a
    /// picture the colour it already was instead of a colour nobody put there.
    #[test]
    fn the_left_edge_reads_itself_rather_than_nothing() {
        let row = [
            Sample {
                luma: 10.0,
                cb: 100.0,
                cr: -50.0,
                alpha: 255.0,
            },
            Sample::default(),
            Sample::default(),
            Sample::default(),
        ];
        let mut out = [(0.0, 0.0); 4];
        bleed(&row, &mut out, 4);
        assert_eq!(out[0], (100.0, -50.0), "the first pixel is its own mean");
        // And it leaves the window as it goes: a quarter of it by the second
        // pixel, none of it by the fifth, which there is not one of here.
        assert_eq!(out[1], (75.0, -37.5));
        assert_eq!(out[3], (25.0, -12.5));
    }
}
