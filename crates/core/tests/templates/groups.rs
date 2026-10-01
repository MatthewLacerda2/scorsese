//! A template carrying a group (#598): the group, what its members show and
//! the groups nested in it come along, and land under ids free in the project.

use scorsese_core::template::{extract, insert};
use scorsese_core::{AssetId, Clip, Fps, Frames, Group, Project, SCHEMA_VERSION, TrackId};

use super::ids;

/// A diagram grouped twice over, on a background.
///
/// ```text
/// diagram   diagram-v1  [m-box ............ 94]
///           diagram-v2  [m-arrow → m-box .. 94]
///           diagram-v3  [m-inner: inner . 60]
/// inner     inner-v1    [m-dot .......... 60]
///
/// v1  [bg ..................................... 150]
/// v2      33 [c-diagram: diagram .......... 127]
/// ```
pub(super) fn explainer() -> Project {
    let document = format!(
        r##"{{ "schema_version": {SCHEMA_VERSION}, "name": "Explainer",
        "timeline_fps": {{ "num": 30, "den": 1 }},
        "assets": [
          {{ "id": "bed", "kind": "color", "color": "#101820" }},
          {{ "id": "box", "kind": "shape", "shape": {{
              "geometry": {{ "rectangle": {{ "width": 0.2, "height": 0.1 }} }},
              "fill": "#ffffffff" }} }},
          {{ "id": "link", "kind": "shape", "shape": {{
              "geometry": {{ "arrow": {{ "from": {{ "x": 0.1, "y": 0.1 }},
                  "to": {{ "attach": {{ "clip": "m-box", "side": "left" }} }} }} }},
              "stroke": "#ffffffff" }} }},
          {{ "id": "inner", "kind": "group", "group": {{ "tracks": [
            {{ "id": "inner-v1", "kind": "video", "clips": [
              {{ "id": "m-dot", "asset": "box", "start": 0, "duration": 60 }} ] }} ] }} }},
          {{ "id": "diagram", "kind": "group", "group": {{ "tracks": [
            {{ "id": "diagram-v1", "kind": "video", "clips": [
              {{ "id": "m-box", "asset": "box", "start": 0, "duration": 94 }} ] }},
            {{ "id": "diagram-v2", "kind": "video", "clips": [
              {{ "id": "m-arrow", "asset": "link", "start": 0, "duration": 94 }} ] }},
            {{ "id": "diagram-v3", "kind": "video", "clips": [
              {{ "id": "m-inner", "asset": "inner", "start": 0, "duration": 60 }} ] }} ] }} }}
        ],
        "tracks": [
          {{ "id": "v1", "kind": "video", "clips": [
            {{ "id": "bg", "asset": "bed", "start": 0, "duration": 150 }} ] }},
          {{ "id": "v2", "kind": "video", "clips": [
            {{ "id": "c-diagram", "asset": "diagram", "start": 33, "duration": 94 }} ] }}
        ] }}"##
    );
    let project = Project::from_json(&document).expect("the fixture parses");
    project.validate().expect("the fixture is valid");
    project
}

fn saved() -> Project {
    extract(&explainer(), &ids(&["bg", "c-diagram"]), "Explainer").expect("a group comes along")
}

fn group<'a>(project: &'a Project, id: &str) -> &'a Group {
    let asset = project.assets.iter().find(|a| a.id.as_str() == id);
    asset.and_then(|a| a.group.as_ref()).expect("a group")
}

fn member<'a>(group: &'a Group, id: &str) -> &'a Clip {
    group
        .clips()
        .map(|(_, c)| c)
        .find(|c| c.id.as_str() == id)
        .expect("a member")
}

/// The group, the group nested in it and every asset a member shows — the
/// arrow following a member is no arrow left pointing at nothing.
#[test]
fn a_group_comes_along_with_everything_its_members_show() {
    let template = saved();
    let assets: Vec<&str> = template.assets.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(assets, ["bed", "box", "link", "inner", "diagram"]);
    assert_eq!(group(&template, "diagram").clips().count(), 3);
}

