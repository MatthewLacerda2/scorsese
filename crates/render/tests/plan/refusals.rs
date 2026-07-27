//! Timelines this pipeline will not render, and why it says no rather than
//! producing something almost right.

use scorsese_core::{Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames};
use scorsese_render::{FrameRange, Plan, PlanError};

use crate::common::{clip, file_asset, project, shape, text_asset, video_track};

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

/// An *imported* asset with no path is a document nobody can render: there is
/// no prompt to put on a card and no file to decode. A prompt in the same
/// state is not this — it has a slug card, which is `plan/sketches.rs`.
#[test]
fn an_imported_asset_with_no_file_is_refused() {
    let project = project(
        vec![Asset {
            path: None,
            ..file_asset("a", AssetKind::Video)
        }],
        vec![video_track("v1", vec![clip("c1", "a", 0, 30)])],
    );
    let error = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect_err("must refuse");
    assert!(matches!(error, PlanError::NoMedia { .. }), "got {error:?}");
}

/// A text asset has no file, and the missing-media refusal must not be aimed
/// at it: its content is in the document and the compositor draws it.
#[test]
fn a_text_clip_needs_no_media_file() {
    let project = project(
        vec![text_asset("title")],
        vec![video_track("v1", vec![clip("c1", "title", 0, 30)])],
    );
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("a text clip plans");
    assert_eq!(shape(&plan), vec![(0, 30, "c1".to_owned(), 30)]);
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
