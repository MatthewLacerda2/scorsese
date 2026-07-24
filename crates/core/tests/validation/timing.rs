//! Time and overlap: when a clip sits somewhere impossible.

use crate::common::{assert_only_problem, clip_id, project, track_id};
use scorsese_core::{Seconds, ValidationError as E};

fn overlap_on_v1() -> E {
    E::OverlappingClips {
        track: track_id("v1"),
        first: clip_id("c-shot"),
        second: clip_id("c-title"),
    }
}

#[test]
fn touching_clips_are_fine() {
    // c-shot ends at 8.0 exactly where c-title starts.
    assert_eq!(project().validate(), Ok(()));
}

#[test]
fn overlapping_clips_are_refused() {
    let mut p = project();
    p.tracks[0].clips[1].start = Seconds(7.5);
    assert_only_problem(&p, &overlap_on_v1());
}

#[test]
fn overlap_is_found_however_the_clips_are_ordered_in_the_file() {
    let mut p = project();
    p.tracks[0].clips.swap(0, 1);
    p.tracks[0].clips[0].start = Seconds(7.5);
    assert_only_problem(&p, &overlap_on_v1());
}

#[test]
fn a_clip_fully_inside_another_is_an_overlap() {
    let mut p = project();
    p.tracks[0].clips[1].start = Seconds(2.0);
    p.tracks[0].clips[1].duration = Seconds(1.0);
    assert_only_problem(&p, &overlap_on_v1());
}

#[test]
fn clips_on_different_tracks_may_share_the_same_instant() {
    // c-shot on v1 and c-logo on v2 both cover t=7; that is compositing, not
    // a conflict.
    assert_eq!(project().validate(), Ok(()));
}

#[test]
fn a_negative_start_is_not_a_time() {
    let mut p = project();
    p.tracks[0].clips[0].start = Seconds(-1.0);
    assert_only_problem(
        &p,
        &E::BadTime {
            clip: clip_id("c-shot"),
            field: "start",
            value: -1.0,
        },
    );
}

#[test]
fn an_infinite_duration_is_not_a_time() {
    let mut p = project();
    p.tracks[0].clips[0].duration = Seconds(f64::INFINITY);
    assert_only_problem(
        &p,
        &E::BadTime {
            clip: clip_id("c-shot"),
            field: "duration",
            value: f64::INFINITY,
        },
    );
}

#[test]
fn a_clip_that_renders_nothing_is_reported() {
    let mut p = project();
    p.tracks[0].clips[0].duration = Seconds::ZERO;
    assert_only_problem(
        &p,
        &E::ZeroDuration {
            clip: clip_id("c-shot"),
        },
    );
}
