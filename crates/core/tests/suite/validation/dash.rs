//! A dashed border: what the pattern must be for there to be dashes at all.
//!
//! The pattern is the static half of a dashed line — where it sits along the
//! line is keyframed, and a keyframe is never validated against a property —
//! so this is everything the document can be held to about dashes.

use crate::common::{assert_only_problem, asset_id, problems, project};
use scorsese_core::{
    Asset, AssetProblem as E, Geometry, Rgba, Shape, ShapeProblem as S, ValidationError as V,
};

fn dashed(dash: Vec<f64>) -> scorsese_core::Project {
    let outline = Geometry::Rectangle {
        width: 0.3,
        height: 0.2,
        radius: 0.0,
    };
    let shape = Shape {
        dash: Some(dash),
        ..Shape::outlined(outline, Rgba::WHITE)
    };
    let mut p = project();
    p.assets.push(Asset::shape(asset_id("box"), shape));
    p
}

#[test]
fn a_pattern_of_positive_lengths_is_valid_odd_or_even() {
    assert_eq!(problems(&dashed(vec![0.02, 0.01])), vec![]);
    assert_eq!(problems(&dashed(vec![0.02])), vec![]);
    assert_eq!(problems(&dashed(vec![0.03, 0.01, 0.005])), vec![]);
}

/// An empty pattern says "dashed" and names no dash, so the only thing it could
/// draw is the solid line the document did not ask for.
#[test]
fn an_empty_pattern_is_refused() {
    assert_only_problem(
        &dashed(vec![]),
        E::Shape(S::BadDash {
            asset: asset_id("box"),
            dash: vec![],
        }),
    );
}

#[test]
fn a_length_of_zero_or_less_is_refused() {
    for dash in [vec![0.02, 0.0], vec![-0.01], vec![0.02, f64::INFINITY]] {
        let found = problems(&dashed(dash.clone()));
        assert!(
            matches!(found.as_slice(), [V::Asset(E::Shape(S::BadDash { .. }))]),
            "{dash:?}: {found:?}"
        );
    }
}

/// Absent is solid, and stays absent on the way back out — a document that
/// never mentioned dashes is not rewritten to mention them.
#[test]
fn the_pattern_round_trips_and_absent_is_not_written() {
    let shape: Shape = serde_json::from_str(
        r##"{ "geometry": { "ellipse": { "width": 0.2, "height": 0.2 } },
             "stroke": "#ffffffff", "dash": [0.02, 0.01] }"##,
    )
    .expect("a dashed shape parses");
    assert_eq!(shape.dash, Some(vec![0.02, 0.01]));
    let solid = Shape {
        dash: None,
        ..shape
    };
    let written = serde_json::to_string(&solid).expect("a shape writes");
    assert!(!written.contains("dash"), "{written}");
}
