//! Where a template lands, and what it is called once it is there.

use scorsese_core::template::{InsertError, extract, insert};
use scorsese_core::{Fps, Frames, Project};

use super::{episode, ids, intro, placed};

fn empty() -> Project {
    Project::new("Episode 2", Fps::THIRTY)
}

/// A project with no tracks takes the template's, ids and all.
#[test]
fn into_an_empty_project_the_template_brings_its_own_lanes() {
    let mut project = empty();
    let done = insert(&mut project, &intro(), Frames(30)).expect("it lands");
    let lanes: Vec<&str> = project.tracks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(lanes, ["v1", "v2", "a1"]);
    assert_eq!(done.new_tracks.len(), 3);
    assert_eq!(placed(&project, "c-title"), ("v2".to_owned(), 60));
    assert_eq!((done.start, done.end), (Frames(30), Frames(150)));
    assert_eq!(project.tracks[0].name.as_deref(), Some("Footage"));
}

/// Where the lanes are free the template goes onto them, by position among
/// the tracks of its kind — no new lanes for an outro after the footage.
#[test]
fn onto_free_lanes_it_lands_by_position_among_its_kind() {
    let mut project = episode();
    let lanes = project.tracks.len();
    let done = insert(&mut project, &intro(), Frames(600)).expect("it lands");
    assert!(done.new_tracks.is_empty(), "{done:?}");
    assert_eq!(project.tracks.len(), lanes);
    assert_eq!(placed(&project, "c-intro-2"), ("v1".to_owned(), 600));
    assert_eq!(placed(&project, "c-title-2"), ("v2".to_owned(), 630));
    assert_eq!(placed(&project, "c-sting-2"), ("a1".to_owned(), 600));
}

/// A blocked lane sends it — and every lane of its kind above it — to new
/// tracks on top, so the template's title is still drawn over its footage.
#[test]
fn a_blocked_lane_and_those_above_it_spill_onto_new_tracks_in_order() {
    let mut project = episode();
    let done = insert(&mut project, &intro(), Frames(100)).expect("it lands");
    let lanes: Vec<&str> = project.tracks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(lanes, ["v1", "v2", "v3", "a1", "v4", "v5", "a2"]);
    assert_eq!(done.new_tracks.len(), 3);
    assert_eq!(placed(&project, "c-intro-2").0, "v4");
    assert_eq!(placed(&project, "c-title-2").0, "v5");
}

/// Copy, not link: a caption is copied, but a file the project already has
/// is its own asset, referenced rather than added twice.
#[test]
fn inserted_twice_captions_are_copied_and_files_are_shared() {
    let mut project = empty();
    insert(&mut project, &intro(), Frames(0)).expect("once");
    let again = insert(&mut project, &intro(), Frames(300)).expect("twice");
    assert_eq!(
        again.added.iter().map(|a| a.as_str()).collect::<Vec<_>>(),
        ["title-2"]
    );
    assert_eq!(
        again.reused.iter().map(|a| a.as_str()).collect::<Vec<_>>(),
        ["intro", "sting"]
    );
    let title = project
        .clips()
        .find(|(_, c)| c.id.as_str() == "c-title-2")
        .unwrap()
        .1;
    assert_eq!(title.asset.as_str(), "title-2");
    assert_eq!(title.keyframes.len(), 1, "its fade came with it");
}

/// An arrow follows the copy of its box, not the original.
#[test]
fn a_copied_arrow_follows_the_copied_box() {
    let diagram = extract(&episode(), &ids(&["c-box", "c-arrow"]), "Diagram").unwrap();
    let mut project = episode();
    insert(&mut project, &diagram, Frames(900)).expect("it lands");
    let arrow = project
        .assets
        .iter()
        .find(|a| a.id.as_str() == "arrow-2")
        .unwrap();
    let json = serde_json::to_string(arrow).unwrap();
    assert!(json.contains("\"c-box-2\""), "{json}");
}

/// Saved at 30 fps, a second is still a second at 24.
#[test]
fn a_template_is_conformed_to_the_projects_frame_rate() {
    let mut project = Project::new("Film", Fps::new(24, 1).unwrap());
    insert(&mut project, &intro(), Frames(0)).expect("it lands");
    assert_eq!(placed(&project, "c-title"), ("v2".to_owned(), 24));
    let intro = project
        .clips()
        .find(|(_, c)| c.id.as_str() == "c-intro")
        .unwrap()
        .1;
    assert_eq!(intro.duration, Frames(96));
}

/// Nothing is half inserted: a clip that would not survive the conform stops
/// the whole insertion.
#[test]
fn a_refused_insertion_writes_nothing() {
    let mut blink = intro();
    blink.timeline_fps = Fps::new(120, 1).unwrap();
    blink.tracks[0].clips[0].duration = Frames(1);
    let mut project = episode();
    let before = project.clone();
    let error = insert(&mut project, &blink, Frames(0)).expect_err("a quarter of a frame");
    assert!(matches!(error, InsertError::Vanishes(_)), "got {error}");
    assert_eq!(project, before);
}
