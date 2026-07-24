//! What occupies each stretch of the timeline, and how many frames it is worth.

use scorsese_core::{AssetKind, Fps};
use scorsese_render::{FrameRange, Plan};

use crate::common::{clip, file_asset, project, shape, video_track};

fn two_clip_project() -> scorsese_core::Project {
    project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("b", AssetKind::Video),
        ],
        vec![video_track(
            "v1",
            vec![clip("c1", "a", 0, 60), clip("c2", "b", 90, 30)],
        )],
    )
}

#[test]
fn clips_are_sequenced_with_black_in_the_holes() {
    let project = two_clip_project();
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![
            (0, 60, "c1".to_owned(), 60),
            (60, 30, "gap".to_owned(), 30),
            (90, 30, "c2".to_owned(), 30),
        ],
        "the hole between the clips is thirty frames of black, not missing time"
    );
    assert_eq!(plan.out_frames(), 120);
}

#[test]
fn a_timeline_that_starts_late_opens_on_black() {
    let project = project(
        vec![file_asset("a", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "a", 15, 30)])],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![(0, 15, "gap".to_owned(), 15), (15, 30, "c1".to_owned(), 30)]
    );
}

#[test]
fn clips_out_of_order_in_the_document_are_still_sequenced_in_time() {
    // Nothing requires a project file to list clips in order, and an agent
    // appending to the array is the normal case.
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("b", AssetKind::Video),
        ],
        vec![video_track(
            "v1",
            vec![clip("late", "b", 30, 30), clip("early", "a", 0, 30)],
        )],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    let order: Vec<String> = shape(&plan).into_iter().map(|entry| entry.2).collect();
    assert_eq!(order, vec!["early".to_owned(), "late".to_owned()]);
}

#[test]
fn touching_clips_leave_no_gap_between_them() {
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("b", AssetKind::Video),
        ],
        vec![video_track(
            "v1",
            vec![clip("c1", "a", 0, 30), clip("c2", "b", 30, 30)],
        )],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![(0, 30, "c1".to_owned(), 30), (30, 30, "c2".to_owned(), 30)]
    );
}

#[test]
fn rendering_at_another_rate_conforms_from_the_timeline_grid() {
    let project = two_clip_project();
    let plan = Plan::build(&project, Fps::SIXTY, FrameRange::ALL).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![
            (0, 60, "c1".to_owned(), 120),
            (60, 30, "gap".to_owned(), 60),
            (90, 30, "c2".to_owned(), 60),
        ],
        "60 fps output is twice the frames for the same timeline"
    );
    assert_eq!(plan.out_frames(), 240);
}

#[test]
fn segment_frame_counts_always_sum_to_the_render_length() {
    // Conforming each segment's duration on its own would round independently
    // and drift; conforming its boundaries cannot. 24 fps off a 30 fps grid is
    // where that shows up, since no segment lands on a whole output frame.
    let project = project(
        vec![file_asset("a", AssetKind::Video)],
        vec![video_track(
            "v1",
            vec![
                clip("c1", "a", 0, 7),
                clip("c2", "a", 7, 7),
                clip("c3", "a", 21, 7),
            ],
        )],
    );
    let plan = Plan::build(&project, Fps::FILM, FrameRange::ALL).expect("plan");

    let summed: u64 = shape(&plan).iter().map(|entry| entry.3).sum();
    assert_eq!(summed, plan.out_frames());
}
