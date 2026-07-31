//! Where the energy sits: low, mid and high, as shares of the whole.
//!
//! Three numbers, because a number a reader can hold in their head beats a
//! curve they will skim. They are enough to say **muddy**, **thin** or
//! **balanced**, which are the failures that most reliably separate an amateur
//! mix from a competent one and which a level meter cannot see at all: a piece
//! can sit at a perfect −14 dBFS and still be unlistenable because everything
//! in it is stacked in the same octave.
//!
//! ## A complementary crossover, not an FFT
//!
//! Two one-pole lowpasses run over the signal, and the three bands are
//! `low = lp(LOW_HZ)`, `mid = lp(HIGH_HZ) − low`, `high = signal − lp(HIGH_HZ)`.
//! They **sum back to the input sample for sample**, so no energy is invented
//! or lost at a crossover and the three shares are shares of something real.
//!
//! It is deliberately coarse — 6 dB per octave, so a tone counts partly in its
//! neighbours even an octave or two from a crossover: a pure 12 kHz sine reads
//! as roughly four fifths high and one fifth mid rather than as all high. That
//! is the right trade here. An FFT would be a new dependency, or a few hundred
//! lines of one in a crate that deliberately has almost none, to sharpen a
//! number that is read as "about half the energy is down low" either way. The
//! question this answers is a ratio between wide regions, and a wide region
//! does not need a sharp edge.
//!
//! Energy, not amplitude: each band accumulates a sum of squares, and the
//! shares are of the total. That is the same quantity the mean level is built
//! from, so "42% low" and "mean −14 dBFS" are two views of one measurement
//! rather than two measurements that might disagree.

/// Where bass stops and the midrange starts, in Hz.
///
/// The mud region is just above this — 250 to 500 Hz is where a mix goes
/// boxy — so putting the boundary at its foot makes a swollen `low` share the
/// symptom of the thing an author would call muddy.
const LOW_HZ: f32 = 250.0;

/// Where the midrange stops and presence and air begin, in Hz.
///
/// Above this is where a mix reads as bright, harsh or thin; below it is where
/// nearly every instrument's fundamental lives.
const HIGH_HZ: f32 = 4_000.0;

/// The share of a signal's energy in each of three regions.
///
/// The three sum to `1.0` up to floating-point rounding. Kept as fractions
/// rather than percentages so a caller decides how to round for a reader, and
/// so a difference between two of them is not a difference between two roundings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bands {
    /// Below [`LOW_HZ`] — bass and the bottom of the mud region.
    pub low: f64,
    /// Between the two crossovers, where nearly every fundamental sits.
    pub mid: f64,
    /// Above [`HIGH_HZ`] — presence and air.
    pub high: f64,
}

impl Bands {
    /// The three shares as whole percentages, rounded so they still sum to 100.
    ///
    /// Rounding each independently can produce 99 or 101, which reads as a bug
    /// in the report rather than as rounding. The largest share absorbs the
    /// remainder, since a point moved onto the biggest number is the point
    /// least likely to change what a reader concludes.
    pub fn percentages(&self) -> (u32, u32, u32) {
        let percent = |share: f64| (share * 100.0).round().clamp(0.0, 100.0) as i32;
        let mut shares = [percent(self.low), percent(self.mid), percent(self.high)];
        let biggest = shares
            .iter()
            .enumerate()
            .max_by_key(|(_, share)| **share)
            .map_or(0, |(at, _)| at);
        shares[biggest] += 100 - shares.iter().sum::<i32>();
        let read = |share: i32| share.clamp(0, 100) as u32;
        (read(shares[0]), read(shares[1]), read(shares[2]))
    }
}

/// Accumulates band energies over samples arriving a run at a time.
///
/// A run at a time for the same reason [`super::Meter`] is: a render's mixdown
/// is written segment by segment and never held whole. The filter state is
/// per channel and survives across runs, so a seam between two runs is not a
/// step change the filters have to recover from.
#[derive(Debug, Clone)]
pub struct BandMeter {
    channels: usize,
    /// One filter pair per channel — filtering across interleaved channels
    /// would be filtering a signal that alternates between two different
    /// sources, which is noise rather than a band split.
    filters: Vec<Crossover>,
    /// Which channel the next sample belongs to, kept across runs so a run
    /// that does not end on a frame boundary cannot rotate the channels.
    next: usize,
    low: f64,
    mid: f64,
    high: f64,
}

