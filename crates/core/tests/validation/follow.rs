//! What a clip may travel along.
//!
//! Five refusals, all answered from the document. The last — an arrow attached
//! to a clip that itself follows — is the one doing the most work: it keeps a
//! path from depending on another path, so the render places every follower in
//! one pass and never has to discover a loop.

use crate::common::{assert_only_problem, clip_id, document};
use scorsese_core::{Follow, FollowProblem as F, Project};

/// A line across the frame, a box, a dot and an arrow attached to the box —
/// each on a track of its own, and the dot following `follows`.
fn following(follows: &str) -> Project {
    let body = r##"
      "assets": [
        { "id": "line", "kind": "shape", "shape": {
            "geometry": { "arrow": { "from": { "x": 0.1, "y": 0.5 },
                                     "to": { "x": 0.9, "y": 0.5 }, "curve": "s" } },
            "stroke": "#ffffffff" } },
        { "id": "tied", "kind": "shape", "shape": {
            "geometry": { "arrow": { "from": { "x": 0.1, "y": 0.1 },
                                     "to": { "attach": { "clip": "c-box", "side": "left" } } } },
            "stroke": "#ffffffff" } },
        { "id": "box", "kind": "shape", "shape": {
            "geometry": { "rectangle": { "width": 0.2, "height": 0.2 } },
            "fill": "#3366ffff" } },
        { "id": "dot", "kind": "shape", "shape": {
            "geometry": { "ellipse": { "width": 0.04, "height": 0.07 } },
            "fill": "#ffcc00ff" } }
      ],
      "tracks": [
        { "id": "v1", "kind": "video", "clips": [
            { "id": "c-line", "asset": "line", "start": 0, "duration": 30 } ] },
        { "id": "v2", "kind": "video", "clips": [
            { "id": "c-box", "asset": "box", "start": 0, "duration": 30 } ] },
        { "id": "v3", "kind": "video", "clips": [
            { "id": "c-tied", "asset": "tied", "start": 0, "duration": 30 } ] },
        { "id": "v4", "kind": "video", "clips": [
            { "id": "c-dot", "asset": "dot", "start": 0, "duration": 30 } ] }
      ]"##;
    let mut project = Project::from_json(&document(body)).expect("the fixture parses");
    dot(&mut project).follow = Some(Follow::new(clip_id(follows)));
    project
}

fn dot(project: &mut Project) -> &mut scorsese_core::Clip {
    project.tracks[3].clips.first_mut().expect("the dot's clip")
}

#[test]
fn a_clip_may_follow_an_arrow_and_one_attached_to_a_box() {
    assert_eq!(following("c-line").validate(), Ok(()));
    assert_eq!(following("c-tied").validate(), Ok(()));
}

/// The commonest way this breaks: the arrow's clip was renamed or removed.
#[test]
fn a_clip_may_not_follow_a_clip_that_is_not_there() {
    assert_only_problem(
        &following("c-nothing"),
        F::NoSuchClip {
            clip: clip_id("c-dot"),
            target: clip_id("c-nothing"),
        },
    );
}

#[test]
fn a_clip_may_follow_only_an_arrow() {
    assert_only_problem(
        &following("c-box"),
        F::NotAnArrow {
            clip: clip_id("c-dot"),
            target: clip_id("c-box"),
            asset: scorsese_core::AssetId::new("box"),
        },
    );
}

#[test]
fn an_arrow_may_not_follow_itself() {
    let mut project = following("c-line");
    dot(&mut project).follow = None;
    project.tracks[0].clips[0].follow = Some(Follow::new(clip_id("c-line")));
    assert_only_problem(
        &project,
        F::Itself {
            clip: clip_id("c-line"),
        },
    );
}

/// The box following the line would make the tied arrow — whose end is the
/// box — a path that moves because another path moved.
#[test]
fn an_arrow_attached_to_a_follower_may_not_be_followed() {
    let mut project = following("c-tied");
    project.tracks[1].clips[0].follow = Some(Follow::new(clip_id("c-line")));
    assert_only_problem(
        &project,
        F::Chained {
            clip: clip_id("c-dot"),
            arrow: clip_id("c-tied"),
            through: clip_id("c-box"),
        },
    );
}

/// Written out and read back, a follow is the same follow — and one that does
/// not orient leaves the key out, as every default in the format does.
#[test]
fn a_follow_round_trips_and_leaves_its_default_out() {
    let project = following("c-line");
    let json = project.to_json().expect("serialises");
    assert!(json.contains(r#""follow": {"#), "{json}");
    assert!(!json.contains("orient"), "{json}");
    assert_eq!(Project::from_json(&json).expect("parses"), project);
}

/// A follow that turns with the line keeps turning after a reload: `orient`
/// is written out the moment it is on.
#[test]
fn an_oriented_follow_keeps_its_orient_through_a_save() {
    let mut project = following("c-line");
    dot(&mut project)
        .follow
        .as_mut()
        .expect("it follows")
        .orient = true;
    let json = project.to_json().expect("serialises");
    assert!(json.contains(r#""orient": true"#), "{json}");
    assert_eq!(Project::from_json(&json).expect("parses"), project);
}
