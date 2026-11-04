//! What a staircase must be to be reported, each limit met by one built just
//! inside it and missed by one just outside: how large a step, how many, how
//! regular — and how exactly what is found is measured.

use super::{RATE, found, tone};

/// Two seconds of a 440 Hz tone at 0.5, gained by `steps(n)` dB from half a
/// second on, where `n` counts the periods of `samples` since then.
fn staircase(samples: f64, steps: impl Fn(f64) -> f64) -> Vec<f32> {
    let period = samples / f64::from(RATE);
    tone(440.0, 0.5, 2.0, |t| {
        steps(((t - 0.5).max(0.0) / period).floor())
    })
}

/// Fifty-four steps at 60 Hz, `size` dB each.
fn even(size: f64) -> impl Fn(f64) -> f64 {
    move |n| -size * n.min(54.0)
}

/// A quarter of a decibel a step is the smallest reported: three tenths are,
/// two tenths are not, however regular.
#[test]
fn a_step_is_at_least_a_quarter_of_a_decibel() {
    let reported = found(1, &staircase(800.0, even(0.3)));
    assert_eq!(reported.zipper.count, 1, "{reported:?}");
    let under = found(1, &staircase(800.0, even(0.2)));
    assert_eq!(under.zipper.count, 0, "{under:?}");
}

/// A staircase takes ten steps to be reported: nine are a sound's flutter
/// lining up as often as a game's fade.
#[test]
fn a_staircase_takes_ten_steps() {
    let ten = found(1, &staircase(800.0, |n| -2.0 * n.min(10.0)));
    assert_eq!(ten.zipper.count, 1, "{ten:?}");
    let nine = found(1, &staircase(800.0, |n| -2.0 * n.min(9.0)));
    assert_eq!(nine.zipper.count, 0, "{nine:?}");
}

/// Four in five steps must go the staircase's way by half its mean step: one
/// in every three under a third the size is too irregular.
#[test]
fn most_steps_go_the_staircases_way() {
    let ragged = |every: f64| move |n: f64| -(n - (n / every).floor() * 0.7).min(54.0);
    let reported = found(1, &staircase(800.0, ragged(5.0)));
    assert_eq!(reported.zipper.count, 1, "{reported:?}");
    let under = found(1, &staircase(800.0, ragged(3.0)));
    assert_eq!(under.zipper.count, 0, "{under:?}");
}

/// A staircase's period is refined to a fraction of a block of half a
/// millisecond, and its step is read to within a twentieth.
#[test]
fn a_staircase_is_measured_to_a_fraction_of_a_block() {
    for samples in [600.0, 800.0, 1_234.0] {
        for size in [0.5, 1.0] {
            let defects = found(1, &staircase(samples, even(size)));
            assert_eq!(defects.zipper.count, 1, "{samples} by {size}: {defects:?}");
            let run = &defects.zipper.worst[0];
            let period = run.period_seconds.expect("a period") * f64::from(RATE);
            assert!(
                (period - samples).abs() < 4.0,
                "{samples} by {size}: {run:?}"
            );
            assert!(
                (run.size_db / size - 1.0).abs() < 0.06,
                "{samples} by {size}: {run:?}"
            );
        }
    }
}
