//! the peak limiter every bake passes through.
//!
//! Not an effect a recipe chooses: a bake **must never clip**. Detuned oscillators
//! drift in and out of phase, a resonant filter can double a peak, and summed notes
//! add up — any of which can push a buffer past ±1, where 16-bit quantization turns
//! the overshoot into a click. So the last thing a bake does is limit.
//!
//! Because we render offline we can do this properly, with **lookahead**: compute
//! the gain each sample needs, then walk the track backwards so the gain is already
//! down *before* a peak arrives (a short attack ramp instead of an instant duck),
//! and forwards so it recovers no faster than the release. Both passes only ever
//! lower the gain, so the ceiling is guaranteed, and a signal that never approaches
//! it is left untouched.
//!
//! **The two channels are linked**, which is the one thing a stereo limiter has
//! to get right. There is a single gain track, computed from whichever side is
//! louder at each sample and applied to both. Limiting each channel on its own
//! would be a fader that moved one side and not the other every time a peak
//! landed off-centre — a centred vocal would drift left whenever the hats on
//! the right got loud, and the whole image would breathe sideways. Linking
//! costs a `max`; not linking costs the stereo image, and a peak on one side
//! genuinely *is* a moment when the mix is too loud.

use crate::stereo::Stereo;

/// The peak the output is held under. A hair below full scale, so quantization
/// rounding cannot reach ±1.0.
const CEILING: f32 = 0.98;
/// Lookahead / attack ramp, in seconds — long enough to duck without a click.
const ATTACK: f32 = 0.002;
/// Recovery, in seconds. Slower than the attack, or the gain pumps audibly.
const RELEASE: f32 = 0.06;

/// Limit `buf` in place so no sample exceeds the ceiling. Non-finite samples (from
/// a pathological patch) are flushed to silence rather than written to the file.
pub(crate) fn apply(buf: &mut Stereo, rate: f32) {
    let mut gain = required_gain(buf);
    ramp_down_before_peaks(&mut gain, slope(ATTACK, rate));
    ramp_up_after_peaks(&mut gain, slope(RELEASE, rate));
    buf.each(|channel| {
        for (s, g) in channel.iter_mut().zip(&gain) {
            *s = (*s * g).clamp(-1.0, 1.0);
        }
    });
}

/// The instantaneous gain each sample-frame needs to sit under the ceiling —
/// 1.0 wherever the signal is already quiet enough. Sanitizes `buf` on the way
/// past.
///
/// One number per frame, from the louder of the two channels: that is what
/// links them.
fn required_gain(buf: &mut Stereo) -> Vec<f32> {
    buf.l
        .iter_mut()
        .zip(buf.r.iter_mut())
        .map(|(l, r)| {
            let peak = sanitised(l).max(sanitised(r));
            if peak > CEILING { CEILING / peak } else { 1.0 }
        })
        .collect()
}

/// How loud one sample is, flushing a non-finite one to silence on the way —
/// a `NaN` that reached the file would be a click, and a `NaN` left in the
/// gain arithmetic would silence everything around it.
fn sanitised(sample: &mut f32) -> f32 {
    if !sample.is_finite() {
        *sample = 0.0;
    }
    sample.abs()
}

/// Backward pass: the gain may fall no faster than `slope` per sample, so it is
/// already down when the peak arrives. This is what the lookahead buys.
fn ramp_down_before_peaks(gain: &mut [f32], slope: f32) {
    for i in (0..gain.len().saturating_sub(1)).rev() {
        gain[i] = gain[i].min(gain[i + 1] + slope);
    }
}

/// Forward pass: the gain may rise no faster than `slope` per sample, so it eases
/// back to unity instead of snapping.
fn ramp_up_after_peaks(gain: &mut [f32], slope: f32) {
    for i in 1..gain.len() {
        gain[i] = gain[i].min(gain[i - 1] + slope);
    }
}

/// Gain change allowed per sample for a ramp lasting `seconds`.
fn slope(seconds: f32, rate: f32) -> f32 {
    1.0 / (seconds * rate).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn peak(buf: &[f32]) -> f32 {
        buf.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }

    fn sine(n: usize, hz: f32, amplitude: f32) -> Vec<f32> {
        (0..n)
            .map(|i| amplitude * (TAU * hz * i as f32 / 44_100.0).sin())
            .collect()
    }

    fn limited(buf: Stereo) -> Stereo {
        let mut buf = buf;
        apply(&mut buf, 44_100.0);
        buf
    }

    #[test]
    fn a_hot_signal_is_brought_under_the_ceiling() {
        let buf = limited(Stereo::centred(sine(44_100, 220.0, 3.0)));
        assert!(peak(&buf.l) <= CEILING + 1e-6, "peaked at {}", peak(&buf.l));
        assert!(peak(&buf.l) > 0.5, "but it is still a loud signal");
        assert_eq!(buf.l, buf.r, "and both sides took the same treatment");
    }

    #[test]
    fn a_quiet_signal_passes_through_untouched() {
        let original = Stereo::centred(sine(1000, 100.0, 0.5));
        assert_eq!(
            limited(original.clone()),
            original,
            "a limiter must be transparent below its ceiling"
        );
    }

    /// The reason the channels are linked: a peak on one side ducks both, so
    /// the balance between them is exactly what it was. Unlinked, the quiet
    /// side would keep its level and the image would lurch away from the loud
    /// one every time the limiter acted.
    #[test]
    fn a_peak_on_one_side_ducks_both_by_the_same_amount() {
        let mut buf = Stereo {
            l: sine(4410, 220.0, 3.0),
            r: sine(4410, 220.0, 0.5),
        };
        let before = peak(&buf.r);
        apply(&mut buf, 44_100.0);
        assert!(peak(&buf.l) <= CEILING + 1e-6, "the loud side is held");
        let after = peak(&buf.r);
        assert!(
            after < before * 0.9,
            "the quiet side did not follow: {before} → {after}"
        );
        let reduction = after / before;
        assert!(
            (reduction - CEILING / 3.0).abs() < 0.05,
            "the two sides took different gains ({reduction})"
        );
    }

    #[test]
    fn the_gain_is_already_down_when_a_lone_peak_arrives() {
        // Silence, then one enormous spike: the samples just before it are quiet,
        // so the ducking is only visible as the spike itself being tamed.
        let mut mono = vec![0.0; 4410];
        mono[2205] = 10.0;
        let buf = limited(Stereo::centred(mono));
        assert!(buf.l[2205] <= CEILING + 1e-6);
        // The gain ramps back over the release, not instantly.
        let mut gain = required_gain(&mut Stereo::centred(vec![0.0, 10.0, 0.0, 0.0]));
        ramp_down_before_peaks(&mut gain, slope(ATTACK, 44_100.0));
        ramp_up_after_peaks(&mut gain, slope(RELEASE, 44_100.0));
        assert!(gain[0] < 1.0, "ducked before the peak");
        assert!(gain[2] < 1.0, "and recovers gradually after it");
    }

    #[test]
    fn non_finite_samples_are_flushed_to_silence() {
        let buf = limited(Stereo {
            l: vec![f32::NAN, f32::INFINITY, 0.5, f32::NEG_INFINITY],
            r: vec![0.5, f32::NAN, f32::INFINITY, 0.25],
        });
        assert!(
            buf.l.iter().chain(&buf.r).all(|s| s.is_finite()),
            "got {buf:?}"
        );
        assert_eq!(buf.l[0], 0.0);
        assert_eq!(buf.r[1], 0.0);
    }

    #[test]
    fn an_empty_buffer_is_a_no_op() {
        assert!(limited(Stereo::silence(0)).is_empty());
    }
}
