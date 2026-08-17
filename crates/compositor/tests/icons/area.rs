//! The rectangle a symbol occupies, which is read rather than drawn.
//!
//! `icon::area_of` is the icon's own square — not the raster-sized layer it is
//! drawn into — and it has two consumers that never look at a pixel: an arrow
//! attaches to it, and `project_check` warns when two layers land on top of each
//! other. A wrong square is a wrong warning, so this is the only place that can
//! object to one.

use scorsese_compositor::Resolution;
use scorsese_compositor::icon::{self, Symbol};
use scorsese_core::{Anchor, AnchorX, AnchorY};

use crate::extent::assert_extent;
use crate::{SIDE, centred, frame, symbol};

/// The size `symbol` builds: three quarters of the 48-pixel raster, which
/// leaves 6 pixels either side of a centred square and 12 on the far side of an
/// anchored one.
const SIZE: f32 = 36.0;

fn raster() -> Resolution {
    Resolution::new(SIDE, SIDE).expect("a square raster")
}

/// The box, as `(left, top, width, height)`.
fn boxed(anchor: Anchor) -> (f32, f32, f32, f32) {
    let area = icon::area_of(&symbol("square", anchor), raster()).expect("a square has a box");
    (area.left, area.top, area.width, area.height)
}

/// Each corner separately: the two axes are separate arithmetic, and a square
/// symbol in a square frame hides a crossed pair from anything that checks one
/// number.
#[test]
fn the_box_is_the_square_the_anchor_places() {
    assert_eq!(
        boxed(centred()),
        (6.0, 6.0, SIZE, SIZE),
        "centred on a 48 raster leaves 6 either side"
    );
    assert_eq!(
        boxed(Anchor {
            x: AnchorX::Left,
            y: AnchorY::Top
        }),
        (0.0, 0.0, SIZE, SIZE),
        "hard against the left and the top"
    );
    assert_eq!(
        boxed(Anchor {
            x: AnchorX::Right,
            y: AnchorY::Bottom
        }),
        (12.0, 12.0, SIZE, SIZE),
        "and against the right and the bottom"
    );
    assert_eq!(
        boxed(Anchor {
            x: AnchorX::Right,
            y: AnchorY::Top
        }),
        (12.0, 0.0, SIZE, SIZE),
        "one axis from each"
    );
}

/// A size that could not describe a square has no box at all, which is what a
/// warning about an overlap has to be able to tell from a box at the origin.
#[test]
fn a_size_that_is_not_a_square_has_no_box() {
    for size in [0.0, -12.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            icon::area_of(
                &Symbol {
                    size,
                    ..symbol("square", centred())
                },
                raster()
            ),
            None,
            "a size of {size}"
        );
    }
}

/// The box and the drawing agree, which is the claim the two consumers rest on.
/// `square` is Lucide's rounded rectangle from 3 to 21 of the 24-unit box, so at
/// three quarters of a 48-pixel raster its ink runs from 9 to 39 — inside the
/// box on every edge, and by the half-stroke that a border straddling the
/// outline costs.
#[test]
fn the_drawing_lands_inside_the_box_the_render_is_told_about() {
    let symbol = symbol("square", centred());
    let area = icon::area_of(&symbol, raster()).expect("a square has a box");
    assert_eq!((area.left, area.top), (6.0, 6.0), "the box");

    let mut frame = frame();
    icon::draw(&mut frame, &symbol);
    assert_extent(
        &frame,
        (9, 9, 38, 38),
        "3 and 21 of 24, scaled by 1.5, placed at 6, and a stroke of 1.5 either side",
    );
}
