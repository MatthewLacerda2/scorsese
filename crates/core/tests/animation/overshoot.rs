//! The curves that go past their keyframes on purpose.
//!
//! An overshoot is *made of* a value nobody wrote — a title scaled to 1.1 on
//! its way to 1.0 — so these are the tests that hold the evaluator to **not**
//! clamping it. Clamping belongs to the properties that cannot go past their
//! ends, and those clamp where the number's meaning is known.

use scorsese_core::Easing;

use crate::{at, track};

/// A 100-frame move from 0 to 1, leaving on `easing`.
fn move_on(easing: Easing) -> scorsese_core::KeyframeTrack {
    track(&[(0, 0.0, easing), (100, 1.0, Easing::Linear)])
}

/// The furthest the move gets above its target and below its start.
fn extremes(easing: Easing) -> (f64, f64) {
    let ramp = move_on(easing);
    (0..=100)
        .map(|t| at(&ramp, t))
        .fold((0.0, 1.0), |(high, low), v| {
            (f64::max(high, v), f64::min(low, v))
        })
}

#[test]
fn back_out_passes_the_mark_by_a_tenth_and_settles_on_it() {
    let (high, low) = extremes(Easing::BackOut);
    assert!((high - 1.1).abs() < 1e-3, "peaks at {high}");
    assert_eq!(low, 0.0, "and never goes back behind the start");
    assert_eq!(at(&move_on(Easing::BackOut), 100), 1.0, "lands exactly");
}

#[test]
fn back_in_winds_up_the_other_way_first() {
    let (high, low) = extremes(Easing::BackIn);
    assert!((low + 0.1).abs() < 1e-3, "dips to {low}");
    assert_eq!(high, 1.0, "and never passes the target");
}

#[test]
fn back_in_out_does_both_and_is_symmetric() {
    let (high, low) = extremes(Easing::BackInOut);
    assert!(
        (high - 1.1).abs() < 1e-3 && (low + 0.1).abs() < 1e-3,
        "{low}..{high}"
    );
    let ramp = move_on(Easing::BackInOut);
    assert_eq!(at(&ramp, 50), 0.5);
    assert!((at(&ramp, 20) + at(&ramp, 80) - 1.0).abs() < 1e-12);
}

#[test]
fn spring_overshoots_swings_back_under_and_comes_to_rest() {
    let ramp = move_on(Easing::Spring);
    let (high, low) = extremes(Easing::Spring);
    assert!(
        (1.14..1.17).contains(&high),
        "the first swing peaks at {high}"
    );
    assert_eq!(low, 0.0, "it never goes behind the start");
    // After the first peak it has to come back *under* the target, or it is
    // a back curve with a different name.
    assert!(
        (40..80).any(|t| at(&ramp, t) < 0.99),
        "and swings back under"
    );
    assert!(
        (at(&ramp, 95) - 1.0).abs() < 1e-3,
        "near still before the end"
    );
    assert_eq!(at(&ramp, 100), 1.0);
}

#[test]
fn the_overshoot_scales_with_the_move_and_follows_its_direction() {
    // A fall from 2 to 1 passes 1 going *down*: the curve is a shape over
    // progress, and the keyframes decide which way is past.
    let fall = track(&[(0, 2.0, Easing::BackOut), (100, 1.0, Easing::Linear)]);
    let lowest = (0..=100).map(|t| at(&fall, t)).fold(f64::MAX, f64::min);
    assert!((lowest - 0.9).abs() < 1e-3, "went down to {lowest}");
}

#[test]
fn a_move_that_does_not_move_never_overshoots() {
    // Nothing to be a tenth of: equal keyframes hold, whatever the curve.
    let still = track(&[(0, 0.5, Easing::Spring), (100, 0.5, Easing::Linear)]);
    assert!((0..=100).all(|t| at(&still, t) == 0.5));
}
