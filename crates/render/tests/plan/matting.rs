//! A track matte in the plan: the matte clip is never a layer of its own, it
//! rides on the shot it masks, and a masked shot with no matte on screen is
//! either nothing (the ordinary way round) or itself (inverted).

use scorsese_core::{Asset, AssetId, ClipId, Fps, Group, Matte, Project};
use scorsese_render::{FrameRange, Plan};

use crate::common::{clip, project, shape, shape_asset, video_track};
use crate::two_videos;

/// A bed and a picture over the whole sixty frames, the picture masked by a
/// shape that is only there from 20 to 40.
fn wiped(invert: bool) -> Project {
    let mut top = clip("top", "over", 0, 60);
    top.matte = Some(Matte {
        clip: ClipId::new("wipe"),
        invert,
    });
    let mut assets = two_videos();
    assets.push(shape_asset("box", 1.0, 1.0));
    project(
        assets,
        vec![
            video_track("v1", vec![clip("bed", "under", 0, 60)]),
            video_track("v2", vec![top]),
            video_track("v3", vec![clip("wipe", "box", 20, 20)]),
        ],
    )
}

/// Which clip masks each layer of each stretch, as `id<matte`.
fn mattes(plan: &Plan<'_>) -> Vec<Vec<String>> {
    plan.segments()
        .iter()
        .map(|segment| {
            segment
                .layers
                .iter()
                .filter_map(|shot| {
                    let matte = shot.matte.as_ref()?;
                    Some(format!("{}<{}", shot.clip.id, matte.shot.clip.id))
                })
                .collect()
        })
        .collect()
}

#[test]
fn a_matte_is_no_layer_and_the_masked_shot_shows_only_while_it_is_there() {
    let project = wiped(false);
    project.validate().expect("valid");
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    assert_eq!(
        shape(&plan),
        vec![
            (0, 20, "bed".to_owned(), 20),
            (20, 20, "bed+top".to_owned(), 20),
            (40, 20, "bed".to_owned(), 20),
        ],
        "the wipe is never a layer; the picture is only there through it"
    );
    assert_eq!(
        mattes(&plan),
        vec![vec![], vec!["top<wipe".to_owned()], vec![]]
    );
    assert_eq!(plan.widest_stack(), 3, "the matte still has to be drawn");
}

#[test]
fn inverted_the_masked_shot_is_whole_while_the_matte_is_away() {
    let project = wiped(true);
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    let layers: Vec<String> = shape(&plan).into_iter().map(|(_, _, l, _)| l).collect();
    assert_eq!(layers, ["bed+top", "bed+top", "bed+top"]);
    assert_eq!(
        mattes(&plan),
        vec![vec![], vec!["top<wipe".to_owned()], vec![]]
    );
}

/// Inside a group the same rule holds among the group's own members.
#[test]
fn a_matte_inside_a_group_rides_on_the_member_it_masks() {
    let mut inner = clip("inner", "box", 0, 30);
    inner.matte = Some(Matte::new(ClipId::new("hole")));
    let group = Group::new(vec![
        video_track("g1", vec![inner]),
        video_track("g2", vec![clip("hole", "box", 0, 30)]),
    ]);
    let project = project(
        vec![
            shape_asset("box", 0.5, 0.5),
            Asset::group(AssetId::new("g"), group),
        ],
        vec![video_track("v1", vec![clip("shown", "g", 0, 30)])],
    );
    project.validate().expect("valid");
    let plan = Plan::build(&project, Fps::THIRTY, FrameRange::ALL).expect("plan");
    let members = &plan.segments()[0].layers[0].members;
    assert_eq!(members.len(), 1, "the hole is not a member layer");
    assert_eq!(members[0].clip.id.as_str(), "inner");
    let matte = members[0].matte.as_ref().expect("masked");
    assert_eq!(matte.shot.clip.id.as_str(), "hole");
}
