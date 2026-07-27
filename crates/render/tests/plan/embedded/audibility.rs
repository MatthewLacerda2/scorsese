//! Whether a video clip contributes sound at all.

use scorsese_core::AssetKind;

use super::plan_of;
use crate::common::{
    audio_shape, clip, file_asset, project, silent_asset, sounding_asset, video_track,
};

#[test]
fn a_video_clip_with_sound_on_it_is_mixed() {
    let project = project(
        vec![sounding_asset("interview", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "interview", 0, 30)])],
    );

    assert_eq!(
        audio_shape(&plan_of(&project)),
        [(0, 30, "c1".to_owned())],
        "a shot with dialogue on it belongs in the mix"
    );
}

#[test]
fn a_video_clip_with_no_sound_on_it_is_not() {
    // And the mix is not merely silent, it does not exist: a film shot without
    // sound gets a file with no audio stream, which is not the same as one with
    // a stream of silence.
    let project = project(
        vec![silent_asset("plate", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "plate", 0, 30)])],
    );

    assert!(plan_of(&project).audio().is_empty());
}

#[test]
fn an_unprobed_asset_is_not_taken_for_audible() {
    // The plan reads the assets table and nothing else. "Nobody asked" is not
    // "yes", and the render's own probe pass is what turns the first into an
    // answer before the plan ever sees it.
    let project = project(
        vec![file_asset("unknown", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "unknown", 0, 30)])],
    );

    assert!(plan_of(&project).audio().is_empty());
}
