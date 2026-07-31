//! What a render says about how loud it came out.
//!
//! Split in two by what the number is *for*: [`mix`] is the delivered
//! soundtrack's own level, and [`clips`] is which clip put it there.
//!
//! A **signal**, so nothing here asserts that a render was refused — the point
//! is the opposite: every one of these renders succeeds, and what is under test
//! is whether the report carries a number an author could have acted on.
//!
//! The defects that argued for this were content and not code: a mix ten
//! decibels too quiet passes every other test in this directory, because
//! `assert_audible` proves there *is* sound and not that the sound is at a sane
//! level.

mod clips;
mod mix;

/// Decibels of slack between what the meter says and what the amplitude was.
///
/// Wide, deliberately: the tone is encoded, decoded and resampled on its way
/// through, and this is asserting that the report is *about this signal* rather
/// than re-deriving the encoder's arithmetic.
pub(super) const SLACK: f64 = 2.0;

/// Where a mono sine of amplitude `a` lands in the delivered **stereo** mix.
///
/// Two separate 3 dB, and conflating them is the easy mistake here:
///
/// - a sine's RMS is `a/√2`, which is 3.01 dB below its own peak;
/// - a mono source upmixed to stereo is spread across two channels by a
///   power-preserving matrix, so each channel carries `1/√2` of it — the same
///   3 dB the audio helpers in `tests/common` already document.
///
/// The meter measures what was delivered, which is the stereo mix, so both
/// apply. This test exists to pin that: a report that quietly measured the
/// source instead would read 3 dB high and nobody would notice.
pub(super) fn sine_mean_dbfs(amplitude: f64) -> f64 {
    20.0 * (amplitude / std::f64::consts::SQRT_2 / std::f64::consts::SQRT_2).log10()
}
