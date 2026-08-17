//! Putting a symbol on a raster.

use scorsese_compositor::icon::{self, Symbol};
use scorsese_core::{Anchor, AnchorX, AnchorY};

use crate::{centred, frame, ink, ink_in, symbol};

/// **The one that matters.** Every vendored icon decodes to commands and puts
/// ink on a raster. With seventeen hundred of them the failure mode is one
/// malformed entry nobody ever looks at, and it should be this suite that finds
/// it rather than a render.
#[test]
fn every_icon_draws_something() {
    let size = f32::from(u16::try_from(crate::SIDE).expect("a small raster")) * 0.75;
    for found in icon::all() {
        let commands = found.commands().expect("contours that decode");
        assert!(commands > 0, "{}: draws nothing at all", found.name());

        let mut frame = frame();
        icon::draw(
            &mut frame,
            &Symbol {
                icon: found,
                size,
                anchor: centred(),
                color: crate::WHITE,
                width: icon::width_for(size),
            },
        );
        assert!(ink(&frame) > 0, "{}: drew no pixels", found.name());
    }
}

/// The anchor decides where the square lands, the same way it does for a
/// closed shape — an icon layer is raster-sized, so nothing downstream would
/// place it if this did not.
#[test]
fn the_anchor_places_the_square() {
    let corner = Anchor {
        x: AnchorX::Left,
        y: AnchorY::Top,
    };
    // A quarter of the raster, so the square fits inside one quadrant and the
    // other three say where it is not.
    let size = f32::from(u16::try_from(crate::SIDE / 2).expect("a small raster"));
    let mut frame = frame();
    icon::draw(
        &mut frame,
        &Symbol {
            size,
            width: icon::width_for(size),
            ..symbol("square", corner)
        },
    );
    assert!(ink_in(&frame, (false, false)) > 0, "in the top-left");
    assert_eq!(ink_in(&frame, (true, true)), 0, "and nowhere else");
    assert_eq!(ink_in(&frame, (true, false)), 0, "nor across the middle");
}

/// A centred symbol is centred on both axes, which is worth its own assertion:
/// the two are separate arithmetic and a swapped pair reads as *nearly right*.
#[test]
fn a_centred_symbol_is_even_on_both_axes() {
    let mut frame = frame();
    icon::draw(&mut frame, &symbol("square", centred()));
    let quadrants = [
        ink_in(&frame, (false, false)),
        ink_in(&frame, (true, false)),
        ink_in(&frame, (false, true)),
        ink_in(&frame, (true, true)),
    ];
    let first = quadrants[0];
    assert!(first > 0, "the square is drawn");
    assert!(
        quadrants.iter().all(|q| q.abs_diff(first) <= 2),
        "the same in each quarter: {quadrants:?}"
    );
}

/// Size is in pixels of the raster, so a bigger symbol is more ink. Nothing
/// about the drawing is resolution-dependent — that is the argument for a
/// vector symbol over an imported PNG.
#[test]
fn a_bigger_symbol_covers_more() {
    let ink_at = |size: f32| {
        let mut frame = frame();
        icon::draw(
            &mut frame,
            &Symbol {
                size,
                width: icon::width_for(size),
                ..symbol("circle", centred())
            },
        );
        ink(&frame)
    };
    assert!(ink_at(32.0) > ink_at(16.0), "a larger circle is more ink");
}

/// The thickness is the caller's, and a width that could not draw a line draws
/// nothing rather than a default nobody asked for — the rule a border already
/// follows.
#[test]
fn the_stroke_width_is_the_callers() {
    let ink_at = |width: f32| {
        let mut frame = frame();
        icon::draw(
            &mut frame,
            &Symbol {
                width,
                ..symbol("circle", centred())
            },
        );
        ink(&frame)
    };
    assert!(ink_at(6.0) > ink_at(2.0), "a thicker line is more ink");
    assert_eq!(ink_at(0.0), 0, "no width, no line");
    assert_eq!(ink_at(f32::NAN), 0, "nor for a width that is not a number");
}

/// A size that could not describe a square draws nothing, for the same reason
/// a shape with no area does: there is no honest picture to invent.
#[test]
fn a_symbol_with_no_size_draws_nothing() {
    for size in [0.0, -12.0, f32::INFINITY] {
        let mut frame = frame();
        icon::draw(
            &mut frame,
            &Symbol {
                size,
                ..symbol("circle", centred())
            },
        );
        assert_eq!(ink(&frame), 0, "a size of {size}");
    }
}

/// The dots are filled rather than stroked, so an icon that has one draws a
/// solid centre where a stroked circle of the same radius would leave a hole.
#[test]
fn a_dotted_icon_fills_its_dots() {
    let mut frame = frame();
    icon::draw(&mut frame, &symbol("dice-5", centred()));
    assert!(ink(&frame) > 0, "the pips and the box");
}
