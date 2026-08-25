//! The rational framerate, over every rate it can be built from.
//!
//! A wrong answer here does not panic and does not produce a broken file. It
//! produces a delivered video that is subtly out of sync — which nobody
//! watches on an agent-driven pull request.

use proptest::prelude::*;
use scorsese_core::{Fps, Frames};

use crate::runner::check;

/// This file, so a failure is written down beside it. See `runner`.
const SOURCE: &str = file!();

/// Any rate that constructs, weighted towards the ones a timeline is really
/// authored on.
///
/// Three parts in four come from the plausible range — up to 240000/1001,
/// which covers everything from silent film to a high-speed camera pulled
/// down for NTSC. The fourth part is anything at all a `u32` pair can say,
/// because the arithmetic is not entitled to assume a sensible rate: nothing
/// stops a `project.json` carrying `{ "num": 4000000000, "den": 7 }`, and
/// what that must not do is give a wrong answer quietly.
fn rate() -> impl Strategy<Value = Fps> {
    prop_oneof![
        3 => (1u32..=240_000, 1u32..=1_001u32),
        1 => (1u32..=u32::MAX, 1u32..=u32::MAX),
    ]
    .prop_map(|(num, den)| Fps::new(num, den).expect("both parts are non-zero"))
}

/// A frame count a timeline can hold: up to a hundred million, which is over
/// a month of footage at 30.
///
/// Bounded on purpose, and the bound is a claim about `f64` rather than about
/// the arithmetic. Seconds are a float — they have to be, ffmpeg takes one —
/// and a float carries about sixteen digits. Round-tripping a frame count
/// near `u64::MAX` through one is not a thing this type offers, so a property
/// asserting it would be asserting a promise nobody made.
fn frame_count() -> impl Strategy<Value = Frames> {
    (0u64..=100_000_000).prop_map(Frames)
}

#[test]
fn frames_and_seconds_round_trip() {
    // The conversion at the edge of the model: a frame goes out to a decoder
    // as seconds and comes back as a frame. If it comes back as a different
    // one, a clip is trimmed a frame from where the edit says.
    check(SOURCE, (rate(), frame_count()), |(fps, f)| {
        prop_assert_eq!(fps.frames(fps.seconds(f)), f, "at {}", fps);
        Ok(())
    });
}

#[test]
fn conforming_onto_the_same_grid_is_the_identity() {
    // There is an example of this at five named rates. It holds at all of
    // them: re-timing from a grid onto itself is not an operation.
    check(SOURCE, (rate(), frame_count()), |(fps, f)| {
        prop_assert_eq!(fps.conform(f, fps), f, "at {}", fps);
        Ok(())
    });
}

#[test]
fn conforming_preserves_the_order_of_two_frames() {
    // The one that would be a cut rewriting itself: if an earlier frame ever
    // conformed later than a later one, two clips have swapped places on a
    // rate change.
    let inputs = (rate(), rate(), frame_count(), frame_count());
    check(SOURCE, inputs, |(from, onto, a, b)| {
        let (first, second) = if a <= b { (a, b) } else { (b, a) };
        let (there, later) = (onto.conform(first, from), onto.conform(second, from));
        prop_assert!(there <= later, "{from} to {onto}: {first} and {second}");
        Ok(())
    });
}

#[test]
fn seconds_ascend_and_never_go_negative() {
    // Frame 0 is the start of the timeline and nothing sits before it.
    check(
        SOURCE,
        (rate(), frame_count(), frame_count()),
        |(fps, a, b)| {
            let (first, second) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(fps.seconds(first) >= 0.0, "{first} at {fps}");
            prop_assert!(fps.seconds(first) <= fps.seconds(second), "at {}", fps);
            Ok(())
        },
    );
}

#[test]
fn a_rate_and_its_multiples_are_one_value() {
    // 30/1, 60/2 and 90/3 are the same framerate. Stored in lowest terms they
    // are the same *value*, so two projects that wrote it differently compare
    // equal instead of conforming to each other for no reason.
    let inputs = (1u32..=200_000u32, 1u32..=200_000u32, 1u32..=20_000u32).prop_filter(
        "the multiplied rate has to fit in a u32",
        |&(num, den, by)| num.checked_mul(by).is_some() && den.checked_mul(by).is_some(),
    );
    check(SOURCE, inputs, |(num, den, by)| {
        let plain = Fps::new(num, den).expect("both parts are non-zero");
        let scaled = Fps::new(num * by, den * by).expect("both parts are non-zero");
        prop_assert_eq!(plain, scaled);
        prop_assert_eq!(plain.num(), scaled.num());
        prop_assert_eq!(plain.den(), scaled.den());
        Ok(())
    });
}

#[test]
fn a_rate_reads_back_from_the_text_it_writes() {
    // `Display` is what a rate looks like in a CLI argument and in a report;
    // `FromStr` is how it gets back. A rate that does not survive the pair is
    // one a project can be saved with and not re-opened at.
    check(SOURCE, rate(), |fps| {
        prop_assert_eq!(fps.to_string().parse::<Fps>(), Ok(fps));
        Ok(())
    });
}
