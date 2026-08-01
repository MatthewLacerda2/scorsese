//! Two measurements, compared.
//!
//! The comparison is where a measurement becomes a judgement — "4.6 dB under
//! the version it was meant to replace" is a finding, and "−14.4 dBFS" on its
//! own is not — so what these assert is that the arithmetic of a difference is
//! exact in both directions.

use scorsese_soundgen::level::{Difference, Profiler};
use scorsese_soundgen::{SAMPLE_RATE, bake_song, song::InlineOnly};

use super::{SLACK, common::songs, square};

/// Measures one buffer whole.
fn profiled(samples: &[f32]) -> scorsese_soundgen::level::Profile {
    let mut profiler = Profiler::new(1, SAMPLE_RATE);
    profiler.feed(samples);
    profiler.finish()
}

/// The case that has to come out clean, because it is what a re-bake of an
/// unchanged recipe produces: nothing moved, in any field.
#[test]
fn a_file_compared_with_itself_differs_by_nothing() {
    let bake = bake_song(&songs::song(), &InlineOnly).expect("the fixture song renders");
    let difference = Difference::between(&bake.profile.whole, &bake.profile.whole);
    assert!(difference.is_nothing(), "{difference:?}");
    assert_eq!(difference.mean_db, Some(0.0));
    assert_eq!(difference.peak_db, Some(0.0));
    assert_eq!(difference.seconds, 0.0);
    let bands = difference.bands.expect("a piece of music has a balance");
    assert_eq!(bands.largest(), 0.0);
}

/// Halving the amplitude is exactly −6.02 dB, and a comparison has to say so —
/// this is the arithmetic the two "4.5 dB quiet" defects were diagnosed by.
#[test]
fn a_quieter_file_reads_as_exactly_that_many_decibels_quieter() {
    let loud = profiled(&square(1.0, 4_000));
    let quiet = profiled(&square(0.5, 4_000));

    let down = Difference::between(&quiet.whole, &loud.whole);
    assert!(!down.is_nothing());
    assert!((down.mean_db.expect("audible") + 6.02).abs() < SLACK);
    assert!((down.peak_db.expect("audible") + 6.02).abs() < SLACK);
    // Crest is peak minus mean, and a square has none of either way round, so
    // scaling the whole signal cannot change it.
    assert!(down.crest_db.expect("audible").abs() < SLACK);

    // And the other way round, since a comparison whose sides did not mirror
    // would make "quieter" depend on which file was named first.
    let up = Difference::between(&loud.whole, &quiet.whole);
    assert!((up.mean_db.expect("audible") - 6.02).abs() < SLACK);
}

/// Nothing to compare against is not the same as no difference: a silence has
/// no level, so the honest answer is that there is no number rather than zero.
#[test]
fn a_silence_has_no_difference_rather_than_a_difference_of_zero() {
    let sound = profiled(&square(0.5, 2_000));
    let silence = profiled(&[0.0; 2_000]);

    let difference = Difference::between(&sound.whole, &silence.whole);
    assert_eq!(difference.mean_db, None);
    assert_eq!(difference.peak_db, None);
    assert_eq!(difference.bands, None);
    // The lengths are still comparable, because a silence has one.
    assert!(difference.seconds.abs() < 0.001);
}
