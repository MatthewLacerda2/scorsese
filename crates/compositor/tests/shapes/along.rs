//! Walking an outline by distance — the arc-length groundwork a trim is cut
//! with and a clip following a path is placed by.
//!
//! Lengths are checked against what geometry says on paper: a straight run is
//! its endpoint distance, a circle is 2πr, a box is its perimeter. The
//! flattening that gets there is an implementation choice, and these are the
//! numbers it has to arrive at whatever it chooses.

use std::f32::consts::PI;

use scorsese_compositor::Resolution;
use scorsese_compositor::shape::{Arrow, Measured, Outline, measure};
use scorsese_core::{Curve, Heads};

use crate::{SIDE, bounds, centred};

fn raster() -> Resolution {
    Resolution::new(SIDE, SIDE).expect("a square raster")
}

fn arrow(from: (f32, f32), to: (f32, f32), curve: Curve) -> Measured {
    let outline = Outline::Arrow(Arrow {
        from,
        to,
        curve,
        heads: Heads::End,
    });
    measure(&outline, raster()).expect("an arrow with length")
}

fn close(a: (f32, f32), b: (f32, f32), within: f32) -> bool {
    (a.0 - b.0).abs() <= within && (a.1 - b.1).abs() <= within
}

#[test]
fn a_straight_arrow_is_as_long_as_its_ends_are_apart_and_runs_evenly() {
    let line = arrow((40.0, 40.0), (160.0, 130.0), Curve::Straight);
    assert!((line.length() - 150.0).abs() < 1e-3, "{}", line.length());
    assert!(!line.is_closed());
    assert!(close(line.at(0.0).position, (40.0, 40.0), 1e-3));
    assert!(close(line.at(0.5).position, (100.0, 85.0), 1e-3));
    assert!(close(line.at(1.0).position, (160.0, 130.0), 1e-3));
    assert!(close(line.at(0.3).tangent, (0.8, 0.6), 1e-4));
}

#[test]
fn a_circle_is_two_pi_r_long_and_starts_at_its_rightmost_point_going_clockwise() {
    let outline = Outline::Ellipse(bounds((100.0, 100.0), centred()));
    let circle = measure(&outline, raster()).expect("a circle has length");
    let expected = 2.0 * PI * 50.0;
    assert!(
        (circle.length() - expected).abs() / expected < 0.005,
        "{} against {expected}",
        circle.length()
    );
    assert!(circle.is_closed());
    // Rightmost, then a quarter of the way round is the bottom: clockwise on a
    // raster whose y runs down.
    assert!(close(circle.at(0.0).position, (150.0, 100.0), 0.5));
    assert!(close(circle.at(0.25).position, (100.0, 150.0), 0.5));
    assert!(close(circle.at(0.0).tangent, (0.0, 1.0), 0.05));
}

#[test]
fn a_box_is_its_perimeter_and_starts_at_its_top_left_corner() {
    let square = Outline::Rectangle {
        bounds: bounds((120.0, 60.0), centred()),
        radius: 0.0,
    };
    let measured = measure(&square, raster()).expect("a box has length");
    assert!((measured.length() - 360.0).abs() < 1e-3);
    assert!(close(measured.at(0.0).position, (40.0, 70.0), 1e-3));
    // The whole top edge is a third of the way round.
    assert!(close(measured.at(1.0 / 3.0).position, (160.0, 70.0), 1e-3));

    let rounded = Outline::Rectangle {
        bounds: bounds((120.0, 60.0), centred()),
        radius: 20.0,
    };
    let expected = 360.0 - 8.0 * 20.0 + 2.0 * PI * 20.0;
    let length = measure(&rounded, raster()).expect("rounded").length();
    assert!((length - expected).abs() / expected < 0.005, "{length}");
}

/// The point of measuring by distance: an S spends its curve parameter
/// unevenly, so equal steps of it are unequal steps along the line. Equal
/// fractions here have to be equal distances.
#[test]
fn equal_fractions_of_an_s_are_equal_distances_along_it() {
    let bowed = arrow((20.0, 40.0), (180.0, 160.0), Curve::S);
    let steps: Vec<f32> = (0..50)
        .map(|i| {
            let (a, b) = (bowed.at(i as f32 / 50.0), bowed.at((i + 1) as f32 / 50.0));
            (b.position.0 - a.position.0).hypot(b.position.1 - a.position.1)
        })
        .collect();
    let (least, most) = steps
        .iter()
        .fold((f32::MAX, 0.0_f32), |(l, m), s| (l.min(*s), m.max(*s)));
    assert!(most / least < 1.02, "steps from {least} to {most}");
    // It leaves and arrives along the axis the ends are furthest apart on — to
    // within the first straight piece's chord, which is what a flattened
    // outline's heading at an end is.
    assert!(close(bowed.at(0.0).tangent, (1.0, 0.0), 0.05));
    assert!(close(bowed.at(1.0).tangent, (1.0, 0.0), 0.05));
    // And an S is symmetric about its middle.
    assert!(close(bowed.at(0.5).position, (100.0, 100.0), 0.5));
}

#[test]
fn a_fraction_outside_the_outline_is_clamped_and_a_heading_is_clockwise_degrees() {
    let line = arrow((100.0, 20.0), (100.0, 180.0), Curve::Straight);
    assert_eq!(line.at(-1.0), line.at(0.0));
    assert_eq!(line.at(3.0), line.at(1.0));
    assert_eq!(line.at(f32::NAN), line.at(0.0));
    // Straight down the raster is a quarter turn clockwise from pointing right.
    assert!((line.at(0.5).heading() - 90.0).abs() < 1e-3);
}

#[test]
fn an_outline_with_no_length_cannot_be_measured() {
    let point = Outline::Arrow(Arrow {
        from: (50.0, 50.0),
        to: (50.0, 50.0),
        curve: Curve::Straight,
        heads: Heads::None,
    });
    assert_eq!(measure(&point, raster()), None);
    let flat = Outline::Ellipse(bounds((0.0, 40.0), centred()));
    assert_eq!(measure(&flat, raster()), None);
}
