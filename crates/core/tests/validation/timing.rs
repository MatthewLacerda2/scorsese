//! Overlap: when two clips fight over the same frame of a track.
//!
//! Negative and fractional times are absent on purpose — they cannot be
//! written down as [`scorsese_core::Frames`], so they fail the load instead
//! of reaching validation. `tests/grid/document.rs` covers that.

use crate::common::{assert_only_problem, clip_id, project, track_id};
use scorsese_core::{Frames, ValidationError as E};

fn overlap_on_v1() -> E {
    E::OverlappingClips {
        track: track_id("v1"),
        first: clip_id("c-shot"),
        second: clip_id("c-title"),
    }
}

#[test]
fn touching_clips_are_fine() {
    // c-shot ends at frame 240 exactly where c-title starts.
    assert_eq!(project().validate(), Ok(()));
}

#[test]
fn overlapping_clips_are_refused() {
    let mut p = project();
    p.tracks[0].clips[1].start = Frames(225);
    assert_only_problem(&p, &overlap_on_v1());
}

#[test]
fn overlap_is_found_however_the_clips_are_ordered_in_the_file() {
    let mut p = project();
    p.tracks[0].clips.swap(0, 1);
    p.tracks[0].clips[0].start = Frames(225);
    assert_only_problem(&p, &overlap_on_v1());
}

#[test]
fn a_clip_fully_inside_another_is_an_overlap() {
    let mut p = project();
    p.tracks[0].clips[1].start = Frames(60);
    p.tracks[0].clips[1].duration = Frames(30);
    assert_only_problem(&p, &overlap_on_v1());
}

#[test]
fn clips_on_different_tracks_may_share_the_same_frame() {
    // c-shot on v1 and c-logo on v2 both cover frame 210; that is
    // compositing, not a conflict.
    assert_eq!(project().validate(), Ok(()));
}

#[test]
fn a_clip_that_renders_nothing_is_reported() {
    let mut p = project();
    p.tracks[0].clips[0].duration = Frames::ZERO;
    assert_only_problem(
        &p,
        &E::ZeroDuration {
            clip: clip_id("c-shot"),
        },
    );
}
