//! the Chamberlin state-variable filter.
//!
//! The filter is what makes a subtractive patch expressive: a raw saw stack is a
//! buzz until an envelope sweeps a resonant cutoff across it. We use the
//! **Chamberlin SVF** — two integrators in a loop yielding lowpass, highpass and
//! bandpass from one topology, with an honest resonance knob — because it is stable
//! at audio rates, costs a handful of adds per sample, and takes a *new cutoff every
//! sample* for free (which is exactly what an envelope sweep and an LFO wobble need).
//! All four outputs the topology gives are exposed: the two it computes as state,
//! the high it computes on the way, and the notch, which is a single add of two of
//! them.
//!
//! A **slope** is that whole loop run twice in series. Both pairs take the same
//! tuning and the same damping, so 24 dB per octave is 12 applied twice — which is
//! also why resonance is a different control at the steeper setting: the emphasis
//! lands once per pair, so a peak is roughly squared and a filter that rang starts
//! to self-oscillate. That is the sound; it is also what a patch moved between the
//! two slopes has to be re-tuned for.
//!
//! The classic caveat is that the plain form goes unstable as the cutoff approaches
//! a quarter of the sample rate. We run it **2× oversampled** (two half-steps per
//! output sample), which pushes the stable ceiling up to ~`sr/3`, and clamp the
//! cutoff there — so a "fully open" filter really is open across the audible band.
//!
//! **That ceiling still holds for two pairs in series, and it was checked rather
//! than assumed.** Stability is a property of one integrator loop — the pole
//! positions of a pair depend on its own `f` and `q` and on nothing upstream of it
//! — so a second pair at the same coefficients is stable exactly where the first
//! is, and a cascade of stable stages is stable. What a cascade *does* change is
//! amplitude: a resonant peak passed through a second resonant peak comes out
//! roughly squared, which is loud rather than divergent, and the bake's limiter is
//! where loud is handled. `cascading_at_the_ceiling_stays_finite` pins both halves
//! of that at the top of the band with the resonance knob at its maximum.

use std::f32::consts::PI;

use crate::patch::{Filter, FilterKind};

/// Lowest cutoff a track is clamped to, in Hz — below hearing, and far from the
/// zero that would stall the integrators.
const MIN_CUTOFF: f32 = 20.0;
/// Highest cutoff as a fraction of the sample rate — the 2×-oversampled SVF's
/// stability ceiling.
const MAX_CUTOFF_RATIO: f32 = 1.0 / 3.0;

/// The most pole pairs any slope runs in series — [`Slope::Db24`]'s two.
///
/// [`Slope::Db24`]: crate::patch::Slope::Db24
const MAX_PAIRS: usize = 2;

/// Two integrator states — one pole pair's whole memory.
#[derive(Clone, Copy, Default)]
struct Svf {
    low: f32,
    band: f32,
}

impl Svf {
    /// One half-step of the SVF at the given tuning `f` and damping `q`, returning
    /// the requested output. Non-finite state (a numerical blow-up from an extreme
    /// setting) resets to silence rather than poisoning the rest of the buffer.
    fn step(&mut self, input: f32, f: f32, q: f32, kind: FilterKind) -> f32 {
        let high = input - self.low - q * self.band;
        self.band += f * high;
        self.low += f * self.band;
        if !self.low.is_finite() || !self.band.is_finite() {
            self.low = 0.0;
            self.band = 0.0;
            return 0.0;
        }
        match kind {
            FilterKind::Lowpass => self.low,
            FilterKind::Highpass => high,
            // Already computed as state and, until now, thrown away every sample.
            FilterKind::Bandpass => self.band,
            // And the notch is one add of the two ends the pair already has.
            FilterKind::Notch => high + self.low,
        }
    }
}

/// One half-step through every pole pair in series, each taking the one before it.
///
/// Each pair reads the same `kind`, so two lowpasses make a steeper lowpass and two
/// notches a deeper one — the slope is the shape applied twice, never two different
/// shapes stacked.
fn cascade(pairs: &mut [Svf], input: f32, f: f32, q: f32, kind: FilterKind) -> f32 {
    pairs
        .iter_mut()
        .fold(input, |signal, pair| pair.step(signal, f, q, kind))
}

