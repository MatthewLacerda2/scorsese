//! A gradient fill: what its stops and geometry must be for it to paint.
//!
//! Checked the same way wherever a fill is written — a shape's `fill` and a
//! colour asset's `color` — and reported against whichever field it was.

use crate::common::{assert_only_problem, asset_id, problems, project};
use scorsese_core::{
    Asset, AssetProblem as E, Fill, FillProblem as G, Geometry, Linear, Point, Project, Radial,
    Rgba, Shape, Stop,
};

const DARK: Rgba = Rgba::opaque(0x0b, 0x10, 0x20);
const LIGHT: Rgba = Rgba::opaque(0x1b, 0x24, 0x40);

fn stops(offsets: &[f64]) -> Vec<Stop> {
    offsets
        .iter()
        .enumerate()
        .map(|(i, &at)| Stop::new(if i % 2 == 0 { DARK } else { LIGHT }, at))
        .collect()
}

fn linear(offsets: &[f64]) -> Fill {
    Fill::Linear(Linear {
        angle: 90.0,
        stops: stops(offsets),
    })
}

fn radial(center: Point, radius: f64) -> Fill {
    Fill::Radial(Radial {
        center,
        radius,
        stops: stops(&[0.0, 1.0]),
    })
}

fn boxed(fill: Fill) -> Project {
    let outline = Geometry::Rectangle {
        width: 0.4,
        height: 0.1,
        radius: 0.5,
    };
    let mut p = project();
    p.assets
        .push(Asset::shape(asset_id("pill"), Shape::filled(outline, fill)));
    p
}

fn backdrop(fill: Fill) -> Project {
    let mut p = project();
    p.assets.push(Asset::color(asset_id("bg"), fill));
    p
}

#[test]
fn a_well_formed_gradient_is_valid_on_a_shape_and_on_a_backdrop() {
    assert_eq!(problems(&boxed(linear(&[0.0, 1.0]))), vec![]);
    assert_eq!(problems(&boxed(linear(&[0.0, 0.5, 0.5, 1.0]))), vec![]);
    assert_eq!(
        problems(&backdrop(radial(Point::new(0.5, 0.45), 0.8))),
        vec![]
    );
}

#[test]
fn one_stop_is_a_colour_written_the_long_way_and_is_refused() {
    assert_only_problem(
        &boxed(linear(&[0.3])),
        E::Fill(G::TooFewStops {
            asset: asset_id("pill"),
            field: "fill",
            count: 1,
        }),
    );
}

#[test]
fn stops_that_go_backwards_or_leave_the_unit_range_are_refused() {
    for offsets in [[0.0, 0.8, 0.4], [0.0, 0.5, 1.5], [-0.1, 0.5, 1.0]] {
        assert_only_problem(
            &backdrop(linear(&offsets)),
            E::Fill(G::StopsOutOfOrder {
                asset: asset_id("bg"),
                field: "color",
                offsets: offsets.to_vec(),
            }),
        );
    }
}

#[test]
fn a_circle_with_no_size_or_no_centre_is_refused() {
    for (center, radius) in [
        (Point::new(0.5, 0.5), 0.0),
        (Point::new(0.5, 0.5), -1.0),
        (Point::new(f64::NAN, 0.5), 0.5),
    ] {
        let found = problems(&backdrop(radial(center, radius)));
        assert!(
            matches!(
                found.as_slice(),
                [scorsese_core::ValidationError::Asset(E::Fill(
                    G::BadCircle { .. }
                ))]
            ),
            "{center:?} r={radius}: {found:?}"
        );
    }
}

#[test]
fn an_angle_that_is_not_a_number_is_refused() {
    let fill = Fill::Linear(Linear {
        angle: f64::INFINITY,
        stops: stops(&[0.0, 1.0]),
    });
    assert_only_problem(
        &boxed(fill),
        E::Fill(G::BadAngle {
            asset: asset_id("pill"),
            field: "fill",
            angle: f64::INFINITY,
        }),
    );
}
