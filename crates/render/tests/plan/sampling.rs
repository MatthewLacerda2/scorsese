//! Turning timeline frames into sample positions, and back into instants.
//!
//! The counts here are asserted as *numbers*. The existing audio tests check
//! that the segments' sample counts sum to the render's total, which is the
//! right invariant and is satisfied by any arithmetic that is merely
//! self-consistent — both sides of that comparison go through the same
//! conversion, so multiplying where it should divide keeps it true.
//!
//! A fractional frame rate is what makes the rounding visible: at 30000/1001
//! one frame is 1601.6 samples, so truncating and rounding disagree, and an
//! error of a fraction of a frame per segment is exactly the drift these
//! functions are written to avoid.

use scorsese_core::{AssetKind, Fps, Frames};
use scorsese_render::{FrameRange, Plan};

use crate::common::{audio_track, clip, file_asset, project, video_track};

/// What the mixer works at, and the only rate these counts mean anything in.
const RATE: u32 = 48_000;

/// One audio clip `frames` long over picture of the same length, at NTSC.
///
/// NTSC is 30000/1001, so nothing here divides evenly and the rounding in
/// `sample_index` has something to do.
fn ntsc_of(frames: u64) -> scorsese_core::Project {
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, frames)]),
            audio_track("a1", vec![clip("m1", "music", 0, frames)]),
        ],
    );
    scorsese_core::Project {
        timeline_fps: Fps::NTSC,
        ..project
    }
}

#[test]
fn one_ntsc_frame_of_audio_is_rounded_rather_than_truncated() {
    // 1 * 1001 * 48000 / 30000 = 1601.6 samples. Rounding is the whole reason
    // the conversion carries a half-denominator: truncating loses two thirds of
    // a sample here, and a segment boundary that drops a fraction every time is
    // how a mix walks away from the picture over a long edit.
    let project = ntsc_of(1);
    let plan = Plan::build(&project, Fps::NTSC, FrameRange::ALL).expect("plan");
    let segment = &plan.audio()[0];

    assert_eq!(plan.samples_of(segment, RATE), 1602);
    assert_eq!(plan.total_samples(RATE), 1602);
}

#[test]
fn thirty_ntsc_frames_of_audio_is_a_whole_number_of_samples() {
    // 30 * 1001 * 48000 / 30000 = 48048 exactly — no rounding involved, so this
    // one pins the multiplication and division on their own, with the half
    // denominator contributing nothing.
    let project = ntsc_of(30);
    let plan = Plan::build(&project, Fps::NTSC, FrameRange::ALL).expect("plan");
    let segment = &plan.audio()[0];

    assert_eq!(plan.samples_of(segment, RATE), 48_048);
}

#[test]
fn a_segments_samples_are_counted_from_where_that_segment_starts() {
    // Two clips, so the second segment does not begin at zero. Its count is its
    // own length and not its distance from the start of the render — the
    // subtraction in `samples_of` is what makes that true.
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, 60)]),
            audio_track(
                "a1",
                vec![clip("m1", "music", 0, 30), clip("m2", "music", 30, 30)],
            ),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    let [first, second] = plan.audio() else {
        panic!("expected the two clips to make two segments");
    };
    assert_eq!(plan.samples_of(first, RATE), 48_000);
    assert_eq!(plan.samples_of(second, RATE), 48_000);
    assert_eq!(plan.total_samples(RATE), 96_000);
}

#[test]
fn the_instant_a_frame_shows_is_offset_from_its_own_segment() {
    // `timeline_frame_of` answers "which instant is this output frame drawing?"
    // and the compositor times keyframes against the answer. Counted from the
    // segment's start: the fifth frame of a segment beginning at 30 is frame 35
    // of the timeline, not frame 5 of it.
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, 60)]),
            audio_track(
                "a1",
                vec![clip("m1", "music", 0, 30), clip("m2", "music", 30, 30)],
            ),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    let [first, second] = plan.audio() else {
        panic!("expected the two clips to make two segments");
    };
    assert_eq!(plan.timeline_frame_of(first, 0), Frames(0));
    assert_eq!(plan.timeline_frame_of(first, 5), Frames(5));
    assert_eq!(plan.timeline_frame_of(second, 0), Frames(30));
    assert_eq!(plan.timeline_frame_of(second, 5), Frames(35));
}