/// Filter `buf` in place, taking the cutoff for sample `i` from `cutoffs[i]` (the
/// caller bakes the envelope and LFO into that track). `sample_rate` is the output
/// rate; the filter itself runs at twice it.
pub(crate) fn apply(buf: &mut [f32], filter: &Filter, cutoffs: &[f32], sample_rate: f32) {
    let mut svf = [Svf::default(); MAX_PAIRS];
    let pairs = &mut svf[..filter.slope.pole_pairs()];
    let q = (1.0 - filter.resonance.clamp(0.0, 0.99)).clamp(0.05, 1.0);
    let ceiling = sample_rate * MAX_CUTOFF_RATIO;
    for (i, s) in buf.iter_mut().enumerate() {
        let cutoff = cutoffs.get(i).copied().unwrap_or(filter.cutoff);
        let f = tuning(cutoff.clamp(MIN_CUTOFF, ceiling), sample_rate);
        // Two half-rate steps per output sample (the 2× oversampling); the second
        // one's output is the sample we keep.
        cascade(pairs, *s, f, q, filter.kind);
        *s = cascade(pairs, *s, f, q, filter.kind);
    }
}

/// The SVF tuning coefficient for a cutoff, at 2× oversampling: `2·sin(π·fc/2·sr)`.
#[inline]
fn tuning(cutoff: f32, sample_rate: f32) -> f32 {
    (2.0 * (PI * cutoff / (2.0 * sample_rate)).sin()).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{Adsr, Slope};
    use std::f32::consts::TAU;

    fn filter(kind: FilterKind, cutoff: f32) -> Filter {
        Filter {
            kind,
            cutoff,
            slope: Slope::Db12,
            resonance: 0.0,
            env_octaves: 0.0,
            vel_octaves: 0.0,
            adsr: Adsr::default(),
        }
    }

    /// A sine at `hz`, one second long at 44.1 kHz.
    fn sine(hz: f32) -> Vec<f32> {
        (0..44_100)
            .map(|i| (TAU * hz * i as f32 / 44_100.0).sin())
            .collect()
    }

    /// Peak amplitude over the settled second half of a buffer.
    fn settled_peak(buf: &[f32]) -> f32 {
        buf[buf.len() / 2..]
            .iter()
            .fold(0.0f32, |m, s| m.max(s.abs()))
    }

    #[test]
    fn lowpass_passes_below_and_stops_above_the_cutoff() {
        let cutoffs = vec![1000.0; 44_100];
        let mut low = sine(100.0);
        let mut high = sine(8000.0);
        apply(
            &mut low,
            &filter(FilterKind::Lowpass, 1000.0),
            &cutoffs,
            44_100.0,
        );
        apply(
            &mut high,
            &filter(FilterKind::Lowpass, 1000.0),
            &cutoffs,
            44_100.0,
        );
        assert!(settled_peak(&low) > 0.7, "100 Hz survives a 1 kHz lowpass");
        assert!(settled_peak(&high) < 0.1, "8 kHz does not");
    }

    #[test]
    fn highpass_is_the_mirror_image() {
        let cutoffs = vec![2000.0; 44_100];
        let mut low = sine(100.0);
        let mut high = sine(8000.0);
        apply(
            &mut low,
            &filter(FilterKind::Highpass, 2000.0),
            &cutoffs,
            44_100.0,
        );
        apply(
            &mut high,
            &filter(FilterKind::Highpass, 2000.0),
            &cutoffs,
            44_100.0,
        );
        assert!(settled_peak(&low) < 0.1, "100 Hz is cut");
        assert!(settled_peak(&high) > 0.7, "8 kHz passes");
    }

    #[test]
    fn bandpass_keeps_the_middle_and_drops_both_ends() {
        let cutoffs = vec![1000.0; 44_100];
        let band = filter(FilterKind::Bandpass, 1000.0);
        let mut low = sine(60.0);
        let mut mid = sine(1000.0);
        let mut high = sine(9000.0);
        for buf in [&mut low, &mut mid, &mut high] {
            apply(buf, &band, &cutoffs, 44_100.0);
        }
        let centre = settled_peak(&mid);
        assert!(centre > settled_peak(&low) * 4.0, "60 Hz is off the bottom");
        assert!(centre > settled_peak(&high) * 4.0, "9 kHz is off the top");
    }

    /// A notch is the mirror: the ends survive and the middle is what goes. It is
    /// tested against the bandpass rather than on its own because the two are one
    /// sum apart, and a sign error would pass a test that only looked at one.
    #[test]
    fn notch_is_the_bandpass_turned_inside_out() {
        let cutoffs = vec![1000.0; 44_100];
        let notch = filter(FilterKind::Notch, 1000.0);
        let mut low = sine(60.0);
        let mut mid = sine(1000.0);
        for buf in [&mut low, &mut mid] {
            apply(buf, &notch, &cutoffs, 44_100.0);
        }
        assert!(settled_peak(&low) > 0.7, "60 Hz passes a 1 kHz notch");
        assert!(
            settled_peak(&mid) < settled_peak(&low) * 0.5,
            "1 kHz does not"
        );
    }

    /// The whole point of the steeper slope: the same cutoff rejects harder.
    #[test]
    fn two_pole_pairs_reject_further_past_the_cutoff_than_one() {
        let cutoffs = vec![1000.0; 44_100];
        let mut gentle = sine(4000.0);
        let mut steep = sine(4000.0);
        apply(
            &mut gentle,
            &filter(FilterKind::Lowpass, 1000.0),
            &cutoffs,
            44_100.0,
        );
        apply(
            &mut steep,
            &Filter {
                slope: Slope::Db24,
                ..filter(FilterKind::Lowpass, 1000.0)
            },
            &cutoffs,
            44_100.0,
        );
        assert!(
            settled_peak(&steep) < settled_peak(&gentle) * 0.5,
            "24 dB rejects harder than 12: {} vs {}",
            settled_peak(&gentle),
            settled_peak(&steep)
        );
    }

    /// The interaction a reader has to be warned about: at 24 dB the emphasis is
    /// applied once per pair, so the same `resonance` rings considerably harder.
    #[test]
    fn resonance_is_a_stronger_control_at_the_steeper_slope() {
        let cutoffs = vec![1000.0; 44_100];
        let mut gentle = sine(1000.0);
        let mut steep = sine(1000.0);
        let mut f = filter(FilterKind::Lowpass, 1000.0);
        f.resonance = 0.9;
        apply(&mut gentle, &f, &cutoffs, 44_100.0);
        f.slope = Slope::Db24;
        apply(&mut steep, &f, &cutoffs, 44_100.0);
        assert!(
            settled_peak(&steep) > settled_peak(&gentle),
            "the peak is applied twice"
        );
    }

    /// The stability claim the module doc makes for the cascade, pinned: two pairs
    /// at the top of the band with the resonance knob at its maximum stay finite,
    /// which is the half that matters, and stay bounded, which is the half a
    /// cascade of resonant peaks could plausibly have lost.
    #[test]
    fn cascading_at_the_ceiling_stays_finite() {
        for cutoff in [20.0, 14_700.0, 1e9] {
            let mut buf = sine(440.0);
            let cutoffs = vec![cutoff; buf.len()];
            let mut f = filter(FilterKind::Lowpass, cutoff);
            f.slope = Slope::Db24;
            f.resonance = 0.99;
            apply(&mut buf, &f, &cutoffs, 44_100.0);
            assert!(
                buf.iter().all(|s| s.is_finite()),
                "cutoff {cutoff} produced non-finite samples at 24 dB"
            );
            assert!(
                buf.iter().all(|s| s.abs() < 100.0),
                "cutoff {cutoff} rang away rather than settling"
            );
        }
    }

    #[test]
    fn resonance_lifts_the_response_at_the_cutoff() {
        let cutoffs = vec![1000.0; 44_100];
        let mut flat = sine(1000.0);
        let mut resonant = sine(1000.0);
        let mut f = filter(FilterKind::Lowpass, 1000.0);
        apply(&mut flat, &f, &cutoffs, 44_100.0);
        f.resonance = 0.9;
        apply(&mut resonant, &f, &cutoffs, 44_100.0);
        assert!(settled_peak(&resonant) > settled_peak(&flat));
    }

    #[test]
    fn extreme_cutoffs_stay_finite_and_bounded() {
        for cutoff in [0.0, -500.0, 1e9, f32::INFINITY] {
            let mut buf = sine(440.0);
            let cutoffs = vec![cutoff; buf.len()];
            let mut f = filter(FilterKind::Lowpass, 1000.0);
            f.resonance = 0.99;
            apply(&mut buf, &f, &cutoffs, 44_100.0);
            assert!(
                buf.iter().all(|s| s.is_finite()),
                "cutoff {cutoff} produced non-finite samples"
            );
        }
    }

    #[test]
    fn a_poisoned_sample_resets_the_filter_instead_of_spreading() {
        // One non-finite input would otherwise infect the integrators forever, so
        // every later sample would come out NaN. The guard flushes the state.
        let mut buf = sine(440.0);
        buf[100] = f32::INFINITY;
        buf[200] = f32::NAN;
        let cutoffs = vec![1000.0; buf.len()];
        apply(
            &mut buf,
            &filter(FilterKind::Lowpass, 1000.0),
            &cutoffs,
            44_100.0,
        );
        assert!(
            buf.iter().all(|s| s.is_finite()),
            "the poison was contained"
        );
        // And the filter is still a filter afterwards, not a stuck zero.
        assert!(
            settled_peak(&buf) > 0.5,
            "it recovered and kept passing signal"
        );
    }
}
