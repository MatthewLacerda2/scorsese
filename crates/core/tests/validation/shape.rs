//! What a `shape` asset must and must not carry.
//!
//! Everything checkable about a shape is checkable from the document — it has
//! no file, and its geometry is fractions — so these are the whole of what
//! validation can say, rather than the document half of a question the render
//! finishes.

use crate::common::{assert_only_problem, asset_id, asset_mut, problems, project};
use scorsese_core::{
    Asset, AssetField as F, AssetKind, AssetProblem as E, DEFAULT_STROKE_WIDTH, Geometry,
    MAX_RADIUS, Rgba, Shape, ShapeProblem as S, ValidationError as V,
};

fn box_of(width: f64, height: f64) -> Geometry {
    Geometry::Rectangle {
        width,
        height,
        radius: 0.0,
    }
}

/// A shape asset in an otherwise valid project, built by the constructor a
/// caller actually reaches for.
///
/// **Nothing here writes over the block the constructor attached**, and that is
/// the point of the shape of it: a helper that assembled the asset field by
/// field would leave `Asset::shape` asserted by nothing at all, and it could
/// stop attaching a shape with every test in this file still green.
fn drawn(shape: Shape) -> scorsese_core::Project {
    let mut p = project();
    p.assets.push(Asset::shape(asset_id("box"), shape));
    p
}

/// The one case that has to undo what the constructor did, and the only one.
fn without_a_shape() -> scorsese_core::Project {
    let mut p = drawn(Shape::filled(box_of(0.2, 0.1), Rgba::WHITE));
    asset_mut(&mut p, "box").shape = None;
    p
}

#[test]
fn a_shape_asset_is_valid_on_its_own() {
    assert_eq!(
        drawn(Shape::filled(box_of(0.2, 0.1), Rgba::WHITE)).validate(),
        Ok(())
    );
}

#[test]
fn a_shape_asset_needs_a_shape() {
    assert_only_problem(
        &without_a_shape(),
        E::MissingField {
            asset: asset_id("box"),
            field: F::Shape,
            kind: AssetKind::Shape,
        },
    );
}

#[test]
fn nothing_but_a_shape_asset_may_carry_a_shape() {
    let mut p = project();
    asset_mut(&mut p, "logo").shape = Some(Shape::filled(box_of(0.2, 0.1), Rgba::WHITE));
    assert_only_problem(
        &p,
        E::StrayField {
            asset: asset_id("logo"),
            field: F::Shape,
            kind: AssetKind::Image,
        },
    );
}

/// The inline kinds are the ones with nothing on disk, and a shape is the
/// third. Getting this wrong would demand a `path` for something that is four
/// numbers in the document.
#[test]
fn a_shape_asset_needs_no_path() {
    let found = problems(&drawn(Shape::filled(box_of(0.2, 0.1), Rgba::WHITE)));
    assert!(
        found.is_empty(),
        "a shape has no file behind it, so nothing should ask for one: {found:?}"
    );
}

#[test]
fn a_shape_needs_area() {
    assert_only_problem(
        &drawn(Shape::filled(box_of(0.2, 0.0), Rgba::WHITE)),
        E::Shape(S::NotSized {
            asset: asset_id("box"),
            width: 0.2,
            height: 0.0,
        }),
    );
}

/// A width that is not a number is the case a naive `> 0.0` check waves
/// through, and it arrives from arithmetic rather than from anyone typing it.
#[test]
fn a_size_that_is_not_a_number_is_refused() {
    let found = problems(&drawn(Shape::filled(box_of(f64::NAN, 0.1), Rgba::WHITE)));
    assert!(
        matches!(found.as_slice(), [V::Asset(E::Shape(S::NotSized { .. }))]),
        "a NaN width is not a size: {found:?}"
    );
}

#[test]
fn a_corner_cannot_be_rounder_than_the_shape() {
    let geometry = Geometry::Rectangle {
        width: 0.2,
        height: 0.1,
        radius: 0.75,
    };
    assert_only_problem(
        &drawn(Shape::filled(geometry, Rgba::WHITE)),
        E::Shape(S::RadiusOutOfRange {
            asset: asset_id("box"),
            radius: 0.75,
            max: MAX_RADIUS,
        }),
    );
}

#[test]
fn a_border_colour_needs_a_width() {
    assert_only_problem(
        &drawn(Shape::filled(box_of(0.2, 0.1), Rgba::WHITE).bordered(Rgba::BLACK, 0.0)),
        E::Shape(S::BorderWithoutWidth {
            asset: asset_id("box"),
            width: 0.0,
        }),
    );
}

/// The one a render could never report: an invisible layer and a layer that
/// failed to draw look identical in the finished frame.
#[test]
fn a_shape_with_neither_fill_nor_border_is_refused() {
    let shape = Shape {
        geometry: box_of(0.2, 0.1),
        fill: None,
        stroke: None,
        stroke_width: 0.004,
    };
    let invisible = S::Invisible {
        asset: asset_id("box"),
    };
    assert_only_problem(&drawn(shape), E::Shape(invisible));
}

#[test]
fn a_border_over_no_fill_is_the_callout_case_and_is_fine() {
    assert_eq!(
        drawn(Shape::outlined(box_of(0.2, 0.1), Rgba::BLACK)).validate(),
        Ok(())
    );
}

/// The width a document gets by leaving `stroke_width` out, which nothing else
/// here can see: every constructor sets it, so only a parse reads the default.
/// Unasserted it could be `0.0` — and a border of no width draws nothing, which
/// is the one failure a finished render cannot report, since an invisible layer
/// and a layer that failed look identical.
#[test]
fn an_omitted_border_width_is_four_thousandths_of_the_height() {
    let json = r##"{ "id": "box", "kind": "shape",
                     "shape": { "geometry": { "rectangle": { "width": 0.2, "height": 0.1 } },
                                "stroke": "#000000ff" } }"##;
    let asset: Asset = serde_json::from_str(json).expect("a shape asset parses");
    let shape = asset.shape.expect("the block is there");
    assert_eq!(shape.stroke_width, DEFAULT_STROKE_WIDTH);
    assert!((shape.stroke_width - 0.004).abs() < f64::EPSILON);
    assert_eq!(
        drawn(shape).validate(),
        Ok(()),
        "and a width validation accepts, which zero would not be"
    );
}
