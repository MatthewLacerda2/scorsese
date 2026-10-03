//! A clip of an image sequence: its stills resolved once, in the plan, and a
//! sequence that names anything but imported pictures refused there.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, ImageSequence};
use scorsese_render::{FrameRange, Plan, PlanError};

use crate::common::{clip, file_asset, project, text_asset, video_track};

fn sequenced(stills: &[&str]) -> scorsese_core::Project {
    let sequence = ImageSequence::new(stills.iter().map(|&id| AssetId::new(id)).collect());
    project(
        vec![
            file_asset("f1", AssetKind::Image),
            file_asset("f2", AssetKind::Image),
            text_asset("title"),
            Asset::image_sequence(AssetId::new("spin"), sequence),
        ],
        vec![video_track("v1", vec![clip("c1", "spin", 0, 30)])],
    )
}

#[test]
fn a_shot_of_a_sequence_carries_its_stills_in_order() {
    let project = sequenced(&["f2", "f1", "f2"]);
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plans");
    let shot = &plan.segments()[0].layers[0];
    let stills: Vec<&str> = shot.stills.iter().map(|still| still.id.as_str()).collect();
    assert_eq!(stills, ["f2", "f1", "f2"]);
    assert_eq!(
        shot.picture().id.as_str(),
        "f2",
        "its size is its first still's"
    );
}

#[test]
fn a_still_that_is_not_an_imported_picture_is_refused() {
    for bad in ["title", "ghost"] {
        let project = sequenced(&["f1", bad]);
        let error = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect_err("refused");
        assert!(
            matches!(&error, PlanError::UnplayableStill { still, .. } if still == bad),
            "{error:?}"
        );
    }
    let mut project = sequenced(&["f1"]);
    project.assets[0].path = None;
    let error = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect_err("no file");
    assert!(
        matches!(error, PlanError::UnplayableStill { .. }),
        "{error:?}"
    );
}

#[test]
fn a_sequence_of_nothing_has_no_media() {
    let project = sequenced(&[]);
    let error = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect_err("refused");
    assert!(matches!(error, PlanError::NoMedia { .. }), "{error:?}");
}
