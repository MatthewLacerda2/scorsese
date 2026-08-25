//! What a clip's `origin` does to where its content lands.
//!
//! The query and the render go through the compositor's own matrix, so these
//! also assert the two agree about a pivot — an attached arrow that ignored
//! one would meet a box that has since moved.

use scorsese_core::{
    Asset, AssetId, Easing, Geometry, Keyframe, KeyframeTrack, Origin, OriginX, Project,
    PropertyPath, Rgba, Shape,
};

use super::common::{clip, project, video_track};
use super::{region, rounded};

/// A half-width, quarter-height box, centred, doubled in width by a keyframe —
/// the same setup [`super::placed`] asserts about a centred pivot, so the only
/// difference between the two answers is the field under test.
fn doubled(origin: Origin) -> Project {
    let mut project = project(
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
    );
    project.tracks[0].clips[0].origin = origin;
    project.tracks[0].clips[0].keyframes = vec![KeyframeTrack::new(
        PropertyPath::new("transform.scale.x"),
        vec![Keyframe {
            t: scorsese_core::Frames(0),
            value: 2.0,
            easing: Easing::Linear,
        }],
    )];
    project
}

#[test]
fn a_default_origin_grows_the_box_about_its_own_centre() {
    // The control: 0.5 wide about the middle becomes 1.0 wide about the middle.
    let (left, _, width, _) = rounded(region(&doubled(Origin::default()), 0, "box"));
    assert_eq!((left, width), (0.0, 1.0));
}

#[test]
fn a_left_origin_grows_the_box_away_from_the_frames_left_edge() {
    // A shape is drawn onto a layer the size of the raster, so the box the
    // pivot belongs to is the *frame* and not the rectangle the shape covers.
    // Everything therefore doubles its distance from the frame's left edge: a
    // box resting at 0.25 goes to 0.5, and its 0.5 of width becomes 1.0.
    let project = doubled(Origin {
        x: OriginX::Left,
        ..Origin::default()
    });
    let (left, _, width, _) = rounded(region(&project, 0, "box"));
    assert_eq!((left, width), (0.5, 1.0));
}

#[test]
fn a_pivot_leaves_the_axis_it_says_nothing_about_alone() {
    // Half of what makes the field cheap to set: naming `x` cannot move a
    // layer vertically, so a bar filling from its left edge does not also have
    // to say where it sits.
    let project = doubled(Origin {
        x: OriginX::Left,
        ..Origin::default()
    });
    let (_, top, _, height) = rounded(region(&project, 0, "box"));
    assert_eq!((top, height), (0.375, 0.25));
}
