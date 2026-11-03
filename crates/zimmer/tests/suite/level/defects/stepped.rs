//! Stepped gain: a level moving in flat steps instead of smoothly.

use super::{RATE, found, tone};

/// The defect the measurement was asked for: a gain set once per game frame
/// and held for the frame, so a fade arrives as a staircase.
#[test]
fn a_fade_stepped_at_a_control_rate_is_zipper_with_its_period() {
    let period = 800.0 / f64::from(RATE); // 16.67 ms, a 60 Hz control rate
    let signal = tone(440.0, 0.5, 2.0, |t| {
        if !(0.5..1.5).contains(&t) {
            return if t < 0.5 { 0.0 } else { -30.0 };
        }
        -0.5 - ((t - 0.5) / period).floor() * 0.5
    });
    let defects = found(1, &signal);
    assert!(defects.zipper.count >= 1, "{defects:?}");
    let run = &defects.zipper.worst[0];
    let found_period = run.period_seconds.expect("a zipper has a period");
    assert!((found_period - period).abs() < 0.0005, "{run:?}");
    assert!((run.size_db - 0.5).abs() < 0.2, "{run:?}");
    assert!(run.from_seconds < 0.6 && run.to_seconds > 1.4, "{run:?}");
    assert_eq!(defects.clicks.count, 0, "{defects:?}");
}
