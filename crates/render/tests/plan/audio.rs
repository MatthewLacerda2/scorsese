//! Sequencing the audible half of a timeline.

use scorsese_core::{AssetKind, Fps, Frames};
use scorsese_render::report::Note;
use scorsese_render::{FrameRange, Plan};

use crate::common::{audio_track, clip, file_asset, project, video_track};

/// What the mixer works at, and the only rate these counts mean anything in.
const RATE: u32 = 48_000;

#[test]
fn a_project_with_no_audio_gets_no_soundtrack_at_all() {
    // Not one of silence: a video with an empty audio stream and a video with
    // none are different files, and the second is what an edit with no sound in
    // it means.
    let project = project(
        vec![file_asset("a", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "a", 0, 30)])],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    assert!(plan.audio().is_empty());
}

#[test]
fn audio_running_past_the_last_picture_is_reported_rather_than_cut_in_silence() {
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, 30)]),
            audio_track("a1", vec![clip("m1", "music", 0, 90)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    assert_eq!(
        plan.notes(),
        [Note::AudioTrimmed {
            audio_end: Frames(90),
            timeline_end: Frames(30),
        }]
    );
}

#[test]
fn audio_ending_on_the_last_picture_frame_is_not_trimmed() {
    // The boundary itself. A bed cut to the frame ends *at* frame 90, which is
    // the frame after the last one — nothing was lost, so there is nothing to
    // report, and a comparison one frame loose here would call every
    // well-finished edit trimmed.
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, 90)]),
            audio_track("a1", vec![clip("m1", "music", 0, 90)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(plan.notes(), [], "nothing was cut");
    assert_eq!(plan.end(), Frames(90));
}

#[test]
fn an_audio_track_is_cut_where_its_own_clips_change_and_not_where_the_picture_does() {
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            // Three shots over one unbroken music bed.
            video_track(
                "v1",
                vec![
                    clip("c1", "a", 0, 30),
                    clip("c2", "a", 30, 30),
                    clip("c3", "a", 60, 30),
                ],
            ),
            audio_track("a1", vec![clip("m1", "music", 0, 90)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(plan.segments().len(), 3, "three shots");
    assert_eq!(
        plan.audio().len(),
        1,
        "one bed, decoded once — a picture cut is no reason to restart the music"
    );
}

#[test]
fn sample_counts_always_sum_to_the_length_of_the_mix() {
    // Conforming each segment's duration on its own would round independently
    // and drift; conforming its boundaries cannot. A frame of 29.97 is 1601.6
    // samples, so single-frame clips are where the two answers part company —
    // and audio that drifts against picture is the one error nobody forgives.
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, 3)]),
            audio_track(
                "a1",
                vec![
                    clip("m1", "music", 0, 1),
                    clip("m2", "music", 1, 1),
                    clip("m3", "music", 2, 1),
                ],
            ),
        ],
    );
    let project = scorsese_core::Project {
        timeline_fps: Fps::NTSC,
        ..project
    };
    let plan = Plan::build(&project, Fps::NTSC, FrameRange::ALL).expect("plan");

    let summed: u64 = plan
        .audio()
        .iter()
        .map(|segment| plan.samples_of(segment, RATE))
        .sum();
    assert_eq!(summed, plan.total_samples(RATE));
}
