//! The true-peak limiter a delivery is held under once it has been raised.
//!
//! The same limiter a bake passes through — linked channels, lookahead so the
//! gain is already down when a peak arrives, a slower release so it does not
//! pump, and the ceiling judged on the **true** peak — with one difference
//! that is the reason it is not that one: a bake is held whole in memory and
//! a render's mix is a file that can run to gigabytes. So this one streams. A
//! frame's gain can depend only on what is at most [`LOOKAHEAD`] ahead of it,
//! which makes that the delay between a sample going in and coming out, and
//! the whole of what is held.
//!
//! **Why the render may limit at all.** [`super::super::Mix`] does not, and
//! the headroom trim does not, because a limiter changes the dynamics of a mix
//! its author balanced. A loudness target is the author asking for exactly
//! that: a mix brought up past where its peaks fit has to have them held, and
//! the alternative — stopping short of the target — is the hand-tuning the
//! target exists to end (#968). Off unless asked for, and reported when it
//! acts.
//!
//! **Deterministic, and blind to how it is fed.** Each frame's gain is a
//! function of the signal alone, never of where a run happened to end, so a
//! mix limited a chunk at a time comes out sample for sample as it would
//! whole. The per-frame peaks are [`FramePeaks`]', the reconstruction the
//! meter in the report measures with — what this promises is what that says.

use std::collections::VecDeque;

use scorsese_zimmer::level::FramePeaks;

/// How far ahead a peak starts pulling the gain down, in seconds. The bake
/// limiter's figure: long enough to duck without a click.
const LOOKAHEAD: f64 = 0.002;

/// How long the gain takes to recover fully, in seconds. Slower than the
/// attack, or the gain pumps audibly.
const RELEASE: f64 = 0.06;

/// How far under the ceiling a frame's own peak is held, as a ratio: 0.05 dB.
///
/// The gain is one number per frame while an overshoot lives *between*
/// frames, so the gain the reconstruction sees around a peak is a hair higher
/// than the one that peak asked for — a hundredth of a decibel over, measured
/// on a tone just under a quarter of the rate. This pays for that, so the
/// ceiling is one the meter agrees was kept.
const GUARD: f64 = 0.994_260_7; // 10^(-0.05/20)

/// A streaming, linked, lookahead true-peak limiter.
#[derive(Debug)]
pub(crate) struct Limiter {
    channels: usize,
    /// The true peak no frame may exceed, as a linear magnitude.
    ceiling: f64,
    /// How far the gain may move per frame, falling and rising.
    attack: f64,
    release: f64,
    /// How many frames ahead a peak can reach back from.
    reach: u64,
    peaks: FramePeaks,
    /// Samples taken in and not yet given back, interleaved.
    waiting: VecDeque<f32>,
    /// How many frames have a known required gain, and how many have been
    /// given back.
    measured: u64,
    emitted: u64,
    /// The frames that can still bound a gain still to be given back, as
    /// `(frame, required gain, required gain + frame × attack)`, increasing
    /// in frame and in the last — the sliding minimum that makes the lookahead
    /// cost one comparison a frame. The gain is kept beside the key so that a
    /// frame's own need is read back exactly rather than through a
    /// subtraction that rounds it.
    window: VecDeque<(u64, f64, f64)>,
    /// The gain the last frame given back was played at.
    previous: f64,
    /// The lowest gain any frame was played at.
    deepest: f64,
}

impl Limiter {
    /// A limiter for `channels` interleaved channels at `rate`, holding the
    /// true peak under `ceiling_dbtp`.
    pub(crate) fn new(channels: usize, rate: u32, ceiling_dbtp: f64) -> Self {
        let rate = f64::from(rate.max(1));
        let ramp = |seconds: f64| 1.0 / (seconds * rate).max(1.0);
        Self {
            channels: channels.max(1),
            ceiling: 10_f64.powf(ceiling_dbtp / 20.0),
            attack: ramp(LOOKAHEAD),
            release: ramp(RELEASE),
            reach: (LOOKAHEAD * rate).ceil() as u64,
            peaks: FramePeaks::new(channels),
            waiting: VecDeque::new(),
            measured: 0,
            emitted: 0,
            window: VecDeque::new(),
            previous: f64::INFINITY,
            deepest: 1.0,
        }
    }

    /// Takes a run of samples and appends to `out` every frame whose gain is
    /// now settled — the input delayed by the lookahead.
    ///
    /// Non-finite samples are taken as silence: a `NaN` that reached the gain
    /// arithmetic would silence everything near it.
    pub(crate) fn feed(&mut self, samples: &[f32], out: &mut Vec<f32>) {
        let clean: Vec<f32> = samples
            .iter()
            .map(|&s| if s.is_finite() { s } else { 0.0 })
            .collect();
        self.waiting.extend(&clean);
        let mut needs = Vec::new();
        self.peaks.feed(&clean, |peak| needs.push(peak));
        for peak in needs {
            self.require(peak);
        }
        while self.measured > self.emitted + self.reach {
            self.emit(out);
        }
    }

    /// Gives back everything still held: the signal ends here.
    pub(crate) fn finish(&mut self, out: &mut Vec<f32>) {
        let mut needs = Vec::new();
        self.peaks.tail(|peak| needs.push(peak));
        for peak in needs {
            self.require(peak);
        }
        while self.emitted < self.measured {
            self.emit(out);
        }
    }

