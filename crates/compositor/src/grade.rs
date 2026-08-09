//! What a [`Grade`] does to pixels.
//!
//! `scorsese-core` says what the five numbers *are* — their neutrals, and which
//! way each one runs. This is where they acquire arithmetic, in one place, next
//! to the loop that runs it. Same split as [`crate::properties`]: the format
//! holds the shape, the crate that implements it holds the meaning.
//!
//! **Straight RGBA in, straight RGBA out, alpha untouched.** A grade changes
//! what colour a pixel is and never how much of it there is — including the
//! vignette, which darkens toward the corners rather than fading out, because a
//! vignette that ate the alpha would let the track below show through the
//! corners of the one above it.
//!
//! No linearisation. The arithmetic runs on the 8-bit values as they arrive,
//! normalised to `0.0..=1.0`. Working in linear light would be more correct in
//! the sense a colour scientist means and is exactly the pipeline
//! `CLAUDE.md` rules out — these are knobs somebody turns until the picture
//! looks right, and the picture they are looking at is the one this produces.

use scorsese_core::Grade;

use crate::frame::{BYTES_PER_PIXEL, Frame, Resolution};

/// Rec.709 luma weights, which is what "the grey of this pixel" means here.
/// Green carries most of the perceived brightness, so an average of the three
/// channels would desaturate a green field to something much darker than it
/// looks.
const LUMA: (f64, f64, f64) = (0.2126, 0.7152, 0.0722);

/// How far a temperature of `1.0` pushes the warm and cool channels.
///
/// A gain rather than an offset, so warming a shot leaves its blacks black
/// instead of tinting them — an offset on red is a red fog over the shadows,
/// which is the first thing anybody notices and hates.
const TEMPERATURE_GAIN: f64 = 0.25;

/// Writes `source` into `out`, graded.
///
/// `out` is resized to fit and is the caller's scratch buffer, reused across
/// frames — a render grading a 1080p layer allocates this once rather than
/// eight megabytes thirty times a second.
pub(crate) fn into(out: &mut Vec<u8>, source: &Frame, grade: Grade) {
    let resolution = source.resolution();
    let bytes = source.bytes();
    out.clear();
    out.reserve(bytes.len());

    // Read once per layer rather than per pixel, so the inner loop is
    // arithmetic and nothing else.
    let saturation = grade.saturation.max(0.0);
    let contrast = grade.contrast.max(0.0);
    let (warm, cool) = (
        1.0 + grade.temperature * TEMPERATURE_GAIN,
        1.0 - grade.temperature * TEMPERATURE_GAIN,
    );
    let vignette = grade.vignette.clamp(0.0, 1.0);

    for (index, pixel) in bytes.chunks_exact(BYTES_PER_PIXEL).enumerate() {
        let (mut r, mut g, mut b) = (
            f64::from(pixel[0]) / 255.0,
            f64::from(pixel[1]) / 255.0,
            f64::from(pixel[2]) / 255.0,
        );

        // Saturation first, about the pixel's own luma: a shot is desaturated
        // and *then* warmed, which is the order the two knobs read in — warming
        // a picture and then taking the colour out again would undo the warmth,
        // and nobody turning the second knob means to undo the first.
        let grey = LUMA.0 * r + LUMA.1 * g + LUMA.2 * b;
        r = grey + (r - grey) * saturation;
        g = grey + (g - grey) * saturation;
        b = grey + (b - grey) * saturation;

        r *= warm;
        b *= cool;

        // Contrast about mid-grey, then brightness as an offset. Contrast is a
        // pivot and brightness a shift, so doing them the other way round would
        // make the contrast knob move the picture's exposure as a side effect.
        r = (r - 0.5) * contrast + 0.5 + grade.brightness;
        g = (g - 0.5) * contrast + 0.5 + grade.brightness;
        b = (b - 0.5) * contrast + 0.5 + grade.brightness;

        if vignette > 0.0 {
            let fall = 1.0 - vignette * corner_distance(index, resolution);
            r *= fall;
            g *= fall;
            b *= fall;
        }

        out.push(quantise(r));
        out.push(quantise(g));
        out.push(quantise(b));
        out.push(pixel[3]);
    }
}

/// How far this pixel is from the layer's centre, as a fraction of the distance
/// to a corner: `0.0` dead centre, `1.0` at a corner.
///
/// Squared on the way out, which is what makes the falloff read as a vignette
/// rather than a grey wash — a linear ramp darkens the middle of the frame
/// visibly, and the middle of the frame is the part a vignette exists to leave
/// alone. Measured against the layer's own rectangle, so a vignette on a
/// picture-in-picture darkens *its* corners.
fn corner_distance(index: usize, resolution: Resolution) -> f64 {
    let width = f64::from(resolution.width());
    let height = f64::from(resolution.height());
    #[expect(
        clippy::cast_precision_loss,
        reason = "a pixel index within a raster; f64 is exact to 2^53, which is \
                  every frame anything will ever composite"
    )]
    let (x, y) = {
        let index = index as f64;
        (index % width, (index / width).floor())
    };
    // Centres of the outermost pixels reach exactly ±1, so a 1×1 layer is all
    // centre rather than all corner.
    let dx = (2.0 * x + 1.0) / width - 1.0;
    let dy = (2.0 * y + 1.0) / height - 1.0;
    ((dx * dx + dy * dy) / 2.0).min(1.0)
}

/// One channel back to a byte, with everything the arithmetic pushed out of
/// range brought back to the edge of it.
///
/// Clamping rather than wrapping, and it is the only sane answer: a brightness
/// that overshoots white is a blown highlight, and one that wrapped would put a
/// black speckle in the middle of it.
fn quantise(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
