//! What a document says about a clip's `matte`, and what it leaves out.

mod common;

use scorsese_core::{ClipId, Matte, Project};

fn clip_with(extra: &str) -> Project {
    let json = common::document(&format!(
        r##""assets": [{{ "id": "a", "kind": "color", "color": "#ffffff" }}],
           "tracks": [{{ "id": "v1", "kind": "video", "clips": [
               {{ "id": "c", "asset": "a", "start": 0, "duration": 30 {extra} }}] }},
             {{ "id": "v2", "kind": "video", "clips": [
               {{ "id": "m", "asset": "a", "start": 0, "duration": 30 }}] }}]"##
    ));
    Project::from_json(&json).expect("parses")
}

fn first(project: &Project) -> &scorsese_core::Clip {
    project.clips().next().expect("a clip").1
}

#[test]
fn no_matte_writes_none() {
    let project = clip_with("");
    assert_eq!(first(&project).matte, None);
    let json = project.to_json().expect("serialise");
    assert!(!json.contains("matte"), "nothing invented: {json}");
}

/// `invert` defaults to false and, false, is left out on the way back.
#[test]
fn a_matte_round_trips_and_invert_defaults_off() {
    let project = clip_with(r#", "matte": { "clip": "m" }"#);
    assert_eq!(first(&project).matte, Some(Matte::new(ClipId::new("m"))));
    assert!(project.is_valid());
    let json = project.to_json().expect("serialise");
    assert!(!json.contains("invert"), "a default not written: {json}");
    assert_eq!(Project::from_json(&json).expect("reloads"), project);

    let inverted = clip_with(r#", "matte": { "clip": "m", "invert": true }"#);
    assert!(first(&inverted).matte.as_ref().is_some_and(|m| m.invert));
}

#[test]
fn an_unknown_key_in_a_matte_is_refused() {
    let json = common::document(
        r#""assets": [], "tracks": [{ "id": "v1", "kind": "video", "clips": [
            { "id": "c", "asset": "a", "start": 0, "duration": 30,
              "matte": { "clip": "m", "luma": true } }] }]"#,
    );
    let error = Project::from_json(&json).expect_err("luma is not a key");
    assert!(error.to_string().contains("luma"), "names it: {error}");
}
