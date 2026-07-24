//! Rendering only part of the timeline.

use scorsese_core::{AssetKind, Fps, Frames, Project};
use scorsese_render::report::Note;
use scorsese_render::{FrameRange, FrameRangeError, Plan};

use crate::common::{clip, clip_from, file_asset, project, shape, source_ins, video_track};

fn timeline() -> Project {
    project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("b", AssetKind::Video),
        ],
        vec![video_track(
            "v1",
            vec![clip("c1", "a", 0, 60), clip("c2", "b", 60, 60)],
        )],
    )
}

fn range(text: &str) -> FrameRange {
    text.parse().expect("a frame range")
}

#[test]
fn the_written_forms_parse() {
    assert_eq!(
        range("30:120"),
        FrameRange::new(Frames(30), Some(Frames(120))).unwrap()
    );
    assert_eq!(range("30:"), FrameRange::new(Frames(30), None).unwrap());
    assert_eq!(
        range(":120"),
        FrameRange::new(Frames::ZERO, Some(Frames(120))).unwrap()
    );
    assert_eq!(range("30:120").to_string(), "30:120");
}

#[test]
fn text_that_is_not_a_range_is_refused() {
    // A bare number is refused rather than guessed at: `120` could as easily
    // mean "from 120" as "up to 120".
    assert_eq!("120".parse::<FrameRange>(), Err(FrameRangeError::Malformed));
    assert_eq!("a:b".parse::<FrameRange>(), Err(FrameRangeError::Malformed));
    assert!(matches!(
        "120:30".parse::<FrameRange>(),
        Err(FrameRangeError::Backwards { .. })
    ));
}

#[test]
fn a_range_covers_only_the_frames_asked_for() {
    let project = timeline();
    let plan = Plan::build(&project, Fps::THIRTY, range("45:75")).expect("plan");

    assert_eq!(
        shape(&plan),
        vec![(45, 15, "c1".to_owned(), 15), (60, 15, "c2".to_owned(), 15)]
    );
    assert_eq!(
        plan.out_frames(),
        30,
        "thirty frames, not the whole timeline"
    );
}

#[test]
fn starting_mid_clip_starts_mid_source() {
    // The reason this matters: rendering frames 45 onward must show what frame
    // 45 shows, not restart the clip. `c1` begins at timeline 0, so entering
    // the render at 45 means entering its source 45 frames in; `c2` begins at
    // 60, inside the range, so it still opens at its own start.
    let project = timeline();
    let plan = Plan::build(&project, Fps::THIRTY, range("45:")).expect("plan");

    assert_eq!(
        source_ins(&plan),
        vec![("c1".to_owned(), 45), ("c2".to_owned(), 0)]
    );
}

#[test]
fn a_clips_own_source_in_is_added_to_what_the_range_skipped() {
    let project = project(
        vec![file_asset("a", AssetKind::Video)],
        vec![video_track("v1", vec![clip_from("c1", "a", 0, 60, 90)])],
    );
    let plan = Plan::build(&project, Fps::THIRTY, range("20:")).expect("plan");

    assert_eq!(source_ins(&plan), vec![("c1".to_owned(), 110)]);
}

#[test]
fn asking_past_the_end_renders_to_the_end_and_says_so() {
    let project = timeline();
    let plan = Plan::build(&project, Fps::THIRTY, range("90:400")).expect("plan");

    assert_eq!(plan.out_frames(), 30, "the timeline ends at 120");
    assert_eq!(
        plan.notes(),
        [Note::RangeClamped {
            asked: Frames(400),
            timeline_end: Frames(120)
        }]
    );
}

#[test]
fn clips_entirely_outside_the_range_are_left_out() {
    let project = timeline();
    let plan = Plan::build(&project, Fps::THIRTY, range("60:120")).expect("plan");

    let included: Vec<String> = shape(&plan).into_iter().map(|entry| entry.2).collect();
    assert_eq!(included, vec!["c2".to_owned()]);
}