    /// How far the deepest gain reduction went, in dB: `0.0` when the limiter
    /// never acted.
    pub(crate) fn deepest_db(&self) -> f64 {
        -20.0 * self.deepest.log10()
    }

    /// Records the next frame's peak as the gain it would need on its own.
    fn require(&mut self, peak: f64) {
        let ceiling = self.ceiling * GUARD;
        let gain = if peak > ceiling { ceiling / peak } else { 1.0 };
        let key = gain + self.measured as f64 * self.attack;
        while self.window.back().is_some_and(|&(_, _, back)| back >= key) {
            self.window.pop_back();
        }
        self.window.push_back((self.measured, gain, key));
        self.measured += 1;
    }

    /// Plays the next frame at its gain: the lowest any peak within reach
    /// asks of it, ramped down to by the attack and limited in how fast it
    /// rises by the release.
    fn emit(&mut self, out: &mut Vec<f32>) {
        let frame = self.emitted;
        while self.window.front().is_some_and(|&(at, _, _)| at < frame) {
            self.window.pop_front();
        }
        let ahead = self.window.front().map_or(1.0, |&(at, gain, _)| {
            gain + (at - frame) as f64 * self.attack
        });
        let gain = ahead.min(self.previous + self.release).min(1.0);
        self.previous = gain;
        self.deepest = self.deepest.min(gain);
        for sample in self.waiting.drain(..self.channels) {
            out.push((sample * gain as f32).clamp(-1.0, 1.0));
        }
        self.emitted += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use scorsese_zimmer::level::Meter;

    use super::*;

    const RATE: u32 = 48_000;

    /// A stereo tone at `hz`, `left` and `right` its two amplitudes.
    fn tone(hz: f32, left: f32, right: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let s = (TAU * hz * i as f32 / RATE as f32).sin();
                [s * left, s * right]
            })
            .collect()
    }

    /// `samples` limited under `ceiling_dbtp`, fed `run` samples at a time.
    fn limited(samples: &[f32], run: usize, ceiling_dbtp: f64) -> (Vec<f32>, f64) {
        let mut limiter = Limiter::new(2, RATE, ceiling_dbtp);
        let mut out = Vec::new();
        for chunk in samples.chunks(run) {
            limiter.feed(chunk, &mut out);
        }
        limiter.finish(&mut out);
        (out, limiter.deepest_db())
    }

    fn true_peak(samples: &[f32]) -> f64 {
        let mut meter = Meter::new(2);
        meter.feed(samples);
        meter.finish().true_peak_dbfs.expect("audible")
    }

    #[test]
    fn a_hot_mix_comes_out_under_the_ceiling_as_the_meter_measures_it() {
        // A quarter-rate tone overshoots its samples, the case a sample-peak
        // limiter cannot see; a low one is the ordinary case.
        for hz in [220.0, RATE as f32 / 4.0 - 1.0] {
            let (out, deepest) = limited(&tone(hz, 3.0, 3.0, RATE as usize), 4_096, -1.0);
            let peak = true_peak(&out);
            assert!(peak <= -1.0, "{hz} Hz came out at {peak} dBTP");
            assert!(peak > -1.5, "and no lower than it had to: {peak}");
            assert!(deepest > 9.0, "a 3x signal needs about 10.5 dB: {deepest}");
        }
    }

    #[test]
    fn every_sample_comes_back_whatever_the_runs_and_the_same() {
        let signal = tone(330.0, 2.0, 0.5, 10_000);
        let (whole, deepest) = limited(&signal, signal.len(), -1.0);
        assert_eq!(whole.len(), signal.len());
        for run in [2, 14, 4_096] {
            let (chunked, depth) = limited(&signal, run, -1.0);
            assert_eq!(chunked, whole, "runs of {run}");
            assert_eq!(depth, deepest);
        }
    }

    /// One gain for both sides: the quiet side dips exactly as far as the loud
    /// one, so the image stays where the author put it.
    #[test]
    fn a_peak_on_one_side_ducks_both_by_the_same_amount() {
        let signal = tone(220.0, 3.0, 0.5, RATE as usize / 2);
        let (out, _) = limited(&signal, 4_096, -1.0);
        for (frame, pair) in out.chunks_exact(2).zip(signal.chunks_exact(2)) {
            if pair[1].abs() > 0.1 {
                let (left, right) = (frame[0] / pair[0], frame[1] / pair[1]);
                assert!((left - right).abs() < 1e-5, "{left} against {right}");
            }
        }
    }

    #[test]
    fn under_the_ceiling_it_is_transparent_and_says_so() {
        let signal = tone(220.0, 0.5, 0.25, 9_600);
        let (out, deepest) = limited(&signal, 1_000, -1.0);
        assert_eq!(out, signal);
        assert_eq!(deepest, 0.0);
    }

    #[test]
    fn non_finite_samples_come_back_as_silence() {
        let (out, _) = limited(&[f32::NAN, 0.25, f32::INFINITY, -0.25], 4, -1.0);
        assert_eq!(out, vec![0.0, 0.25, 0.0, -0.25]);
    }

    #[test]
    fn what_it_holds_is_the_lookahead_and_no_more() {
        let mut limiter = Limiter::new(2, RATE, -1.0);
        let mut out = Vec::new();
        for _ in 0..50 {
            limiter.feed(&[0.5; 4_096], &mut out);
            let held = limiter.waiting.len() / 2;
            assert!(held as u64 <= limiter.reach + 16, "held {held} frames");
        }
    }
}
