//! A `blend` with nothing beneath it: said once, worded for where it is, and
//! quiet the moment there is anything below to blend with.

use scorsese_core::{Asset, AssetId, Blend, Clip, Group, HashCheck, Project};
use scorsese_render::Checkup;

use crate::common::{clip, project, text_asset, video_track};

fn blended(id: &str, blend: Blend, start: u64, duration: u64) -> Clip {
    Clip {
        blend,
        ..clip(id, "title", start, duration)
    }
}

/// Every warning the checkup has about blends.
fn about_blends(project: &Project) -> Vec<String> {
    let nowhere = std::env::temp_dir().join("scorsese-checkup-no-such-project.scor");
    Checkup::of(project, &nowhere, HashCheck::Skip)
        .lines()
        .iter()
        .filter(|line| line.says.contains("blend:"))
        .map(|line| line.says.clone())
        .collect()
}

#[test]
fn add_on_the_lowest_layer_is_said_to_look_normal() {
    let project = project(
        vec![text_asset("title")],
        vec![video_track(
            "v1",
            vec![blended("glowing", Blend::Add, 0, 30)],
        )],
    );
    let said = about_blends(&project);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("`glowing`") && said[0].contains("`normal`"),
        "{}",
        said[0]
    );
}

#[test]
fn multiply_on_the_lowest_layer_is_said_to_disappear() {
    let project = project(
        vec![text_asset("title")],
        vec![video_track(
            "v1",
            vec![blended("stain", Blend::Multiply, 0, 30)],
        )],
    );
    let said = about_blends(&project);
    assert!(said[0].contains("disappears"), "{said:?}");
}

/// Anything below for any instant of it is enough to stay quiet — and a
/// `normal` clip on the bottom is no news at all.
#[test]
fn anything_beneath_for_any_instant_is_enough() {
    let project = project(
        vec![text_asset("title")],
        vec![
            video_track(
                "v1",
                vec![
                    clip("plate", "title", 20, 30),
                    clip("bottom", "title", 60, 10),
                ],
            ),
            video_track("v2", vec![blended("over", Blend::Screen, 0, 25)]),
        ],
    );
    assert_eq!(about_blends(&project), Vec::<String>::new());
}

/// Inside a group the canvas is transparent rather than black, and the
/// warning says so and names the group.
#[test]
fn a_member_blending_with_its_groups_empty_canvas_is_named_with_the_group() {
    let group = Group::new(vec![video_track(
        "g1",
        vec![blended("member", Blend::Multiply, 0, 30)],
    )]);
    let project = project(
        vec![
            text_asset("title"),
            Asset::group(AssetId::new("diagram"), group),
        ],
        vec![video_track("v1", vec![clip("c-diagram", "diagram", 0, 30)])],
    );
    let said = about_blends(&project);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("group `diagram`") && said[0].contains("`normal`"),
        "{}",
        said[0]
    );
}
