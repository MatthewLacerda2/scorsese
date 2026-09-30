//! What an easing curve promises, at every progress value rather than at the
//! ends.
//!
//! Two families, held to different promises. The **tame** curves — linear,
//! the quadratic family, `hold`, and a cubic bezier whose handles stay inside
//! the unit square — never leave their endpoints and never move backwards. The
//! **overshooting** ones — the back family, `spring`, and a bezier with a `y`
//! outside `0..=1` — do both on purpose, which is what they are for, so what
//! they are held to instead is arriving exactly and not going wild on the way.

use proptest::prelude::*;
use scorsese_core::Easing;

use crate::runner::check;

/// This file, so a failure is written down beside it. See `runner`.
const SOURCE: &str = file!();

/// Every preset, which is every variant but the one that carries numbers.
/// A `static` so the sampling strategy can borrow it, and exhaustive on
/// purpose: nine values are cheaper to check all of than to draw from.
pub(crate) static PRESETS: [Easing; 9] = [
    Easing::Linear,
    Easing::EaseIn,
    Easing::EaseOut,
    Easing::EaseInOut,
    Easing::Hold,
    Easing::BackIn,
    Easing::BackOut,
    Easing::BackInOut,
    Easing::Spring,
];

/// The presets that stay inside their endpoints.
static TAME: [Easing; 5] = [
    Easing::Linear,
    Easing::EaseIn,
    Easing::EaseOut,
    Easing::EaseInOut,
    Easing::Hold,
];

/// A well-formed cubic bezier with its `y` handles drawn from `y`.
fn bezier(y: std::ops::RangeInclusive<f64>) -> impl Strategy<Value = Easing> {
    (0.0f64..=1.0, y.clone(), 0.0f64..=1.0, y)
        .prop_map(|(x1, y1, x2, y2)| Easing::CubicBezier([x1, y1, x2, y2]))
}

/// Draws any easing a valid document can hold. Shared with the keyframe
/// properties, which need an easing on every point of a generated track.
pub(crate) fn any_easing() -> impl Strategy<Value = Easing> {
    prop_oneof![3 => prop::sample::select(PRESETS.as_slice()), 1 => bezier(-1.0..=2.0)]
}

/// Draws an easing that never leaves its endpoints: a tame preset, or a bezier
/// whose handles sit inside the unit square — which the convex hull of its
/// control points keeps inside `0..=1`.
pub(crate) fn tame_easing() -> impl Strategy<Value = Easing> {
    prop_oneof![prop::sample::select(TAME.as_slice()), bezier(0.0..=1.0)]
}

/// Linear progress through a segment: what `value_at` computes and hands to
/// [`Easing::apply`].
fn progress() -> impl Strategy<Value = f64> {
    0.0f64..=1.0
}

#[test]
fn every_curve_but_hold_starts_at_nothing_and_arrives_at_everything() {
    // Exact, and for the overshooting curves too: however far one swings on
    // the way, a keyframe's own value has to read back as written. `Hold` is
    // the one that does not travel — a `Hold` that answered 1.0 at the end
    // would jump one frame early.
    check(SOURCE, any_easing(), |easing| {
        prop_assert_eq!(easing.apply(0.0), 0.0, "{:?} leaves where it says", easing);
        let arrival = if easing == Easing::Hold { 0.0 } else { 1.0 };
        prop_assert_eq!(
            easing.apply(1.0),
            arrival,
            "{:?} arrives as it says",
            easing
        );
        Ok(())
    });
}

#[test]
fn a_tame_curve_never_overshoots_its_own_endpoints() {
    // The property that catches an easing inventing a value nobody wrote,
    // one layer before it reaches a pixel. The overshooting curves are the
    // stated exception, held by the next property instead — never a widened
    // bound here that would also excuse an accident.
    check(SOURCE, (tame_easing(), progress()), |(easing, p)| {
        let eased = easing.apply(p);
        prop_assert!(
            (0.0..=1.0).contains(&eased),
            "{easing:?} at {p} gave {eased}"
        );
        Ok(())
    });
}

#[test]
fn a_preset_overshoots_by_a_fifth_of_the_move_at_most() {
    // The presets pass their marks by a tenth (back) and a sixth (spring).
    // Nothing a preset does should go further than that: past a fifth is a
    // curve that has stopped being a settle and become a different motion.
    let presets = prop::sample::select(PRESETS.as_slice());
    check(SOURCE, (presets, progress()), |(easing, p)| {
        let eased = easing.apply(p);
        prop_assert!(
            (-0.2..=1.2).contains(&eased),
            "{easing:?} at {p} gave {eased}"
        );
        Ok(())
    });
}

#[test]
fn a_tame_curve_only_ever_moves_forwards() {
    // Non-decreasing. A curve that dipped would run an animation backwards
    // for part of a segment, which is a thing an author can ask for with two
    // keyframes and must never get from one — unless they asked for a curve
    // whose whole shape is going past the mark and coming back.
    let inputs = (tame_easing(), progress(), progress());
    check(SOURCE, inputs, |(easing, a, b)| {
        let (early, late) = if a <= b { (a, b) } else { (b, a) };
        let (first, second) = (easing.apply(early), easing.apply(late));
        // A hair of solver tolerance on a bezier; exact for the rest.
        prop_assert!(
            first <= second + 1e-9,
            "{easing:?}: {early} gave {first}, {late} gave {second}"
        );
        Ok(())
    });
}

#[test]
fn a_cubic_bezier_reads_its_own_curve_back() {
    // Pick a point on the curve by its parameter, ask the easing for its x,
    // and it must answer that point's y. This is the solver, checked against
    // the definition rather than against itself. The ends are kept clear of,
    // where a handle on the corner flattens x to nothing and the answer is
    // rightly fuzzy over a sliver of the segment no frame falls in.
    let points = (0.0f64..=1.0, -1.0f64..=2.0, 0.0f64..=1.0, -1.0f64..=2.0);
    check(SOURCE, (points, 0.02f64..=0.98), |((x1, y1, x2, y2), s)| {
        let bernstein = |p1: f64, p2: f64| {
            3.0 * (1.0 - s) * (1.0 - s) * s * p1 + 3.0 * (1.0 - s) * s * s * p2 + s * s * s
        };
        let eased = Easing::CubicBezier([x1, y1, x2, y2]).apply(bernstein(x1, x2));
        let expected = bernstein(y1, y2);
        prop_assert!(
            (eased - expected).abs() < 1e-4,
            "at s={s}: {eased}, the curve says {expected}"
        );
        Ok(())
    });
}

#[test]
fn ease_in_out_is_symmetric_about_the_middle() {
    // Slow at both ends and quickest in the middle means the curve is its own
    // reflection: what it has covered by `p` is exactly what it has left at
    // `1 - p`. There is an example at one value; it holds at all of them, and
    // for the back curve that winds up and settles in the same measure.
    //
    // Not an exact equality, because `1.0 - p` is itself a rounding for most
    // of the range. The tolerance is three orders of magnitude tighter than
    // an eighth of a frame at any framerate anyone renders at, so nothing it
    // admits is a thing a viewer could see.
    check(SOURCE, progress(), |p| {
        for easing in [Easing::EaseInOut, Easing::BackInOut] {
            let (there, back) = (easing.apply(p), easing.apply(1.0 - p));
            prop_assert!(
                (there + back - 1.0).abs() < 1e-12,
                "{easing:?} at {p}: {there} and {back}"
            );
        }
        Ok(())
    });
}
