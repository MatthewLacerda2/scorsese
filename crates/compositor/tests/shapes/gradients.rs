//! A gradient fill: laid across the shape's own box, and dithered.
//!
//! Sampled rather than measured — what is being claimed is which colour lands
//! where, and a colour is a pixel's to say.

use scorsese_compositor::shape::{self, Figure, Outline};
use scorsese_core::{Fill, Linear, Point, Radial, Rgba, Stop};

use super::{RED, at, bounds, centred, clear, frame};

const BLACK: Rgba = Rgba::opaque(0, 0, 0);
const WHITE: Rgba = Rgba::opaque(0xff, 0xff, 0xff);

fn stops(from: Rgba, to: Rgba) -> Vec<Stop> {
    vec![Stop::new(from, 0.0), Stop::new(to, 1.0)]
}

/// A 100×40 box in the middle of the raster, painted `fill`.
fn painted(fill: Fill) -> scorsese_compositor::Frame {
    let mut frame = frame();
    let figure = Figure {
        outline: Outline::Rectangle {
            bounds: bounds((100.0, 40.0), centred()),
            radius: 0.0,
        },
        fill: Some(fill),
        border: None,
    };
    shape::draw(&mut frame, &figure);
    frame
}

/// The box spans x 50..150 (pixel centres 89.5 and 110.5 are mirror images) and y 80..120 on the 200-pixel raster.
#[test]
fn a_linear_gradient_runs_across_the_shapes_own_box_not_the_frame() {
    let frame = painted(Fill::Linear(Linear {
        angle: 90.0,
        stops: stops(BLACK, WHITE),
    }));
    let red = |x| at(&frame, x, 100).0;
    assert!(red(50) <= 4, "the left edge is the first stop: {}", red(50));
    assert!(red(149) >= 251, "the right edge is the last: {}", red(149));
    assert!(
        (125..=131).contains(&red(100)),
        "halfway is half: {}",
        red(100)
    );
    let (upper, lower) = (at(&frame, 100, 85).0, at(&frame, 100, 115).0);
    assert!(
        upper.abs_diff(lower) <= 4,
        "90° does not vary down: {upper} {lower}"
    );
    assert!(
        clear(&frame, 49, 100) && clear(&frame, 150, 100),
        "nothing past the box"
    );
    assert!(clear(&frame, 100, 79), "nor above it");
}

/// 180° runs top to bottom, the CSS way round.
#[test]
fn the_angle_is_the_way_the_colours_travel() {
    let frame = painted(Fill::Linear(Linear {
        angle: 180.0,
        stops: stops(BLACK, WHITE),
    }));
    assert!(at(&frame, 100, 80).0 <= 4, "black at the top");
    assert!(at(&frame, 100, 119).0 >= 251, "white at the bottom");
}

/// The radius is of the box's shorter side, so on a 100×40 box a radius of
/// 0.5 is 20 pixels: the last stop is reached at the top edge from the middle,
/// and carries on to the ends.
#[test]
fn a_radial_gradient_spreads_from_its_centre_in_circles() {
    let frame = painted(Fill::Radial(Radial {
        center: Point::new(0.5, 0.5),
        radius: 0.5,
        stops: stops(WHITE, BLACK),
    }));
    assert!(at(&frame, 100, 100).0 >= 240, "near white in the middle");
    assert!(at(&frame, 100, 80).0 <= 20, "all but black 20px up");
    assert_eq!(at(&frame, 60, 100).0, 0, "black past the radius");
    let (left, right) = (at(&frame, 89, 100), at(&frame, 110, 100));
    assert!(
        left.0.abs_diff(right.0) <= 1,
        "symmetric: {left:?} {right:?}"
    );
}

/// Two stops of one colour must come out as that colour everywhere — the
/// dither never moves a value that sits exactly on a level.
#[test]
fn a_gradient_between_equal_colours_is_flat_and_exact() {
    let frame = painted(Fill::Linear(Linear {
        angle: 30.0,
        stops: stops(RED, RED),
    }));
    for x in 50..150 {
        assert_eq!(at(&frame, x, 100), (0xff, 0, 0, 0xff), "column {x}");
    }
}

/// The dithering claim itself: a ramp one level deep across the box is not a
/// hard step in the middle, but noise whose average follows the ramp — so a
/// strip of the box averages to the value underneath it, not to a level.
#[test]
fn a_shallow_ramp_is_dithered_into_noise_that_follows_it() {
    let frame = painted(Fill::Linear(Linear {
        angle: 90.0,
        stops: stops(Rgba::opaque(16, 16, 16), Rgba::opaque(17, 17, 17)),
    }));
    for strip in 0..10 {
        let left = 50 + strip * 10;
        let pixels = (left..left + 10).flat_map(|x| (80..120).map(move |y| (x, y)));
        let total: f64 = pixels.map(|(x, y)| f64::from(at(&frame, x, y).0)).sum();
        let mean = total / 400.0 - 16.0;
        let ramp = (f64::from(left - 50) + 5.0) / 100.0;
        assert!(
            (mean - ramp).abs() <= 0.12,
            "strip at x {left} averages {mean:.3} above the first stop; the ramp says {ramp:.3}"
        );
    }
}
