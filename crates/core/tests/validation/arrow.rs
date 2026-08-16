//! What an arrow must and must not be.
//!
//! An arrow answers different questions from a box, and the difference is the
//! reason these are apart from `shape.rs`: it has no size to be zero and no
//! corner to over-round, and it has two endpoints instead — which can be in the
//! same place, or not be numbers at all.
//!
//! Note what is **not** refused: a point outside the frame. An arrow entering
//! from off-screen is an ordinary thing to draw, and refusing it would be
//! mistaking the raster for the coordinate space.

use crate::common::{assert_only_problem, asset_id, problems, project};
use scorsese_core::{
    Asset, AssetProblem as E, Curve, Geometry, Heads, Point, Rgba, Shape, ShapeProblem as S,
};

fn arrow(from: Point, to: Point) -> Geometry {
    Geometry::Arrow {
        from: from.into(),
        to: to.into(),
        curve: Curve::Straight,
        heads: Heads::End,
    }
}

/// An arrow in an otherwise valid project, drawn in whatever way the test is
/// about.
fn drawn(shape: Shape) -> scorsese_core::Project {
    let mut p = project();
    p.assets.push(Asset::shape(asset_id("a-to-b"), shape));
    p
}

fn plain() -> Shape {
    Shape::outlined(
        arrow(Point::new(0.2, 0.3), Point::new(0.8, 0.7)),
        Rgba::BLACK,
    )
}

#[test]
fn an_arrow_with_a_stroke_is_valid_on_its_own() {
    assert_eq!(drawn(plain()).validate(), Ok(()));
}

/// A line encloses nothing, so a colour for its inside is a colour nothing
/// would read — usually a `stroke` that was meant.
#[test]
fn an_arrow_may_not_be_filled() {
    let shape = Shape {
        fill: Some(Rgba::WHITE),
        ..plain()
    };
    assert_only_problem(
        &drawn(shape),
        E::Shape(S::FilledLine {
            asset: asset_id("a-to-b"),
        }),
    );
}

/// Both at once, because the fill is not a stand-in for the missing stroke: the
/// arrow is both wrongly described *and* invisible, and an agent repairing it
/// needs to be told the second thing as well as the first.
#[test]
fn an_arrow_with_only_a_fill_is_both_wrong_and_invisible() {
    let shape = Shape::filled(
        arrow(Point::new(0.2, 0.3), Point::new(0.8, 0.7)),
        Rgba::WHITE,
    );
    let found = problems(&drawn(shape));
    assert_eq!(found.len(), 2, "both findings, not one: {found:?}");
}

#[test]
fn an_arrow_without_a_stroke_draws_nothing() {
    let shape = Shape {
        stroke: None,
        ..plain()
    };
    assert_only_problem(
        &drawn(shape),
        E::Shape(S::Invisible {
            asset: asset_id("a-to-b"),
        }),
    );
}

/// Two ends in one place have no length to draw and no direction to aim a head
/// along.
#[test]
fn an_arrow_needs_two_different_ends() {
    let at = Point::new(0.4, 0.4);
    let shape = Shape::outlined(arrow(at, at), Rgba::BLACK);
    assert_only_problem(
        &drawn(shape),
        E::Shape(S::NoLength {
            asset: asset_id("a-to-b"),
            at,
        }),
    );
}

#[test]
fn an_endpoint_that_is_not_a_number_is_refused() {
    let from = Point::new(f64::NAN, 0.3);
    let to = Point::new(0.8, 0.7);
    let shape = Shape::outlined(arrow(from, to), Rgba::BLACK);
    let found = problems(&drawn(shape));
    assert!(
        matches!(found.as_slice(), [_] if format!("{found:?}").contains("Unplaced")),
        "one finding, about the endpoint: {found:?}"
    );
}

/// The case that must stay legal: an arrow flying in from off the edge of the
/// frame.
#[test]
fn an_endpoint_outside_the_frame_is_fine() {
    let shape = Shape::outlined(
        arrow(Point::new(-0.4, 0.5), Point::new(0.5, 1.6)),
        Rgba::BLACK,
    );
    assert_eq!(drawn(shape).validate(), Ok(()));
}
