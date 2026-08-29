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

/// The displacement arithmetic, named where it is decided. `tests/taping/`
/// asserts what a wobbling picture *looks* like — whole pixels, varying down
/// the frame, moving with it — and every one of those claims is satisfied by
/// more than one set of numbers. These are the numbers.
#[cfg(test)]
mod tests {
    use super::*;

    fn tracking(jitter: f64, head_switch: f64) -> Tracking {
        Tracking::new(1234, jitter, head_switch, 1000, 800)
    }

    /// A twentieth of the **width** at `1.0`, and clamped at both ends of the
    /// range: a negative wobble is not a wobble the other way, there being no
    /// such thing to an author holding one number.
    #[test]
    fn the_amplitude_is_a_fraction_of_the_width() {
        let near = |got: f64, want: f64| assert!((got - want).abs() < 1e-9, "{got} against {want}");
        near(tracking(1.0, 0.0).amplitude, 50.0);
        near(tracking(0.2, 0.0).amplitude, 10.0);
        near(tracking(0.0, 0.0).amplitude, 0.0);
        near(tracking(-1.0, 0.0).amplitude, 0.0);
        near(tracking(4.0, 0.0).amplitude, 50.0);
    }

    /// Eight waves down the frame however tall it is, and never a wave of no
    /// rows — which would divide by zero on the very smallest layer.
    #[test]
    fn there_are_eight_waves_down_the_frame() {
        assert_eq!(tracking(1.0, 0.0).wave, 100);
        assert_eq!(Tracking::new(1, 1.0, 0.0, 10, 4).wave, 1);
    }

    /// A third of the width at the bottom row at `1.0`, and clamped like the
    /// wobble.
    #[test]
    fn the_tear_is_a_larger_fraction_of_the_same_width() {
        let near = |got: f64, want: f64| assert!((got - want).abs() < 1e-9, "{got} against {want}");
        near(tracking(0.0, 1.0).tear, 300.0);
        near(tracking(0.0, 0.5).tear, 150.0);
        near(tracking(0.0, -1.0).tear, 0.0);
    }

    /// The wave's own value at every boundary, and something strictly between
    /// its ends in between — which is what says the two are interpolated rather
    /// than the row's own hash being read directly, and that the interpolation
    /// is not a step.
    #[test]
    fn the_wobble_lands_on_its_hashed_value_at_each_boundary() {
        let tracking = tracking(1.0, 0.0);
        for wave in 0..3u64 {
            let want = grain::value(tracking.wobble, wave);
            let got = tracking.wobble(wave as usize * tracking.wave);
            assert!((got - want).abs() < f64::EPSILON, "wave {wave}");
        }
        let (from, to) = (
            grain::value(tracking.wobble, 0),
            grain::value(tracking.wobble, 1),
        );
        let middle = tracking.wobble(tracking.wave / 2);
        assert!(
            (middle - (from + to) / 2.0).abs() < 1e-9,
            "the midpoint of a smoothstep is the midpoint of its ends"
        );
        assert_ne!(middle, from);
        assert_ne!(middle, to);
    }

    /// Every row outside the band moves by the wobble alone; the tear is added
    /// to it and never in place of it, so a taped picture does not stop
    /// wobbling where it starts tearing.
    #[test]
    fn the_tear_is_added_to_the_wobble_rather_than_replacing_it() {
        let tracking = tracking(1.0, 1.0);
        let wobble = tracking.shift(700, None);
        let torn = tracking.shift(700, Some(1.0));
        assert_eq!(
            torn - wobble,
            (tracking.tear * tracking.ragged(700)).round() as isize
        );
        assert!(torn.abs() > wobble.abs(), "and it is much the larger");
    }

    /// The wobble stays inside `-1.0..=1.0`, so the amplitude above is the
    /// whole of what bounds a displacement.
    #[test]
    fn the_wobble_never_leaves_its_range() {
        let tracking = tracking(1.0, 0.0);
        for y in 0..800 {
            let wobble = tracking.wobble(y);
            assert!((-1.0..=1.0).contains(&wobble), "row {y}: {wobble}");
        }
    }
}
