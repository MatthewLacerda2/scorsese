//! What an arrow document means, before anything draws one.
//!
//! Two defaults carry real weight here, and both are the kind of thing a
//! document leaves out precisely because it expects the obvious answer: an
//! arrow with no `curve` is straight, and one with no `heads` is pointed at the
//! end it runs to. Neither is stated in most projects, so neither is asserted
//! anywhere else.

use scorsese_core::{Curve, Geometry, Heads, Project, Shape};

/// One arrow asset, with the two optional fields left out.
const TERSE: &str = r##"{
  "schema_version": 29,
  "name": "arrows",
  "timeline_fps": { "num": 30, "den": 1 },
  "assets": [
    { "id": "a-to-b", "kind": "shape",
      "shape": {
        "geometry": { "arrow": { "from": { "x": 0.2, "y": 0.3 },
                                 "to":   { "x": 0.8, "y": 0.7 } } },
        "stroke": "#000000"
      } }
  ],
  "tracks": []
}"##;

fn only_shape(json: &str) -> Shape {
    let project = Project::from_json(json).expect("a project with one arrow in it");
    project.assets[0]
        .shape
        .clone()
        .expect("the arrow's shape block")
}

#[test]
fn an_arrow_with_nothing_said_about_it_is_straight_and_points_at_its_end() {
    let Geometry::Arrow { curve, heads, .. } = only_shape(TERSE).geometry else {
        panic!("the geometry parsed as something other than an arrow");
    };
    assert_eq!(curve, Curve::Straight);
    assert_eq!(heads, Heads::End);
}

#[test]
fn the_endpoints_are_read_as_written() {
    let Geometry::Arrow { from, to, .. } = only_shape(TERSE).geometry else {
        panic!("the geometry parsed as something other than an arrow");
    };
    let from = from.point().expect("a plain point, not an attachment");
    let to = to.point().expect("a plain point, not an attachment");
    assert_eq!((from.x, from.y), (0.2, 0.3));
    assert_eq!((to.x, to.y), (0.8, 0.7));
}

/// A line is an arrow that points at nothing, which is the whole reason `line`
/// is not a variant of its own.
#[test]
fn heads_none_is_how_a_plain_line_is_written() {
    let with_heads = TERSE.replace(
        r#""to":   { "x": 0.8, "y": 0.7 } } }"#,
        r#""to":   { "x": 0.8, "y": 0.7 }, "heads": "none" } }"#,
    );
    let Geometry::Arrow { heads, .. } = only_shape(&with_heads).geometry else {
        panic!("the geometry parsed as something other than an arrow");
    };
    assert_eq!(heads, Heads::None);
}

#[test]
fn a_bowed_arrow_says_so() {
    let bowed = TERSE.replace(
        r#""to":   { "x": 0.8, "y": 0.7 } } }"#,
        r#""to":   { "x": 0.8, "y": 0.7 }, "curve": "s" } }"#,
    );
    let Geometry::Arrow { curve, .. } = only_shape(&bowed).geometry else {
        panic!("the geometry parsed as something other than an arrow");
    };
    assert_eq!(curve, Curve::S);
}

/// The round trip has to survive the defaults being absent: a document that
/// said nothing about `curve` must not come back saying something about it that
/// its author would then be held to.
#[test]
fn an_arrow_round_trips() {
    let project = Project::from_json(TERSE).expect("parse");
    let again = Project::from_json(&project.to_json().expect("serialise")).expect("reparse");
    assert_eq!(project, again);
}
