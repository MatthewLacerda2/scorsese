//! Which part of a source a clip shows.
//!
//! The whole premise of the field is that **the asset is not touched**, so what
//! these hold to is that the rectangle is a property of the *clip*, written in
//! fractions of the source, and checkable without knowing how big that source
//! is.

mod common;

use scorsese_core::{Crop, LoadError, Project, TimelineProblem, ValidationError};

/// A document with one clip carrying `body` among its fields.
fn with_clip(body: &str) -> String {
    common::document(&format!(
        r#""assets": [{{ "id": "a1", "kind": "video", "path": "assets/a.mp4" }}],
           "tracks": [{{ "id": "v1", "kind": "video", "clips": [
               {{ "id": "c1", "asset": "a1", "start": 0, "duration": 30{body} }}
           ]}}]"#
    ))
}

fn crop_of(json: &str) -> Option<Crop> {
    let project = Project::from_json(json).expect("the document parses");
    project.clips().next().expect("a clip").1.crop
}

/// The reason the field is optional: every project authored before it existed
/// keeps rendering exactly as it did.
#[test]
fn a_clip_that_says_nothing_shows_its_whole_source() {
    assert_eq!(crop_of(&with_clip("")), None);
    assert_eq!(
        Crop::default(),
        Crop {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0
        },
        "and absent has to mean the same rectangle as writing the whole source"
    );
}

#[test]
fn a_rectangle_is_read_as_four_fractions_of_the_source() {
    let crop = crop_of(&with_clip(
        r#", "crop": { "x": 0.158, "y": 0.079, "width": 0.842, "height": 0.921 }"#,
    ))
    .expect("the clip carries one");
    assert_eq!((crop.x, crop.y), (0.158, 0.079));
    assert_eq!((crop.width, crop.height), (0.842, 0.921));
}

#[test]
fn the_absent_case_is_not_written_back_out() {
    let project = Project::from_json(&with_clip("")).expect("parses");
    let json = project.to_json().expect("serialise");
    assert!(
        !json.contains("\"crop\""),
        "an uncropped clip must not grow a field it did not have:\n{json}"
    );
}

/// A partial rectangle is a rectangle whose other edges nobody stated, and
/// guessing them would be guessing which region was meant.
#[test]
fn a_rectangle_missing_an_edge_is_refused() {
    let error = Project::from_json(&with_clip(r#", "crop": { "x": 0.1, "y": 0.1 }"#))
        .expect_err("an incomplete rectangle fails");
    assert!(matches!(error, LoadError::Parse(_)), "got {error:?}");
}

/// The point of fractions: this is decidable from the document alone, where a
/// pixel rectangle would need dimensions only a probe records.
#[test]
fn a_rectangle_that_runs_off_the_source_is_refused() {
    for rectangle in [
        r#"{ "x": 0.5, "y": 0.0, "width": 0.6, "height": 1.0 }"#,
        r#"{ "x": 0.0, "y": 0.5, "width": 1.0, "height": 0.6 }"#,
        r#"{ "x": -0.1, "y": 0.0, "width": 0.5, "height": 0.5 }"#,
    ] {
        let project =
            Project::from_json(&with_clip(&format!(r#", "crop": {rectangle}"#))).expect("parses");
        let errors = project.validate().expect_err("it does not validate");
        assert!(
            errors.as_slice().iter().any(|error| matches!(
                error,
                ValidationError::Timeline(TimelineProblem::CropOutsideSource { .. })
            )),
            "{rectangle} should be refused, and gave {errors:?}"
        );
    }
}

/// A rectangle enclosing nothing is not a crop, it is a clip that would render
/// no pixels — and nothing downstream has a sensible thing to do with it.
#[test]
fn a_rectangle_enclosing_none_of_the_source_is_refused() {
    let project = Project::from_json(&with_clip(
        r#", "crop": { "x": 0.2, "y": 0.2, "width": 0.0, "height": 0.5 }"#,
    ))
    .expect("parses");
    let errors = project.validate().expect_err("it does not validate");
    assert!(
        errors.as_slice().iter().any(|error| matches!(
            error,
            ValidationError::Timeline(TimelineProblem::CropOutsideSource { .. })
        )),
        "got {errors:?}"
    );
}

/// The message names the fields the way `project.json` spells them, so it
/// points at something to edit rather than at a description of it.
#[test]
fn the_refusal_names_the_edges_as_the_document_writes_them() {
    let project = Project::from_json(&with_clip(
        r#", "crop": { "x": 0.9, "y": 0.0, "width": 0.9, "height": 1.0 }"#,
    ))
    .expect("parses");
    let said = project.validate().expect_err("refused").to_string();
    for edge in ["x", "y", "width", "height"] {
        assert!(said.contains(edge), "`{edge}` should be named in:\n{said}");
    }
}

/// The whole source is a legal thing to write, and means what absent means.
#[test]
fn a_rectangle_that_is_the_whole_source_is_accepted() {
    let project = Project::from_json(&with_clip(
        r#", "crop": { "x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0 }"#,
    ))
    .expect("parses");
    assert!(project.validate().is_ok());
}
