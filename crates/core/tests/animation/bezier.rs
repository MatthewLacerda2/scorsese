//! `cubic_bezier`: four numbers, read exactly as CSS reads them.
//!
//! The expected values are CSS's own curves at known points — computed by
//! bisecting the Bernstein form to far past `f64` precision — so anyone who
//! copied four numbers out of a stylesheet gets the motion they saw there.

use scorsese_core::Easing;

use crate::{at, track};

/// Close enough that no frame at any rate anyone renders could tell.
const WITHIN: f64 = 1e-7;

fn curve(points: [f64; 4]) -> Easing {
    Easing::CubicBezier(points)
}

fn assert_reads(points: [f64; 4], samples: &[(f64, f64)]) {
    for &(x, expected) in samples {
        let got = curve(points).apply(x);
        assert!(
            (got - expected).abs() < WITHIN,
            "{points:?} at {x}: {got}, CSS says {expected}"
        );
    }
}

#[test]
fn css_ease_is_css_ease() {
    let ease = [0.25, 0.1, 0.25, 1.0];
    let samples = [
        (0.1, 0.094_796_305_7),
        (0.25, 0.408_510_591_4),
        (0.5, 0.802_403_387_6),
    ];
    assert_reads(ease, &samples);
    assert_reads(ease, &[(0.75, 0.960_458_978_3), (0.9, 0.994_316_477_5)]);
}

#[test]
fn css_ease_in_and_ease_out_are_mirror_images() {
    let ease_in = [
        (0.1, 0.017_026_609_7),
        (0.5, 0.315_356_812_6),
        (0.9, 0.839_427_845_8),
    ];
    assert_reads([0.42, 0.0, 1.0, 1.0], &ease_in);
    let ease_out = [
        (0.1, 0.160_572_154_2),
        (0.5, 0.684_643_187_4),
        (0.9, 0.982_973_390_3),
    ];
    assert_reads([0.0, 0.0, 0.58, 1.0], &ease_out);
}

#[test]
fn a_y_past_one_overshoots_and_is_not_clamped() {
    // The usual CSS stand-in for back-out: past the mark by about 9%.
    let pop = [0.34, 1.56, 0.64, 1.0];
    assert_reads(pop, &[(0.25, 0.816_289_198_7), (0.5, 1.087_400_670_2)]);
    assert_reads(pop, &[(0.75, 1.059_646_860_0)]);
}

#[test]
fn the_ends_are_exact_whatever_the_control_points() {
    for points in [
        [0.25, 0.1, 0.25, 1.0],
        [0.34, 1.56, 0.64, 1.0],
        [0.9, -0.7, 0.1, 1.8],
    ] {
        assert_eq!(curve(points).apply(0.0), 0.0, "{points:?}");
        assert_eq!(curve(points).apply(1.0), 1.0, "{points:?}");
    }
    let ramp = track(&[
        (0, 3.0, curve([0.3, 2.0, 0.7, -1.0])),
        (30, 7.0, Easing::Linear),
    ]);
    assert_eq!(at(&ramp, 0), 3.0);
    assert_eq!(at(&ramp, 30), 7.0);
}

#[test]
fn a_curve_with_its_handles_on_the_line_is_linear() {
    for x in [0.1, 0.33, 0.5, 0.9] {
        let got = curve([0.2, 0.2, 0.8, 0.8]).apply(x);
        assert!((got - x).abs() < WITHIN, "{x} read {got}");
    }
}

#[test]
fn a_document_writes_the_four_numbers_under_one_key() {
    let written = serde_json::to_value(curve([0.25, 0.1, 0.25, 1.0])).unwrap();
    assert_eq!(
        written,
        serde_json::json!({ "cubic_bezier": [0.25, 0.1, 0.25, 1.0] })
    );
    // Integers read as numbers, as they do everywhere else in a document.
    let read: Easing = serde_json::from_str(r#"{ "cubic_bezier": [0, 0, 1, 1] }"#).unwrap();
    assert_eq!(read, curve([0.0, 0.0, 1.0, 1.0]));
    // And every preset stays a bare word.
    assert_eq!(serde_json::to_value(Easing::BackOut).unwrap(), "back_out");
    assert_eq!(serde_json::to_value(Easing::Spring).unwrap(), "spring");
}

#[test]
fn a_curve_that_runs_time_backwards_still_answers() {
    // Validation refuses it; the evaluator may be handed one anyway and must
    // neither loop forever nor panic.
    for points in [
        [-2.0, 0.0, 3.0, 1.0],
        [1.5, 1.0, -0.5, 0.0],
        [f64::NAN, 0.0, 1.0, 1.0],
    ] {
        let _ = curve(points).apply(0.5);
    }
}