/// Into a project with no ids in common, everything keeps its own.
#[test]
fn into_an_empty_project_a_group_keeps_its_ids() {
    let mut project = Project::new("Episode 2", Fps::THIRTY);
    insert(&mut project, &saved(), Frames(0)).expect("it lands");
    let lanes: Vec<&str> = group(&project, "diagram")
        .tracks
        .iter()
        .map(|t| t.id.as_str())
        .collect();
    assert_eq!(lanes, ["diagram-v1", "diagram-v2", "diagram-v3"]);
    member(group(&project, "diagram"), "m-box");
}

/// Inserted beside the original, the copy's members, lanes and every
/// reference inside it are renamed, so nothing in it names the original.
#[test]
fn a_copied_group_is_renamed_inside_as_well() {
    let mut project = explainer();
    let done = insert(&mut project, &saved(), Frames(600)).expect("it lands");
    let added: Vec<&str> = done.added.iter().map(|a| a.as_str()).collect();
    assert_eq!(added, ["bed-2", "box-2", "link-2", "inner-2", "diagram-2"]);

    let copy = group(&project, "diagram-2");
    let lanes: Vec<&str> = copy.tracks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(lanes, ["diagram-v1-2", "diagram-v2-2", "diagram-v3-2"]);
    assert_eq!(member(copy, "m-box-2").asset.as_str(), "box-2");
    assert_eq!(member(copy, "m-inner-2").asset.as_str(), "inner-2");
    let nested = group(&project, "inner-2");
    assert_eq!(nested.tracks[0].id.as_str(), "inner-v1-2");
    assert_eq!(member(nested, "m-dot-2").asset.as_str(), "box-2");

    let arrow = project
        .asset(&AssetId::new("link-2"))
        .expect("the copied arrow");
    let json = serde_json::to_string(arrow).unwrap();
    assert!(json.contains("\"m-box-2\""), "{json}");
    let placed = project
        .clips()
        .map(|(_, c)| c)
        .find(|c| c.id.as_str() == "c-diagram-2");
    assert_eq!(placed.expect("the group clip").asset.as_str(), "diagram-2");
}

/// Saved at 30 fps and inserted at 24: the members are conformed with the
/// group clip, and the clip that showed all of the group still fits in it,
/// though rounding alone would leave it a frame past the group's new end.
#[test]
fn a_group_is_conformed_with_its_clip() {
    let mut project = Project::new("Film", Fps::new(24, 1).unwrap());
    insert(&mut project, &saved(), Frames(0)).expect("it lands");
    let diagram = group(&project, "diagram");
    assert_eq!(
        diagram.length(),
        Frames(75),
        "94 frames at 30 is 75.2 at 24"
    );
    assert_eq!(
        member(group(&project, "inner"), "m-dot").duration,
        Frames(48)
    );
    let clip = project
        .clips()
        .map(|(_, c)| c)
        .find(|c| c.id.as_str() == "c-diagram");
    let clip = clip.expect("the group clip");
    assert_eq!((clip.start, clip.duration), (Frames(26), Frames(75)));
}

/// A new lane for the template keeps its id where it is free, and is never
/// given one a group's lane already has: track ids are one namespace for the
/// whole document.
#[test]
fn a_new_lane_does_not_take_a_groups_lane_id() {
    let source = explainer();
    let mut project = Project::new("Episode 2", Fps::THIRTY);
    for id in ["box", "inner"] {
        project
            .assets
            .push(source.asset(&AssetId::new(id)).unwrap().clone());
    }
    project.assets[1].group.as_mut().unwrap().tracks[0].id = TrackId::new("v1");
    let mut template = saved();
    template.tracks[1].id = TrackId::new("titles");
    let done = insert(&mut project, &template, Frames(0)).expect("it lands");
    let lanes: Vec<&str> = done.new_tracks.iter().map(|t| t.as_str()).collect();
    assert_eq!(lanes, ["v2", "titles"], "v1 is the group's, titles is free");
}
