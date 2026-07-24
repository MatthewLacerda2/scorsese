//! Timelines this pipeline will not render, and why it says no rather than
//! producing something almost right.

use scorsese_core::{AssetId, AssetKind, Clip, ClipId, Fps, Frames};
use scorsese_render::report::Note;
use scorsese_render::{FrameRange, Plan, PlanError};

use crate::common::{
    audio_track, clip, file_asset, project, sketch_asset, text_asset, video_track,
};

#[test]
fn an_empty_project_has_nothing_to_render() {
    let project = project(Vec::new(), Vec::new());
    assert_eq!(
        Plan::build(&project, Fps::THIRTY, FrameRange::ALL),
        Err(PlanError::NothingToRender)
    );
}

#[test]
fn an_empty_video_track_is_not_something_to_render_either() {
    let project = project(Vec::new(), vec![video_track("v1", Vec::new())]);
    assert_eq!(
        Plan::build(&project, Fps::THIRTY, FrameRange::ALL),
        Err(PlanError::NothingToRender)
    );
}

#[test]
fn two_video_tracks_are_refused_rather_than_flattened() {
    // Compositing tracks together is the compositor's job. Rendering only the
    // bottom one would produce a file that looks finished and is not.
    let project = project(
        vec![file_asset("a", AssetKind::Video)],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, 30)]),
            video_track("v2", vec![clip("c2", "a", 0, 30)]),
        ],
    );
    assert_eq!(
        Plan::build(&project, Fps::THIRTY, FrameRange::ALL),
        Err(PlanError::MultipleVideoTracks { count: 2 })
    );
}

#[test]
fn an_empty_second_video_track_is_ignored() {
    let project = project(
        vec![file_asset("a", AssetKind::Video)],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, 30)]),
            video_track("v2", Vec::new()),
        ],
    );
    assert!(Plan::build(&project, Fps::THIRTY, FrameRange::ALL).is_ok());
}

#[test]
fn a_sketch_clip_is_refused_until_slug_cards_exist() {
    let project = project(
        vec![sketch_asset("veo1")],
        vec![video_track("v1", vec![clip("c1", "veo1", 0, 30)])],
    );
    let error = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect_err("must refuse");
    assert!(
        matches!(error, PlanError::NotGenerated { .. }),
        "got {error:?}"
    );
}

#[test]
fn a_text_clip_is_refused_until_the_compositor_can_draw_it() {
    let project = project(
        vec![text_asset("title")],
        vec![video_track("v1", vec![clip("c1", "title", 0, 30)])],
    );
    let error = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect_err("must refuse");
    assert!(
        matches!(error, PlanError::NeedsCompositor { .. }),
        "got {error:?}"
    );
}

#[test]
fn a_clip_pointing_at_no_asset_is_refused() {
    let project = project(
        Vec::new(),
        vec![video_track(
            "v1",
            vec![Clip::new(
                ClipId::new("c1"),
                AssetId::new("ghost"),
                Frames::ZERO,
                Frames(30),
            )],
        )],
    );
    let error = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect_err("must refuse");
    assert!(
        matches!(error, PlanError::UnknownAsset { .. }),
        "got {error:?}"
    );
}

#[test]
fn a_range_that_selects_nothing_is_refused() {
    let project = project(
        vec![file_asset("a", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "a", 0, 30)])],
    );
    let range: FrameRange = "60:".parse().expect("a range");
    let error = Plan::build(&project, Fps::THIRTY, range).expect_err("must refuse");
    assert!(
        matches!(error, PlanError::EmptyRange { .. }),
        "got {error:?}"
    );
}

#[test]
fn audio_tracks_are_reported_rather_than_dropped_in_silence() {
    let project = project(
        vec![
            file_asset("a", AssetKind::Video),
            file_asset("music", AssetKind::Audio),
        ],
        vec![
            video_track("v1", vec![clip("c1", "a", 0, 30)]),
            audio_track("a1", vec![clip("m1", "music", 0, 30)]),
        ],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    assert_eq!(plan.notes(), [Note::AudioNotMixed { tracks: 1 }]);
}
