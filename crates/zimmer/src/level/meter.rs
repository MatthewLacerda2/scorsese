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

use super::intersample::{Channel, TAPS};

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

/// How many channels a correlation is a statement about.
///
/// Two, and only two. Width is a relationship between a *pair* of channels: a
/// mono signal has no second channel to be wide against, and a signal of more
/// than two has no one pair the number would be about.
const STEREO: usize = 2;

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
    /// Sum of `l·r` over every frame — how much the two channels agree, and
    /// the numerator of [`Meter::correlation`].
    sum_of_products: f64,
    /// Each channel's own energy, `Σl²` and `Σr²`, which is what that
    /// numerator has to be normalised by. Kept apart from
    /// [`Meter::sum_of_squares`], which is both channels together: a mean is
    /// about the signal and a correlation is about the two sides of it.
    sum_of_left: f64,
    sum_of_right: f64,
    /// The end of the signal so far: the frames not yet measured, preceded by
    /// the [`TAPS`] already-measured frames that are their left-hand context.
    ///
    /// Never more than twice that many frames, whatever a caller feeds — this
    /// is a window on the seam and not a copy of the signal.
    recent: Vec<f32>,
    /// How many leading frames of [`Meter::recent`] have already been counted.
    /// The rest are waiting for their right-hand neighbours to arrive.
    settled: usize,
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
            sum_of_products: 0.0,
            sum_of_left: 0.0,
            sum_of_right: 0.0,
            recent: Vec::new(),
            settled: 0,
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
        if self.channels == STEREO {
            for frame in samples.chunks_exact(STEREO) {
                let (left, right) = (f64::from(frame[0]), f64::from(frame[1]));
                self.sum_of_products += left * right;
                self.sum_of_left += left * left;
                self.sum_of_right += right * right;
            }
        }
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
            true_peak_dbfs: Some(ratio_to_dbfs(self.true_peak().max(self.peak))),
            mean_dbfs: Some(ratio_to_dbfs(mean_square.sqrt())),
        }
    }

    /// How much of the signal is common to both channels, in `-1..=1`.
    ///
    /// The third question a row answers, after how loud and where. `1.0` is
    /// the same waveform in both ears — mono in a stereo container, which is
    /// what a score that never used the `pan` it has comes out as. `0.0` is
    /// two channels with nothing in common.
    ///
    /// **Negative is the defect.** It means the channels are cancelling, and
    /// the energy that cancels is gone the moment anything folds the mix down
    /// to mono — which is not hypothetical for a video played on a phone or a
    /// laptop. Everything above zero is a taste; below it is a fault.
    ///
    /// `None` where there is no width to speak of: a meter of anything other
    /// than two channels, and a signal with a silent channel, where the
    /// arithmetic is a division by zero rather than a zero.
    pub fn correlation(&self) -> Option<f64> {
        if self.channels != STEREO {
            return None;
        }
        let energy = self.sum_of_left * self.sum_of_right;
        if energy <= 0.0 {
            return None;
        }
        // Clamped because the arithmetic lands a hair outside the range it is
        // defined over in the case that matters most: two identical channels
        // sum to the same number three times and divide to 1.0 give or take an
        // ulp, and a report saying a signal is 1.0000000002 wide reads as a
        // bug in the meter rather than as the mono it is.
        Some((self.sum_of_products / energy.sqrt()).clamp(-1.0, 1.0))
    }

    /// Oversamples each channel and keeps the largest excursion found.
    ///
    /// Per channel rather than across the interleaved buffer — see
    /// [`Meter::new`] for why that distinction matters.
    ///
    /// **A frame is only measurable once both its neighbourhoods exist.** The
    /// kernel reaches [`TAPS`] frames either side of the frame it reconstructs,
    /// and anything outside the buffer reads as zero — so a frame measured
    /// while it still sits at the end of the newest run is measured against a
    /// silence that is about to be replaced by real samples. That fabricated edge rings, the ringing is an
    /// excursion, and a running maximum keeps it forever: a signal fed in
    /// 4 KB runs used to read a decibel hotter than the same signal fed whole.
    ///
    /// So each run measures the frames from [`Meter::settled`] up to `TAPS`
    /// short of the end, keeps the last `TAPS` measured frames as the next
    /// run's left-hand context, and holds the rest back. The one place the
    /// zeros are real is the end of the signal, which is
    /// [`Meter::true_peak`]'s business.
    fn measure_true_peak(&mut self, samples: &[f32]) {
        let mut joined = std::mem::take(&mut self.recent);
        joined.extend_from_slice(samples);
        let frames = joined.len() / self.channels;
        // `max` rather than a bare subtraction: a run shorter than the kernel
        // adds no measurable frames at all, and must not un-measure any.
        let ready = frames.saturating_sub(TAPS).max(self.settled);
        for index in 0..self.channels {
            let channel = Channel::of(&joined, self.channels, index);
            for frame in self.settled..ready {
                self.true_peak = self.true_peak.max(channel.peak_from(frame));
            }
        }
        let drop = ready.saturating_sub(TAPS);
        self.settled = ready - drop;
        joined.drain(..drop * self.channels);
        self.recent = joined;
    }

    /// The largest excursion anywhere, including the frames still held back.
    ///
    /// Those are measured here rather than in [`Meter::feed`] because here is
    /// the only moment their right-hand neighbours are known to be silence:
    /// whoever is asking has stopped feeding, so the signal ends where the
    /// buffer does. Read rather than accumulated, so asking twice — or asking
    /// and then feeding more — gives the answer for the signal as it stands
    /// each time.
    fn true_peak(&self) -> f64 {
        let mut peak = self.true_peak;
        for index in 0..self.channels {
            let channel = Channel::of(&self.recent, self.channels, index);
            for frame in self.settled..channel.frames() {
                peak = peak.max(channel.peak_from(frame));
            }
        }
        peak
    }
}

/// A linear amplitude ratio as decibels below full scale.
fn ratio_to_dbfs(ratio: f64) -> f64 {
    20.0 * ratio.log10()
}
