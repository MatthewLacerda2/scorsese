//! How a clip says it wants to be fitted into the raster.

mod common;

use scorsese_core::{Fit, LoadError, Project};

/// A document with one clip carrying `body` among its fields.
fn with_clip(body: &str) -> String {
    common::document(&format!(
        r#""assets": [{{ "id": "a1", "kind": "video", "path": "assets/a.mp4" }}],
           "tracks": [{{ "id": "v1", "kind": "video", "clips": [
               {{ "id": "c1", "asset": "a1", "start": 0, "duration": 30{body} }}
           ]}}]"#
    ))
}

fn fit_of(json: &str) -> Fit {
    let project = Project::from_json(json).expect("the document parses");
    project.clips().next().expect("a clip").1.fit
}

#[test]
fn a_clip_that_says_nothing_is_fitted() {
    // The whole reason the field is optional: every project authored before it
    // existed keeps rendering exactly as it did.
    assert_eq!(fit_of(&with_clip("")), Fit::Fit);
}

#[test]
fn the_three_modes_are_snake_case_on_the_wire() {
    assert_eq!(fit_of(&with_clip(r#", "fit": "fit""#)), Fit::Fit);
    assert_eq!(fit_of(&with_clip(r#", "fit": "fill""#)), Fit::Fill);
    assert_eq!(fit_of(&with_clip(r#", "fit": "native""#)), Fit::Native);
}

#[test]
fn a_fit_mode_that_does_not_exist_is_refused() {
    // Not silently fitted: a clip asking to be "cover"ed meant something, and
    // guessing which of the three it meant is worse than saying no.
    let error =
        Project::from_json(&with_clip(r#", "fit": "cover""#)).expect_err("an unknown mode fails");
    assert!(matches!(error, LoadError::Parse(_)), "got {error:?}");
}

#[test]
fn the_default_is_not_written_back_out() {
    let project = Project::from_json(&with_clip("")).expect("parses");
    let json = project.to_json().expect("serialise");
    assert!(
        !json.contains("\"fit\""),
        "a clip that never chose a fit mode should not gain one on save: {json}"
    );
}

#[test]
fn a_chosen_mode_round_trips() {
    let project = Project::from_json(&with_clip(r#", "fit": "native""#)).expect("parses");
    let json = project.to_json().expect("serialise");
    assert!(json.contains("\"native\""), "{json}");
    assert_eq!(Project::from_json(&json).expect("reparse"), project);
}

#[test]
fn fit_is_not_a_reason_to_refuse_a_project() {
    // It says nothing about coherence — every mode is renderable, and an audio
    // clip carrying one is meaningless rather than wrong.
    let project = Project::from_json(&with_clip(r#", "fit": "fill""#)).expect("parses");
    assert_eq!(project.validate(), Ok(()));
}
