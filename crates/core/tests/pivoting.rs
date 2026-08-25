//! Which point of itself a clip's transform turns about: `origin`.

mod common;

use scorsese_core::{LoadError, Origin, OriginX, OriginY, Project};

/// A document with one clip carrying `body` among its fields.
fn with_clip(body: &str) -> String {
    common::document(&format!(
        r#""assets": [{{ "id": "a1", "kind": "video", "path": "assets/a.mp4" }}],
           "tracks": [{{ "id": "v1", "kind": "video", "clips": [
               {{ "id": "c1", "asset": "a1", "start": 0, "duration": 30{body} }}
           ]}}]"#
    ))
}

fn origin_of(json: &str) -> Origin {
    let project = Project::from_json(json).expect("the document parses");
    project.clips().next().expect("a clip").1.origin
}

#[test]
fn a_clip_that_says_nothing_turns_about_its_centre() {
    // The whole reason the field is optional: every project authored before it
    // existed keeps rendering exactly as it did.
    assert_eq!(origin_of(&with_clip("")), Origin::default());
    assert_eq!(Origin::default().fractions(), (0.5, 0.5));
}

#[test]
fn each_named_point_is_the_fraction_of_the_box_it_names() {
    // The arithmetic the compositor pivots on, asserted by name rather than by
    // rendering a frame and looking at where an edge landed.
    let fractions = |x, y| Origin { x, y }.fractions();
    assert_eq!(fractions(OriginX::Left, OriginY::Top), (0.0, 0.0));
    assert_eq!(fractions(OriginX::Center, OriginY::Center), (0.5, 0.5));
    assert_eq!(fractions(OriginX::Right, OriginY::Bottom), (1.0, 1.0));
    assert_eq!(fractions(OriginX::Left, OriginY::Bottom), (0.0, 1.0));
    assert_eq!(fractions(OriginX::Right, OriginY::Top), (1.0, 0.0));
}

#[test]
fn one_axis_may_be_named_without_the_other() {
    // Half the point of the bar that prompted this: it pivots on its left edge
    // and stays where it is vertically, so naming `y` would be noise.
    let origin = origin_of(&with_clip(r#", "origin": { "x": "left" }"#));
    assert_eq!(origin.x, OriginX::Left);
    assert_eq!(origin.y, OriginY::Center);
}

#[test]
fn the_named_points_are_snake_case_on_the_wire() {
    let origin = origin_of(&with_clip(r#", "origin": { "x": "right", "y": "bottom" }"#));
    assert_eq!(
        origin,
        Origin {
            x: OriginX::Right,
            y: OriginY::Bottom
        }
    );
}

#[test]
fn a_point_that_does_not_exist_is_refused() {
    // Not silently centred: a clip asking to pivot on its "start" meant
    // something, and guessing which of the three it meant is worse than
    // saying no. So is a field nobody implements — `deny_unknown_fields`
    // catches an origin written as a pair of numbers.
    for body in [
        r#", "origin": { "x": "start" }"#,
        r#", "origin": { "x": 0.0, "y": 0.5 }"#,
        r#", "origin": { "across": "left" }"#,
    ] {
        let error = Project::from_json(&with_clip(body)).expect_err("an unknown origin fails");
        assert!(
            matches!(error, LoadError::Parse(_)),
            "{body}: got {error:?}"
        );
    }
}

#[test]
fn the_default_is_not_written_back_out() {
    let project = Project::from_json(&with_clip("")).expect("parses");
    let json = project.to_json().expect("serialise");
    assert!(
        !json.contains("\"origin\""),
        "a clip that never chose a pivot should not gain one on save: {json}"
    );
}

#[test]
fn a_chosen_point_round_trips() {
    let project = Project::from_json(&with_clip(r#", "origin": { "x": "left" }"#)).expect("parses");
    let json = project.to_json().expect("serialise");
    assert!(json.contains("\"left\""), "{json}");
    assert_eq!(Project::from_json(&json).expect("reparse"), project);
}

#[test]
fn an_origin_is_not_a_reason_to_refuse_a_project() {
    // It says nothing about coherence — every point is renderable, and an
    // audio clip carrying one is meaningless rather than wrong.
    let project = Project::from_json(&with_clip(r#", "origin": { "y": "top" }"#)).expect("parses");
    assert_eq!(project.validate(), Ok(()));
}
