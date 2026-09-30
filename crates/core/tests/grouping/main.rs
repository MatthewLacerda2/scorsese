//! Wrapping clips on the timeline into a group, and back.

mod ungrouping;

use std::collections::BTreeSet;

use scorsese_core::grouping::{self, GroupError, Grouping};
use scorsese_core::{AssetKind, ClipId, Frames, Project, SCHEMA_VERSION};

/// A background on `v1`, two boxes on `v2` and `v3` starting a second in, and
/// an arrow on `v4` following the second box.
pub(crate) fn diagram() -> Project {
    let document = format!(
        r##"{{ "schema_version": {SCHEMA_VERSION}, "name": "g",
        "timeline_fps": {{ "num": 30, "den": 1 }},
        "assets": [
          {{ "id": "bed", "kind": "color", "color": "#101820" }},
          {{ "id": "box", "kind": "shape", "shape": {{
              "geometry": {{ "rectangle": {{ "width": 0.2, "height": 0.1 }} }},
              "fill": "#ffffffff" }} }},
          {{ "id": "link", "kind": "shape", "shape": {{
              "geometry": {{ "arrow": {{ "from": {{ "x": 0.1, "y": 0.1 }},
                  "to": {{ "attach": {{ "clip": "b2", "side": "left" }} }} }} }},
              "stroke": "#ffffffff", "stroke_width": 0.004 }} }}
        ],
        "tracks": [
          {{ "id": "v1", "kind": "video", "clips": [
            {{ "id": "bg", "asset": "bed", "start": 0, "duration": 120 }} ] }},
          {{ "id": "v2", "kind": "video", "clips": [
            {{ "id": "b1", "asset": "box", "start": 30, "duration": 60 }} ] }},
          {{ "id": "v3", "kind": "video", "clips": [
            {{ "id": "b2", "asset": "box", "start": 45, "duration": 60 }} ] }},
          {{ "id": "v4", "kind": "video", "clips": [
            {{ "id": "arrow", "asset": "link", "start": 45, "duration": 30 }} ] }}
        ] }}"##
    );
    let project = Project::from_json(&document).expect("the fixture parses");
    project.validate().expect("the fixture is valid");
    project
}

pub(crate) fn ids(names: &[&str]) -> BTreeSet<ClipId> {
    names.iter().map(|name| ClipId::new(*name)).collect()
}

pub(crate) fn choose(names: &[&str]) -> Grouping {
    Grouping {
        clips: ids(names),
        asset: None,
        clip: None,
        track: None,
    }
}

#[test]
fn clips_become_one_clip_of_a_new_group_at_the_same_place_and_time() {
    let mut project = diagram();
    let grouped = grouping::group(&mut project, &choose(&["b1", "b2", "arrow"])).unwrap();

    assert_eq!(grouped.asset.as_str(), "group");
    assert_eq!(grouped.clip.as_str(), "c-group");
    assert_eq!(
        grouped.track.as_str(),
        "v2",
        "the lowest track a clip came from"
    );
    assert_eq!((grouped.start, grouped.duration), (Frames(30), Frames(75)));
    let asset = project
        .asset(&grouped.asset)
        .expect("the group is in the table");
    assert_eq!(asset.kind, AssetKind::Group);
    let group = asset.group.as_ref().expect("with its tracks");
    let lanes: Vec<&str> = group.tracks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(
        lanes,
        ["group-v2", "group-v3", "group-v4"],
        "in stacking order"
    );
    let starts: Vec<(&str, u64)> = group
        .clips()
        .map(|(_, c)| (c.id.as_str(), c.start.get()))
        .collect();
    assert_eq!(
        starts,
        [("b1", 0), ("b2", 15), ("arrow", 15)],
        "from the group's zero"
    );
    assert_eq!(
        project.clips().count(),
        2,
        "the background and the group clip"
    );
    assert_eq!(project.every_clip().count(), 5, "and the three inside");
    project.validate().expect("the result is valid");
}

/// The arrow follows `b2`; grouping `b2` without it would leave an arrow on
/// the frame aimed into a group, which is refused — and nothing is written.
#[test]
fn an_arrow_left_outside_refuses_the_whole_request() {
    let mut project = diagram();
    let before = project.clone();
    let error = grouping::group(&mut project, &choose(&["b1", "b2"])).unwrap_err();
    assert!(matches!(error, GroupError::Refused(_)), "got {error}");
    assert!(
        error.to_string().contains("`link`"),
        "names the arrow: {error}"
    );
    assert_eq!(project, before);
}

#[test]
fn nothing_unknown_is_grouped_and_no_id_is_taken_twice() {
    let mut project = diagram();
    let error = grouping::group(&mut project, &choose(&["b1", "nope"])).unwrap_err();
    assert!(
        matches!(error, GroupError::NoSuchClip { .. }),
        "got {error}"
    );
    let error = grouping::group(&mut project, &choose(&[])).unwrap_err();
    assert!(matches!(error, GroupError::Nothing), "got {error}");
    let mut named = choose(&["b1"]);
    named.asset = Some(scorsese_core::AssetId::new("box"));
    let error = grouping::group(&mut project, &named).unwrap_err();
    assert!(matches!(error, GroupError::TakenId { .. }), "got {error}");
    assert_eq!(project, diagram());
}
