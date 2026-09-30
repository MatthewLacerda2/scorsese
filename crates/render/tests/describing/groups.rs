//! What a group clip says it is: one layer, named as a group, with how many of
//! its own clips are on screen.

use scorsese_core::{Asset, AssetId, Group};
use scorsese_render::Shown;

use crate::common::{clip, project, shape_asset, video_track};
use crate::described;

#[test]
fn a_group_is_named_with_how_many_of_its_clips_are_showing() {
    let group = Group::new(vec![
        video_track("g1", vec![clip("a", "box", 0, 30)]),
        video_track("g2", vec![clip("b", "box", 0, 30)]),
    ]);
    let project = project(
        vec![
            shape_asset("box", 0.2, 0.2),
            Asset::group(AssetId::new("diagram"), group),
        ],
        vec![video_track("v1", vec![clip("shown", "diagram", 0, 30)])],
    );
    let description = described(&project);

    assert_eq!(
        description.stretches[0].picture[0].shows,
        Shown::Group { members: 2 }
    );
    assert!(
        format!("{description}").contains("diagram (group, 2 of its clips on screen)"),
        "{description}"
    );
}
