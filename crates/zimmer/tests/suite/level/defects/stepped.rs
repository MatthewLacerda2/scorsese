//! Stepped gain: a level moving in flat steps instead of smoothly.

use super::{RATE, found, steady, tone};

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

/// A fade over a quarter of a second — fifteen steps — is found on its own,
/// in a file where most of the sound is steady. Its last few steps may go
/// unclaimed: a window that reaches past the end of a staircase is mostly
/// flat, and only windows that are mostly staircase vouch for one.
#[test]
fn a_short_staircase_is_found_where_it_is() {
    let period = 800.0 / f64::from(RATE);
    let signal = tone(330.0, 0.5, 2.0, |t| {
        -2.0 * ((t - 1.0).max(0.0) / period).floor().min(15.0)
    });
    let defects = found(1, &signal);
    assert_eq!(defects.zipper.count, 1, "{defects:?}");
    let run = &defects.zipper.worst[0];
    assert!((run.size_db - 2.0).abs() < 0.3, "{run:?}");
    assert!((run.from_seconds - 1.0).abs() < 0.03, "{run:?}");
    assert!((1.15..1.27).contains(&run.to_seconds), "{run:?}");
}

/// A fade-out at the very end of a file is judged like one anywhere else,
/// whatever the file's length.
#[test]
fn a_staircase_at_the_end_of_a_file_is_found() {
    let period = 800.0 / f64::from(RATE);
    for seconds in [1.0, 2.0, 2.5] {
        let from = seconds - 0.35;
        let signal = tone(440.0, 0.5, seconds, |t| {
            -((t - from).max(0.0) / period).floor().min(20.0)
        });
        let defects = found(1, &signal);
        assert_eq!(defects.zipper.count, 1, "a {seconds} s file: {defects:?}");
    }
}

/// A note rich in harmonics pulses once a cycle, at a period in the same
/// range a game's control rate is, and as it dies away the pulse can read as
/// a staircase. Stepped gain is not reported on a sound that pulses on its
/// own: it misses there rather than invents.
#[test]
fn a_low_note_dying_away_is_not_a_staircase() {
    for hz in [60.0, 120.0, 180.0, 250.0, 270.0, 339.0, 523.0] {
        for decay in [3.0, 10.0, 30.0, 60.0] {
            let signal = buzz(hz, 1.0, |t| -decay * t);
            let defects = found(1, &signal);
            assert_eq!(
                defects.zipper.count, 0,
                "{hz} Hz at -{decay} dB/s: {defects:?}"
            );
        }
    }
}

/// Noise is nothing but random change, and a staircase under it is still
/// found: the steps all go one way, and noise goes every way.
#[test]
fn noise_faded_in_steps_is_a_staircase() {
    let period = 800.0 / f64::from(RATE);
    let signal: Vec<f32> = super::noise(0.3, 3 * RATE as usize / 2)
        .into_iter()
        .enumerate()
        .map(|(i, x)| x * 10f32.powf(-(i as f32 / 800.0).floor() / 20.0))
        .collect();
    let defects = found(1, &signal);
    assert!(defects.zipper.count >= 1, "{defects:?}");
    let run = &defects.zipper.worst[0];
    assert!(
        (run.period_seconds.expect("a period") - period).abs() < 0.0005,
        "{run:?}"
    );
    assert!((run.size_db - 1.0).abs() < 0.3, "{run:?}");
}

/// A note with a sharp pulse a cycle, as a voice or a reed has: the first
/// eight harmonics of `hz`, gained by `gain(t)` in dB.
fn buzz(hz: f64, seconds: f64, gain: impl Fn(f64) -> f64) -> Vec<f32> {
    let samples = (seconds * f64::from(RATE)) as usize;
    (0..samples)
        .map(|i| {
            let t = i as f64 / f64::from(RATE);
            let wave: f64 = (1..=8)
                .map(|n| (std::f64::consts::TAU * hz * f64::from(n) * t).cos() / 8.0)
                .sum();
            (0.6 * wave * 10f64.powf(gain(t) / 20.0)) as f32
        })
        .collect()
}

/// Two notes a few tens of hertz apart beat, and as they die away the beat
/// can read as a staircase at its own period: halved, each beat's halves are
/// level, but quartered they are not. A beat is not stepped gain, at any
/// decay.
#[test]
fn two_notes_beating_as_they_fade_are_not_a_staircase() {
    for (low, high) in [
        (220.0, 250.0),
        (330.0, 370.0),
        (440.0, 410.0),
        (440.0, 460.0),
        (440.0, 470.0),
        (440.0, 480.0),
        (440.0, 500.0),
        (523.0, 554.0),
    ] {
        for decay in [20.0, 30.0, 40.0, 60.0] {
            let gain = |t: f64| 10f64.powf(-decay * t / 20.0);
            let signal: Vec<f32> = tone(low, 0.3, 2.0, steady)
                .into_iter()
                .zip(tone(high, 0.3, 2.0, steady))
                .enumerate()
                .map(|(i, (a, b))| {
                    ((f64::from(a) + f64::from(b)) * gain(i as f64 / f64::from(RATE))) as f32
                })
                .collect();
            let defects = found(1, &signal);
            assert_eq!(
                defects.zipper.count, 0,
                "{low} and {high} Hz at -{decay} dB/s: {defects:?}"
            );
        }
    }
}

/// A level that drops at every frame and climbs back part of the way inside
/// it, in two quarter-frame steps, fades frame by frame but is not flat
/// within a frame. Halved at the frame it reads as a staircase; quartered,
/// or cut half a frame later, it does not, and it is not one.
#[test]
fn a_level_that_climbs_within_every_frame_is_not_a_staircase() {
    for (frame_ms, step_db) in [(12.0, 1.0), (12.0, 2.0), (16.6667, 0.5), (16.6667, 1.0)] {
        let frame = frame_ms / 1000.0;
        let signal = tone(440.0, 0.5, 1.5, |t| {
            let into = (t / frame).fract();
            let climb = if into < 0.25 {
                -step_db
            } else if into < 0.75 {
                0.0
            } else {
                step_db
            };
            -step_db * (t / frame).floor() + climb
        });
        let defects = found(1, &signal);
        assert_eq!(
            defects.zipper.count, 0,
            "{frame_ms} ms frames of {step_db} dB: {defects:?}"
        );
    }
}
