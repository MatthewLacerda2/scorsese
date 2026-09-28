//! What a template holds once it is lifted out.

use scorsese_core::template::{ExtractError, extract};
use scorsese_core::{GenerationState, Project, SCHEMA_VERSION};

use super::{episode, ids, intro, placed};

/// A template is a `project.json` document: it loads, validates, and starts
/// at frame zero however far into the episode it was cut from.
#[test]
fn a_template_is_a_document_that_opens_at_zero() {
    let intro = intro();
    let reread = Project::from_json(&intro.to_json().unwrap()).expect("it round-trips");
    reread.validate().expect("and validates");
    assert_eq!(
        (intro.name.as_str(), intro.schema_version),
        ("Intro", SCHEMA_VERSION)
    );
    assert_eq!(placed(&intro, "c-intro"), ("v1".to_owned(), 0));
    assert_eq!(placed(&intro, "c-title"), ("v2".to_owned(), 30));
    assert_eq!(placed(&intro, "c-sting"), ("a1".to_owned(), 0));
    assert_eq!(intro.tracks[0].name.as_deref(), Some("Footage"));
}

/// Only what was chosen, and only what it needs.
#[test]
fn only_the_chosen_clips_and_the_assets_they_show_come_along() {
    let intro = intro();
    assert_eq!(intro.clips().count(), 3);
    let tracks: Vec<&str> = intro.tracks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(tracks, ["v1", "v2", "a1"], "v3 held nothing chosen");
    let assets: Vec<&str> = intro.assets.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(assets, ["intro", "sting", "title"]);
}

/// A generated video's still comes along with it, and a brief in flight is
/// kept as a sketch — the template records what to make, not somebody's job.
#[test]
fn a_briefs_still_comes_along_and_a_queued_brief_is_a_sketch() {
    let hero = extract(&episode(), &ids(&["c-hero"]), "Hero").expect("it stands alone");
    let assets: Vec<&str> = hero.assets.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(assets, ["face", "hero"]);
    let brief = hero
        .assets
        .iter()
        .find(|a| a.id.as_str() == "hero")
        .unwrap();
    assert_eq!(brief.state, Some(GenerationState::Sketch));
    assert_eq!(brief.operation, None);
}

#[test]
fn an_arrow_cannot_leave_the_clip_it_follows_behind() {
    let error = extract(&episode(), &ids(&["c-arrow"]), "Arrow").expect_err("c-box is not chosen");
    assert!(
        matches!(error, ExtractError::FollowsUnchosen { .. }),
        "got {error}"
    );
    extract(&episode(), &ids(&["c-arrow", "c-box"]), "Diagram").expect("with its box it stands");
}

#[test]
fn nothing_or_a_clip_that_is_not_there_is_refused() {
    let none = extract(&episode(), &ids(&[]), "Empty").expect_err("no clips");
    assert!(matches!(none, ExtractError::Nothing));
    let missing = extract(&episode(), &ids(&["c-intro", "c-gone"]), "X").expect_err("no c-gone");
    assert!(missing.to_string().contains("`c-gone`"), "got {missing}");
}
