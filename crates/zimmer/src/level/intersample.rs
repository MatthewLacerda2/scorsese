//! The waveform *between* the samples, reconstructed.
//!
//! A sample is not the signal — it is a point on a band-limited curve, and the
//! curve between two samples can exceed both of them. So a buffer whose every
//! sample sits under full scale can still clip the moment anything reconstructs
//! it: a converter on the way to a speaker, a resampler on the way into a mix,
//! an AAC encoder on the way into a delivered video. All three are on paths
//! this workspace already puts audio through.
//!
//! **One copy of the arithmetic, read by two callers, and that is the whole
//! reason it is a module.** [`super::meter`] reports the true peak; the
//! [limiter](crate::fx::limiter) holds a bake under it. When they reconstructed
//! the waveform differently the limiter could satisfy its own guarantee and be
//! reported as clipping by the meter that shipped beside it — which is exactly
//! what happened, and is a class of bug no amount of care in either file would
//! have caught. Sharing the kernel makes the two agree by construction; all
//! that is left to choose is the number each holds against.

/// Oversampling factor.
///
/// Four is what ITU-R BS.1770 asks for at these rates, and it is enough: the
/// error left over is a few hundredths of a decibel, which is well inside the
/// headroom a true-peak ceiling leaves anyway.
pub(crate) const OVERSAMPLE: usize = 4;

/// Half-width of the interpolating kernel, in input samples either side.
///
/// Eight taps total. A windowed sinc rather than linear interpolation, because
/// linear interpolation between two samples never exceeds the larger of them —
/// it would report the sample peak again under a longer name, which is worse
/// than not reporting it.
///
/// Also how far a caller feeding a signal a run at a time has to see back:
/// keeping this many frames of the previous run is what stops every segment
/// boundary from being reconstructed as a pair of edges.
pub(crate) const TAPS: usize = 4;

/// How many points are computed strictly between one sample and the next.
const STEPS: usize = OVERSAMPLE - 1;

/// One channel of an interleaved run, read as the continuous signal it stands
/// for.
///
/// Per channel rather than across the interleaved buffer: interpolating
/// between a left sample and the right sample beside it is interpolating
/// between two different signals, and would invent an excursion at every frame
/// boundary.
pub(crate) struct Channel<'a> {
    samples: &'a [f32],
    channels: usize,
    channel: usize,
    frames: usize,
    /// The kernel, evaluated once per channel rather than once per sample.
    ///
    /// The weights depend only on which tap and which sub-sample position, so
    /// there are `STEPS × (2·TAPS + 1)` of them for any signal of any length.
    /// Computing them inline meant two `sin` calls per tap per step per frame —
    /// fifty-four transcendentals to place one sample — which was affordable
    /// while the meter was the only caller and is not now that the limiter
    /// reconstructs every bake as well.
    weights: [[f64; TAPS * 2 + 1]; STEPS],
}

impl<'a> Channel<'a> {
    /// The `channel`th of `channels` woven together in `samples`.
    pub(crate) fn of(samples: &'a [f32], channels: usize, channel: usize) -> Self {
        let channels = channels.max(1);
        let mut weights = [[0.0; TAPS * 2 + 1]; STEPS];
        for (step, row) in weights.iter_mut().enumerate() {
            let offset = (step + 1) as f64 / OVERSAMPLE as f64;
            for (tap, weight) in row.iter_mut().enumerate() {
                *weight = lanczos(tap as f64 - TAPS as f64 - offset);
            }
        }
        Self {
            samples,
            channels,
            channel,
            frames: samples.len() / channels,
            weights,
        }
    }

    /// A signal that is already one channel on its own.
    pub(crate) fn mono(samples: &'a [f32]) -> Self {
        Self::of(samples, 1, 0)
    }

    /// How many sample-frames long it is.
    pub(crate) fn frames(&self) -> usize {
        self.frames
    }

