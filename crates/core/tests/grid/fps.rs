//! What a framerate is, and what it refuses to be.

use scorsese_core::{Fps, Frames};

#[test]
fn a_framerate_needs_both_parts() {
    assert!(Fps::new(0, 1).is_err(), "0fps is not a rate");
    assert!(Fps::new(30, 0).is_err(), "a zero denominator is not a rate");
    assert!(Fps::new(30, 1).is_ok());
}

#[test]
fn equal_rates_written_differently_are_one_value() {
    assert_eq!(Fps::new(60, 2).expect("valid"), Fps::THIRTY);
    assert_eq!(Fps::new(30000, 1001).expect("valid"), Fps::NTSC);
}

#[test]
fn ntsc_is_a_rational_and_stays_one() {
    // 29.97 is exactly 30000/1001. Frame 10,000 sits at 333.667s, and only
    // exact arithmetic gets there — a rounded 29.97 drifts by ~0.1s.
    assert_eq!((Fps::NTSC.num(), Fps::NTSC.den()), (30000, 1001));
    let at = Fps::NTSC.seconds(Frames(10_000));
    assert!((at - 333.666_666_7).abs() < 1e-6, "frame 10,000 at {at}s");
}

#[test]
fn frames_and_seconds_convert_both_ways() {
    assert_eq!(Fps::THIRTY.seconds(Frames(240)), 8.0);
    assert_eq!(Fps::THIRTY.frames(8.0), Frames(240));
    assert_eq!(Fps::FILM.frames(2.0), Frames(48));
}

#[test]
fn a_time_before_the_timeline_lands_on_frame_zero() {
    assert_eq!(Fps::THIRTY.frames(-1.0), Frames::ZERO);
    assert_eq!(Fps::THIRTY.frames(f64::NAN), Frames::ZERO);
}

#[test]
fn conforming_picks_the_nearest_source_frame() {
    // 24 → 30: a second of film is a second of timeline, and the frames in
    // between land on their nearest neighbour rather than being invented.
    assert_eq!(Fps::THIRTY.conform(Frames(24), Fps::FILM), Frames(30));
    assert_eq!(Fps::THIRTY.conform(Frames(1), Fps::FILM), Frames(1));
    assert_eq!(Fps::THIRTY.conform(Frames(3), Fps::FILM), Frames(4));
    assert_eq!(Fps::FILM.conform(Frames(30), Fps::THIRTY), Frames(24));
}

#[test]
fn conforming_onto_the_same_grid_changes_nothing() {
    for rate in [Fps::FILM, Fps::PAL, Fps::THIRTY, Fps::NTSC, Fps::SIXTY] {
        assert_eq!(rate.conform(Frames(12_345), rate), Frames(12_345), "{rate}");
    }
}

#[test]
fn conforming_ntsc_does_not_drift_over_a_long_timeline() {
    // An hour of 29.97 conformed to 30 and back is the same frame, which is
    // the property float seconds cannot offer.
    let hour = Frames(107_892);
    let there = Fps::THIRTY.conform(hour, Fps::NTSC);
    assert_eq!(Fps::NTSC.conform(there, Fps::THIRTY), hour);
}

#[test]
fn a_framerate_reads_and_writes_as_text() {
    assert_eq!(Fps::THIRTY.to_string(), "30");
    assert_eq!(Fps::NTSC.to_string(), "30000/1001");
    assert_eq!("30".parse(), Ok(Fps::THIRTY));
    assert_eq!("30000/1001".parse(), Ok(Fps::NTSC));
}

#[test]
fn a_decimal_framerate_is_refused() {
    // Accepting "29.97" would put the rounding the rational type exists to
    // prevent right back at the command line.
    assert!("29.97".parse::<Fps>().is_err());
    assert!("thirty".parse::<Fps>().is_err());
    assert!("0".parse::<Fps>().is_err());
}
