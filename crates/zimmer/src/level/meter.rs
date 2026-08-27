//! How loud a signal came out: peak, true peak, and mean level.
//!
//! The *how loud* half of [`super`]. One number per statistic over whatever
//! samples it is fed, and nothing here knows how long the thing being measured
//! is or what part of it this is — [`super::profile`] decides that by handing
//! one of these a stretch at a time, and [`super::bands`] answers the other
//! question a report asks, which is **where** the energy sits.
//!
//! The **true** peak is the one that takes work, and [`super::intersample`]
//! does it: sample peak under-reads what a lossy encoder will produce, because
//! the waveform between two samples can exceed both of them, so a mix reading
//! −0.2 dBFS can clip after AAC. That is precisely the case a delivery render
//! should mention and precisely the one a sample peak cannot see.

use super::intersample::Channel;

/// Full scale, as the number a sample of `1.0` is.
const FULL_SCALE: f64 = 1.0;

/// How far above full scale is worth calling out. Sample peak exactly at `1.0`
/// is what a mix is clamped to on its way to a file, so equality is the
/// interesting case rather than a strict excess.
///
/// Held against the true peak, and so is the [limiter](crate::fx::limiter)'s
/// own ceiling — a decibel under this one, from the same reconstruction. That
/// is what makes the guarantee and the measurement agree rather than two
/// files each being reasonable on its own.
const CLIPPING: f64 = FULL_SCALE;

/// How loud a signal is, in dBFS.
///
/// Every field is `None` for a signal that is entirely silent. A silence is not
/// "minus infinity decibels" to anyone reading a report — it is a clip that
/// makes no sound, which is a different sentence and usually a more urgent one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    /// The loudest single sample.
    pub peak_dbfs: Option<f64>,
    /// The loudest point of the waveform *between* samples, which is what an
    /// encoder will have to reproduce.
    pub true_peak_dbfs: Option<f64>,
    /// Root mean square over the whole signal — the number that says whether
    /// this sits where its neighbours sit.
    pub mean_dbfs: Option<f64>,
}

impl Loudness {
    /// True when nothing was heard at all.
    pub fn is_silent(&self) -> bool {
        self.peak_dbfs.is_none()
    }

    /// True when the waveform reaches or passes full scale, so the delivered
    /// file is clipping. Judged on **true** peak, since that is what survives
    /// the encoder.
    pub fn is_clipping(&self) -> bool {
        self.true_peak_dbfs
            .is_some_and(|dbfs| dbfs >= ratio_to_dbfs(CLIPPING))
    }
}

/// Accumulates loudness over samples arriving a run at a time.
///
/// A run at a time because the mixdown is written segment by segment and never
/// held whole: holding a minute of stereo float to measure it would be sixty
/// megabytes to learn three numbers.
#[derive(Debug, Clone)]
pub struct Meter {
    channels: usize,
    peak: f64,
    true_peak: f64,
    sum_of_squares: f64,
    counted: u64,
    /// The tail of the previous run, so the interpolator sees across the seam
    /// rather than treating every segment boundary as a pair of edges.
    tail: Vec<f32>,
}

impl Meter {
    /// A meter for a signal of `channels` interleaved channels.
    ///
    /// The count is asked for rather than assumed because the caller knows and
    /// this does not — an imported file may be anything — and because getting
    /// it wrong is not a rounding error: interpolating between a left sample
    /// and the right sample beside it is interpolating between two different
    /// signals, and would invent excursions at every frame boundary.
    pub fn new(channels: usize) -> Self {
        Self {
            channels: channels.max(1),
            peak: 0.0,
            true_peak: 0.0,
            sum_of_squares: 0.0,
            counted: 0,
            tail: Vec::new(),
        }
    }

    /// Takes another run of interleaved samples.
    pub fn feed(&mut self, samples: &[f32]) {
        for &sample in samples {
            let magnitude = f64::from(sample).abs();
            self.peak = self.peak.max(magnitude);
            self.sum_of_squares += f64::from(sample) * f64::from(sample);
        }
        self.counted += samples.len() as u64;
        self.measure_true_peak(samples);
    }

    /// What the signal came out as.
    pub fn finish(&self) -> Loudness {
        if self.counted == 0 || self.peak == 0.0 {
            return Loudness {
                peak_dbfs: None,
                true_peak_dbfs: None,
                mean_dbfs: None,
            };
        }
        let mean_square = self.sum_of_squares / self.counted as f64;
        Loudness {
            peak_dbfs: Some(ratio_to_dbfs(self.peak)),
            // Never below the sample peak: the interpolated curve passes
            // through every sample, so a kernel that somehow read lower would
            // be reporting a waveform that does not contain its own samples.
            true_peak_dbfs: Some(ratio_to_dbfs(self.true_peak.max(self.peak))),
            mean_dbfs: Some(ratio_to_dbfs(mean_square.sqrt())),
        }
    }

    /// Oversamples each channel and keeps the largest excursion found.
    ///
    /// Per channel rather than across the interleaved buffer — see
    /// [`Meter::new`] for why that distinction matters.
    fn measure_true_peak(&mut self, samples: &[f32]) {
        let mut joined = std::mem::take(&mut self.tail);
        joined.extend_from_slice(samples);
        for index in 0..self.channels {
            let channel = Channel::of(&joined, self.channels, index);
            for frame in 0..channel.frames() {
                self.true_peak = self.true_peak.max(channel.peak_from(frame));
            }
        }
        // Keep enough of the end that the next run's first frames have their
        // left-hand taps.
        let keep = (super::intersample::TAPS * self.channels).min(joined.len());
        self.tail = joined.split_off(joined.len() - keep);
    }
}

/// A linear amplitude ratio as decibels below full scale.
fn ratio_to_dbfs(ratio: f64) -> f64 {
    20.0 * ratio.log10()
}
