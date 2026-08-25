//! What an easing curve promises, at every progress value rather than at the
//! ends.

use proptest::prelude::*;
use scorsese_core::Easing;

use crate::runner::check;

/// This file, so a failure is written down beside it. See `runner`.
const SOURCE: &str = file!();

/// Every variant there is. A `static` rather than a `const` so the sampling
/// strategy can borrow it, and exhaustive on purpose: five values are cheaper
/// to check all of than to draw from.
pub(crate) static ALL: [Easing; 5] = [
    Easing::Linear,
    Easing::EaseIn,
    Easing::EaseOut,
    Easing::EaseInOut,
    Easing::Hold,
];

/// Draws one of them. Shared with the keyframe properties, which need an
/// easing on every point of a generated track.
pub(crate) fn any_easing() -> impl Strategy<Value = Easing> {
    prop::sample::select(ALL.as_slice())
}

/// Linear progress through a segment: what `value_at` computes and hands to
/// [`Easing::apply`].
fn progress() -> impl Strategy<Value = f64> {
    0.0f64..=1.0
}

#[test]
fn every_curve_but_hold_starts_at_nothing_and_arrives_at_everything() {
    // Exhaustive, so this is a stronger statement than a property: the
    // endpoints are shared by every easing that travels, and `Hold` is the
    // one that does not travel at all. Said out loud rather than papered
    // over — a `Hold` that answered 1.0 at the end would jump one frame
    // early, which is a different bug from an easing that overshoots.
    for easing in ALL {
        assert_eq!(easing.apply(0.0), 0.0, "{easing:?} leaves where it says");
        let arrival = if easing == Easing::Hold { 0.0 } else { 1.0 };
        assert_eq!(easing.apply(1.0), arrival, "{easing:?} arrives as it says");
    }
}

#[test]
fn no_curve_overshoots_its_own_endpoints() {
    // The property that catches an easing inventing a value nobody wrote,
    // one layer before it reaches a pixel: an opacity eased past 1.0 is a
    // frame brighter than the keyframe asked for.
    //
    // A future back-or-elastic curve deliberately trips this. When one lands
    // it gets an exception here with a reason written beside it, the same way
    // a mutant does — never a widened bound that also excuses an accident.
    check(SOURCE, (any_easing(), progress()), |(easing, p)| {
        let eased = easing.apply(p);
        prop_assert!(
            (0.0..=1.0).contains(&eased),
            "{easing:?} at {p} gave {eased}"
        );
        Ok(())
    });
}

#[test]
fn every_curve_only_ever_moves_forwards() {
    // Non-decreasing. A curve that dipped would run an animation backwards
    // for part of a segment, which is a thing an author can ask for with two
    // keyframes and must never get from one.
    let inputs = (any_easing(), progress(), progress());
    check(SOURCE, inputs, |(easing, a, b)| {
        let (early, late) = if a <= b { (a, b) } else { (b, a) };
        let (first, second) = (easing.apply(early), easing.apply(late));
        prop_assert!(
            first <= second,
            "{easing:?}: {early} gave {first}, {late} gave {second}"
        );
        Ok(())
    });
}

#[test]
fn ease_in_out_is_symmetric_about_the_middle() {
    // Slow at both ends and quickest in the middle means the curve is its own
    // reflection: what it has covered by `p` is exactly what it has left at
    // `1 - p`. There is an example at one value; it holds at all of them.
    //
    // Not an exact equality, because `1.0 - p` is itself a rounding for most
    // of the range. The tolerance is three orders of magnitude tighter than
    // an eighth of a frame at any framerate anyone renders at, so nothing it
    // admits is a thing a viewer could see.
    check(SOURCE, progress(), |p| {
        let (there, back) = (Easing::EaseInOut.apply(p), Easing::EaseInOut.apply(1.0 - p));
        prop_assert!(
            (there + back - 1.0).abs() < 1e-12,
            "at {p}: {there} and {back}"
        );
        Ok(())
    });
}