    /// The largest excursion between `frame` and the frame after it.
    ///
    /// The samples themselves are not included: a caller that has them in hand
    /// already knows how loud they are, and every caller here does.
    pub(crate) fn peak_past(&self, frame: usize) -> f64 {
        (0..STEPS)
            .map(|step| self.between(frame, step).abs())
            .fold(0.0, f64::max)
    }

    /// The value `step + 1` sub-samples of the way past `frame`.
    fn between(&self, frame: usize, step: usize) -> f64 {
        let mut sum = 0.0;
        for (tap, weight) in self.weights[step].iter().enumerate() {
            let at = frame as isize + tap as isize - TAPS as isize;
            sum += self.at(at) * weight;
        }
        sum
    }

    /// One sample of this channel. Outside the run it reads as zero, which is
    /// what the signal is before it starts and after it ends.
    fn at(&self, frame: isize) -> f64 {
        if frame < 0 || frame as usize >= self.frames {
            return 0.0;
        }
        f64::from(self.samples[frame as usize * self.channels + self.channel])
    }
}

/// The Lanczos kernel, zero outside [`TAPS`].
fn lanczos(x: f64) -> f64 {
    if x.abs() >= TAPS as f64 {
        return 0.0;
    }
    sinc(x) * sinc(x / TAPS as f64)
}

/// Normalised sinc, `1` at zero.
fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        return 1.0;
    }
    let x = std::f64::consts::PI * x;
    x.sin() / x
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The case the whole module exists for, in three samples: a signal whose
    /// samples are all under full scale and whose waveform is not.
    #[test]
    fn the_curve_between_two_samples_can_exceed_both() {
        // A quarter-rate tone read half a sample late: +a, +a, -a, -a.
        let a = 0.7;
        let tone: Vec<f32> = (0..64)
            .map(|i| if (i / 2) % 2 == 0 { a } else { -a })
            .collect();
        let channel = Channel::mono(&tone);
        let peak = (16..48).map(|f| channel.peak_past(f)).fold(0.0, f64::max);
        assert!(peak > f64::from(a) * 1.2, "read only {peak}");
    }

    /// A signal that is not moving has nothing between its samples to find —
    /// to within a quarter of a percent, which is where the truncated kernel
    /// lands and is the error every reading here carries.
    ///
    /// It errs *upward*, which is the direction both callers want: a meter
    /// that over-reports full scale by two hundredths of a decibel says
    /// "clipping" a shade early, and a limiter reading the same way ducks a
    /// shade harder than it had to.
    #[test]
    fn a_constant_reconstructs_to_itself() {
        let flat = vec![0.5; 64];
        let channel = Channel::mono(&flat);
        for frame in 8..56 {
            let between = channel.peak_past(frame);
            assert!(
                between >= 0.5,
                "frame {frame} read under the signal: {between}"
            );
            assert!(between < 0.5 * 1.005, "frame {frame} read {between}");
        }
    }

    /// Reading one channel of an interleaved pair must never mix the two —
    /// that is what would invent an excursion at every frame boundary. Here
    /// each channel on its own is a constant while the interleaved buffer
    /// alternates, so a reading that strayed across would be unmissable.
    #[test]
    fn a_channel_reads_only_its_own_samples() {
        let interleaved: Vec<f32> = (0..64)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let left = Channel::of(&interleaved, 2, 0);
        let right = Channel::of(&interleaved, 2, 1);
        assert_eq!(left.frames(), 32);
        assert_eq!(right.frames(), 32);
        assert!((left.peak_past(16) - 1.0).abs() < 0.005);
        assert!((right.peak_past(16) - 1.0).abs() < 0.005);
    }

    #[test]
    fn an_empty_run_has_no_frames_and_reads_as_silence() {
        let channel = Channel::mono(&[]);
        assert_eq!(channel.frames(), 0);
        assert_eq!(channel.peak_past(0), 0.0);
        assert_eq!(channel.at(-1), 0.0);
    }
}
