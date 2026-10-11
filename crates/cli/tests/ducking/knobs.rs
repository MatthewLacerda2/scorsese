//! The knobs past depth and timing: the resting level, the pause that
//! breathes, and narration named by clip or by time. The fixture is two lines
//! at frames 60..150 and 400..460 over a 600-frame bed, at 30 fps.

use crate::common::{reload, run_in};
use crate::{dip_of, scored};

fn ducked(label: &str, knobs: &[&str]) -> Vec<(u64, f64)> {
    let dir = scored(label);
    let mut arguments = vec!["duck", "--music", "music"];
    arguments.extend_from_slice(knobs);
    run_in(&dir, &arguments).ok();
    dip_of(&reload(&dir))
}

#[test]
fn between_sets_the_level_outside_the_lines() {
    let points = ducked("duck-between", &["--between", "0.5"]);
    assert_eq!(points[0], (51, 0.5), "{points:?}");
    assert_eq!(points[7], (478, 0.5), "{points:?}");
}

#[test]
fn a_long_min_gap_holds_both_lines_under_as_one() {
    assert_eq!(
        ducked("duck-min-gap", &["--min-gap", "20"]),
        vec![(51, 1.0), (60, 0.25), (460, 0.25), (478, 1.0)]
    );
}

#[test]
fn a_named_clip_is_the_only_narration() {
    assert_eq!(
        ducked("duck-clip", &["--clip", "v2"]),
        vec![(391, 1.0), (400, 0.25), (460, 0.25), (478, 1.0)]
    );
}

#[test]
fn a_range_in_seconds_is_the_only_narration() {
    assert_eq!(
        ducked("duck-range", &["--range", "2:5"]),
        vec![(51, 1.0), (60, 0.25), (150, 0.25), (168, 1.0)]
    );
}

#[test]
fn a_backwards_range_and_an_unknown_clip_fail_and_say_why() {
    let dir = scored("duck-refused");
    let run = run_in(&dir, &["duck", "--music", "music", "--range", "5:2"]);
    assert!(run.failed);
    run.says("ends before it starts");

    let run = run_in(&dir, &["duck", "--music", "music", "--clip", "nope"]);
    assert!(run.failed);
    run.says("`nope`");
    assert!(dip_of(&reload(&dir)).is_empty(), "nothing was written");
}
