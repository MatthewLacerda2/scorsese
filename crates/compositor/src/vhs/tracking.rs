//! How far sideways each row of the picture ended up.
//!
//! Two artefacts share one number, because they are one operation: a row read
//! at the wrong moment lands at the wrong horizontal position. The **tracking
//! wobble** is that happening a little, everywhere, all the time; **head
//! switching** is it happening a lot, in a band of rows at the bottom, because
//! the tape's two heads hand the picture over there.
//!
//! **Whole pixels, and that is deliberate.** A line read late is a line moved
//! over, not a line resampled — so there is no interpolation here, no softening,
//! and no fraction of a pixel to disagree about. It also makes what this
//! produces *structure* rather than texture, which is the difference between an
//! artefact an encoder keeps and one it deletes; `docs/golden-renders.md` has
//! why that matters to the pixel gate.
//!
//! **Pure functions of the seed and the row**, which for the seed's part means
//! pure functions of the clip and the frame — [`crate::grain::seed`] is where
//! both are folded in, and it is the same mechanism grain uses to move without
//! carrying anything between frames. A wobble drawn from a generator would
//! differ between two renders of the same project, which does not fail the
//! pixel gate loudly, it makes it meaningless.

use crate::grain;

/// Which noise field is which. Two, so the wobble and the tear are independent
/// rather than the same numbers read twice.
const WOBBLE: u64 = 3;
const TEAR: u64 = 4;

/// The widest the wobble goes at `jitter` of `1.0`, as a fraction of the
/// layer's width.
///
/// **Of the width and not the height**, which is the one measurement in this
/// crate that is: a row is displaced *along* itself, and how far it can go is
/// bounded by how long it is rather than by how tall the picture is.
const WIDEST: f64 = 0.05;

/// How many wobbles fit down one frame.
///
/// A count rather than a pitch in pixels, so the wave has the same shape at any
/// delivery size. Eight is a wave long enough to read as the picture *leaning*
/// rather than as every row going its own way — which would be noise, and noise
/// is what the knob next door does.
const WAVES: usize = 8;

/// How far the tear at the very bottom of the head-switching band goes, as a
/// fraction of the layer's width, at `head_switch` of `1.0`.
const TORN: f64 = 0.3;

/// How much the tear varies from row to row, as a fraction of itself. A clean
/// displacement reads as the picture being cut and pasted; the ragged edge is
/// what says the signal fell apart there.
const RAGGED: f64 = 0.35;

/// Where every row of one layer's picture ended up, for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Tracking {
    /// The wobble's own noise field, and the tear's. Separate fields of the
    /// same frame's seed — see [`grain::field`].
    wobble: u64,
    tear_field: u64,
    /// The wobble's peak displacement, in pixels.
    amplitude: f64,
    /// How many rows one wobble spans. Never zero.
    wave: usize,
    /// The tear on the band's last row, in pixels, before the raggedness.
    tear: f64,
}

impl Tracking {
    pub(crate) fn new(seed: u64, jitter: f64, head_switch: f64, width: u32, height: u32) -> Self {
        let width = f64::from(width);
        Self {
            wobble: grain::field(seed, WOBBLE),
            tear_field: grain::field(seed, TEAR),
            amplitude: jitter.clamp(0.0, 1.0) * WIDEST * width,
            wave: (height as usize / WAVES).max(1),
            tear: head_switch.clamp(0.0, 1.0) * TORN * width,
        }
    }

    /// How far right this row is displaced, in whole pixels. Negative is left.
    ///
    /// `switching` is how deep into the head-switching band the row sits, from
    /// [`super::lines::Band::depth`], and `None` for the rest of the picture.
    pub(crate) fn shift(&self, y: usize, switching: Option<f64>) -> isize {
        let tear = switching.map_or(0.0, |depth| self.tear * depth * self.ragged(y));
        (self.wobble(y) * self.amplitude + tear).round() as isize
    }

    /// The wobble at this row, in `-1.0..=1.0`.
    ///
    /// **Smooth down the frame and independent between frames**, which is the
    /// shape a tracking error actually has: the picture leans, the lean varies
    /// gradually from top to bottom, and how it leans is a fresh accident every
    /// frame rather than a drift with a memory. One hashed value per wave,
    /// interpolated between with a smoothstep — a straight line between them
    /// would put a visible crease across the picture at every boundary, because
    /// the *slope* is what the eye reads in a displacement field.
    fn wobble(&self, y: usize) -> f64 {
        let wave = (y / self.wave) as u64;
        let t = (y % self.wave) as f64 / self.wave as f64;
        let t = t * t * (3.0 - 2.0 * t);
        let (from, to) = (
            grain::value(self.wobble, wave),
            grain::value(self.wobble, wave + 1),
        );
        from + (to - from) * t
    }

    /// The factor this row's tear is scaled by: one, give or take.
    fn ragged(&self, y: usize) -> f64 {
        1.0 + RAGGED * grain::value(self.tear_field, y as u64)
    }
}
