//! Where the incoming shot ends up, and what a dissolve refuses to do.

use scorsese_compositor::DissolveError;
use scorsese_compositor::dissolve::dissolve;
use scorsese_core::{ClipId, Frames, Project, SCHEMA_VERSION};

use crate::dissolving::{clip, cut, track_of};

#[test]
fn the_incoming_shot_is_pulled_back_onto_a_track_above() {
    let mut project = cut();
    let placed = dissolve(
        &mut project,
        &ClipId::new("a"),
        &ClipId::new("b"),
        Frames(10),
    )
    .expect("two shots that meet");

    assert!(placed.track_created, "there was no track above to use");
    assert_eq!(placed.pulled_back, 10);
    // Pulled back so the two overlap for exactly the dissolve's length.
    assert_eq!(clip(&project, "b").start, Frames(20));
    assert_eq!(clip(&project, "a").end(), Frames(30));
    // Above, so the incoming shot draws over the outgoing one. A later track
    // is what "on top" means, so this is the whole of "b is the shot arriving".
    assert!(
        track_of(&project, "b") > track_of(&project, "a"),
        "the incoming shot has to draw over the other"
    );
}

/// Every check runs before anything moves, so a refusal costs nothing. That
/// matters most for an agent working unattended: a dissolve that half-happened
/// would leave a timeline nobody asked for and no error explaining it.
#[test]
fn shots_that_do_not_meet_are_refused_and_nothing_moves() {
    let mut project = cut();
    project.tracks[0].clips[1].start = Frames(40);
    let before = project.clone();

    let refused = dissolve(
        &mut project,
        &ClipId::new("a"),
        &ClipId::new("b"),
        Frames(10),
    );

    assert!(matches!(refused, Err(DissolveError::DoNotMeet { .. })));
    assert_eq!(project, before, "a refused dissolve changed the project");
}

#[test]
fn a_shot_too_short_for_the_dissolve_is_refused() {
    let mut project = cut();
    let before = project.clone();

    let refused = dissolve(
        &mut project,
        &ClipId::new("a"),
        &ClipId::new("b"),
        Frames(90),
    );

    assert!(matches!(refused, Err(DissolveError::TooShort { .. })));
    assert_eq!(project, before);
}

/// A dissolve of no length is not a dissolve, and silently doing nothing is
/// the worst answer: the caller believes a crossover exists.
#[test]
fn a_dissolve_of_no_length_is_refused() {
    let mut project = cut();

    let refused = dissolve(
        &mut project,
        &ClipId::new("a"),
        &ClipId::new("b"),
        Frames(0),
    );

    assert!(matches!(refused, Err(DissolveError::TooShort { .. })));
}

#[test]
fn a_group_member_is_refused_by_naming_its_group() {
    let document = format!(
        r##"{{ "schema_version": {SCHEMA_VERSION}, "name": "g",
        "timeline_fps": {{ "num": 30, "den": 1 }},
        "assets": [
          {{ "id": "bed", "kind": "color", "color": "#101820" }},
          {{ "id": "pair", "kind": "group", "group": {{ "tracks": [
            {{ "id": "g1", "kind": "video", "clips": [
              {{ "id": "a", "asset": "bed", "start": 0, "duration": 30 }},
              {{ "id": "b", "asset": "bed", "start": 30, "duration": 30 }} ] }} ] }} }}
        ],
        "tracks": [ {{ "id": "v1", "kind": "video", "clips": [
          {{ "id": "both", "asset": "pair", "start": 0, "duration": 60 }} ] }} ] }}"##
    );
    let mut project = Project::from_json(&document).expect("the fixture parses");
    project.validate().expect("the fixture is valid");
    let refused = dissolve(
        &mut project,
        &ClipId::new("a"),
        &ClipId::new("b"),
        Frames(10),
    );
    let error = refused.expect_err("a member cannot be carried onto the timeline");
    assert!(matches!(error, DissolveError::InGroup { .. }));
    assert!(error.to_string().contains("inside group `pair`"), "{error}");
}
