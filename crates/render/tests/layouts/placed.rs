//! The rectangles that can be answered for, and what moves them.

use scorsese_core::{
    Anchor, AnchorY, Asset, AssetId, Easing, Geometry, Keyframe, KeyframeTrack, Project,
    PropertyPath, Rgba, Shape,
};

use super::common::{clip, project, text_asset, video_track};
use super::{layout, region, rounded};

/// A half-width, quarter-height box, which centred is a rectangle anyone can
/// work out on paper: the check is that the compositor agrees.
fn boxed() -> Project {
    project(
        vec![Asset::shape(
            AssetId::new("panel"),
            Shape::filled(
                Geometry::Rectangle {
                    width: 0.5,
                    height: 0.25,
                    radius: 0.0,
                },
                Rgba::WHITE,
            ),
        )],
        vec![video_track("diagram", vec![clip("box", "panel", 0, 30)])],
    )
}

#[test]
fn a_centred_box_covers_the_middle_half_of_the_frame() {
    assert_eq!(
        rounded(region(&boxed(), 0, "box")),
        (0.25, 0.375, 0.5, 0.25)
    );
}

#[test]
fn an_anchor_rests_the_box_against_the_edge_it_names() {
    let mut project = boxed();
    project.tracks[0].clips[0].anchor = Anchor {
        y: AnchorY::Bottom,
        ..Anchor::default()
    };
    let (_, top, _, height) = rounded(region(&project, 0, "box"));
    assert_eq!((top, height), (0.75, 0.25));
}

#[test]
fn a_position_keyframe_moves_the_rectangle_with_the_layer() {
    let mut project = boxed();
    project.tracks[0].clips[0].keyframes = vec![KeyframeTrack::new(
        PropertyPath::new("transform.position.x"),
        vec![
            Keyframe {
                t: scorsese_core::Frames(0),
                value: 0.0,
                easing: Easing::Linear,
            },
            Keyframe {
                t: scorsese_core::Frames(20),
                value: 0.2,
                easing: Easing::Linear,
            },
        ],
    )];
    // A tenth of the way through a move of 0.2 of the width, at frame 10.
    assert_eq!(rounded(region(&project, 10, "box")).0, 0.35);
}

#[test]
fn a_scale_keyframe_grows_the_rectangle_about_its_own_centre() {
    let mut project = boxed();
    project.tracks[0].clips[0].keyframes = vec![KeyframeTrack::new(
        PropertyPath::new("transform.scale.x"),
        vec![Keyframe {
            t: scorsese_core::Frames(0),
            value: 2.0,
            easing: Easing::Linear,
        }],
    )];
    let (left, _, width, _) = rounded(region(&project, 0, "box"));
    assert_eq!((left, width), (0.0, 1.0));
}

#[test]
fn a_colour_covers_the_whole_frame() {
    let project = project(
        vec![Asset::color(AssetId::new("wash"), Rgba::WHITE)],
        vec![video_track("bg", vec![clip("plate", "wash", 0, 30)])],
    );
    assert_eq!(rounded(region(&project, 0, "plate")), (0.0, 0.0, 1.0, 1.0));
}

#[test]
fn a_video_sketch_has_the_rectangle_its_card_draws() {
    let project = project(
        vec![super::common::sketch_asset("shot")],
        vec![video_track("picture", vec![clip("open", "shot", 0, 30)])],
    );
    assert_eq!(rounded(region(&project, 0, "open")), (0.0, 0.0, 1.0, 1.0));
}

#[test]
fn a_narration_sketch_is_the_band_it_draws_across_the_foot() {
    let project = project(
        vec![text_asset("words"), super::common::narration_asset("vo")],
        vec![
            video_track("titles", vec![clip("card", "words", 0, 30)]),
            super::common::audio_track("narration", vec![clip("line", "vo", 0, 30)]),
        ],
    );
    assert_eq!(rounded(region(&project, 0, "line")), (0.0, 0.7, 1.0, 0.3));
}

#[test]
fn a_title_is_the_wrapped_block_and_not_the_frame_it_is_set_on() {
    let project = project(
        vec![text_asset("words")],
        vec![video_track("titles", vec![clip("card", "words", 0, 30)])],
    );
    let block = region(&project, 0, "card");
    assert!(
        block.width < 1.0 && block.height < 1.0,
        "a line of text is not the whole frame: {block}"
    );
    // Centred, so what is left over is the same either side.
    let slack = (block.left - (1.0 - block.right())).abs();
    assert!(slack < 0.001, "the block is not centred: {block}");
}

#[test]
fn the_layers_come_back_bottom_first_the_way_they_are_drawn() {
    let mut project = boxed();
    project
        .assets
        .push(Asset::color(AssetId::new("wash"), Rgba::opaque(0, 0, 0)));
    project
        .tracks
        .insert(0, video_track("bg", vec![clip("plate", "wash", 0, 30)]));
    let placed = layout(&project, 0).placed;
    let order: Vec<&str> = placed
        .iter()
        .map(|placement| placement.clip.as_str())
        .collect();
    assert_eq!(order, ["plate", "box"]);
}

#[test]
fn two_boxes_that_share_the_frame_overlap_and_two_that_do_not_never_touch() {
    let over = region(&boxed(), 0, "box");
    let mut aside = boxed();
    aside.tracks[0].clips[0].anchor = Anchor {
        y: AnchorY::Bottom,
        ..Anchor::default()
    };
    let below = region(&aside, 0, "box");
    assert!(over.overlaps(&over));
    assert!(!over.overlaps(&below));
}