impl BandMeter {
    /// A meter for `channels` interleaved channels at `rate` samples a second.
    pub fn new(channels: usize, rate: u32) -> Self {
        let channels = channels.max(1);
        Self {
            channels,
            filters: vec![Crossover::new(rate); channels],
            next: 0,
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        }
    }

    /// Takes another run of interleaved samples.
    pub fn feed(&mut self, samples: &[f32]) {
        for &sample in samples {
            let (low, mid, high) = self.filters[self.next].split(sample);
            self.low += f64::from(low) * f64::from(low);
            self.mid += f64::from(mid) * f64::from(mid);
            self.high += f64::from(high) * f64::from(high);
            self.next = (self.next + 1) % self.channels;
        }
    }

    /// The shares as they stand, or `None` when nothing was heard.
    ///
    /// `None` rather than three zeroes or three thirds: a silence has no
    /// spectral balance, and inventing one would put a row of plausible
    /// percentages under a clip that makes no sound.
    pub fn finish(&self) -> Option<Bands> {
        let total = self.low + self.mid + self.high;
        if total <= 0.0 {
            return None;
        }
        Some(Bands {
            low: self.low / total,
            mid: self.mid / total,
            high: self.high / total,
        })
    }
}

/// One channel's pair of lowpasses, and the three-way split they define.
#[derive(Debug, Clone, Copy)]
struct Crossover {
    low_alpha: f32,
    high_alpha: f32,
    low_state: f32,
    high_state: f32,
}

impl Crossover {
    fn new(rate: u32) -> Self {
        Self {
            low_alpha: alpha(LOW_HZ, rate),
            high_alpha: alpha(HIGH_HZ, rate),
            low_state: 0.0,
            high_state: 0.0,
        }
    }

    /// Splits one sample into three that add back up to it.
    fn split(&mut self, sample: f32) -> (f32, f32, f32) {
        // Non-finite input would poison the filter state for the rest of the
        // signal, so it is dropped rather than propagated — the same guard the
        // patch filter keeps, and for the same reason.
        let sample = if sample.is_finite() { sample } else { 0.0 };
        self.low_state += self.low_alpha * (sample - self.low_state);
        self.high_state += self.high_alpha * (sample - self.high_state);
        (
            self.low_state,
            self.high_state - self.low_state,
            sample - self.high_state,
        )
    }
}

/// The one-pole coefficient for a cutoff, `1 − e^(−2π·fc/rate)`.
///
/// Clamped into `0..=1`: a cutoff at or above the Nyquist frequency is a filter
/// that passes everything, and a rate of zero would otherwise produce a
/// coefficient that makes the state diverge.
fn alpha(cutoff: f32, rate: u32) -> f32 {
    if rate == 0 {
        return 1.0;
    }
    let exponent = -std::f32::consts::TAU * cutoff / rate as f32;
    (1.0 - exponent.exp()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the whole three-number report rests on: nothing is invented
    /// and nothing is lost at a crossover, so the shares are shares of the
    /// signal that was actually fed in.
    #[test]
    fn the_three_bands_add_back_up_to_the_input() {
        let mut crossover = Crossover::new(48_000);
        for i in 0..500 {
            let sample = (i as f32 * 0.37).sin() * 0.8;
            let (low, mid, high) = crossover.split(sample);
            assert!((low + mid + high - sample).abs() < 1e-5);
        }
    }

    /// Percentages are rounded to sum to 100, because a row reading
    /// "low 42% mid 51% high 8%" looks like a bug rather than like rounding.
    #[test]
    fn percentages_always_sum_to_a_hundred() {
        for (low, mid, high) in [
            (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0),
            (0.425, 0.505, 0.07),
            (1.0, 0.0, 0.0),
            (0.005, 0.005, 0.99),
        ] {
            let (low, mid, high) = Bands { low, mid, high }.percentages();
            assert_eq!(low + mid + high, 100, "{low} + {mid} + {high}");
        }
    }
}
