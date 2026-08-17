//! Source-over: how a drawn layer's pixels land on ones that are already there.
//!
//! **Over an empty frame, source-over is a copy** — which is why nothing else in
//! this crate really asserts it. A shape test that finds a border solid blue has
//! asserted the border, and several different arithmetics all come out blue on a
//! transparent start. Over a bed that is already painted they do not: the alpha
//! that comes out, and each colour that comes out, is one number, and naming
//! those numbers is the only assertion the mixing itself has.
//!
//! A plain filled rectangle is what gets drawn, because none of this is about
//! geometry. The frame is not always empty in earnest — a slug card draws its
//! prompt over a background it filled, and a shape draws its border over its own
//! fill — so these are the real case rather than a contrived one.

mod common;

use common::{pixel, solid, translucent};
use scorsese_compositor::Frame;
use scorsese_compositor::shape::{Boxed, Figure, Outline, draw};
use scorsese_core::{Anchor, Rgba};

/// A quarter opaque: 64 of 255, so the source contributes 0.251 of every pixel
/// it lands on and whatever was underneath keeps 0.749 of it.
const QUARTER: u8 = 0x40;

/// The middle of the plate, well inside its edges, so no anti-aliased boundary
/// is being read.
const INSIDE: (u32, u32) = (32, 32);

/// A quarter of red over black keeps a quarter of its own red and none of the
/// bed's nothing: 0.251 × 255 = 64. The alpha is a quarter plus the three
/// quarters an opaque bed keeps, which is all of it.
#[test]
fn a_translucent_fill_over_an_opaque_bed_keeps_its_share_of_each() {
    let mut frame = solid((0x00, 0x00, 0x00));
    draw(&mut frame, &plate(rgba(0xff, 0x00, 0x00, QUARTER)));

    assert_levels(
        &frame,
        (0x40, 0x00, 0x00, 0xff),
        "a quarter of red over black",
    );
}

/// The same fill over white, which is the half the case above cannot see: the
/// red is already full so it cannot move, and the green and blue are three
/// quarters of the bed's 255 — 191 — arriving through the *other* term of the
/// mix.
#[test]
fn a_translucent_fill_over_a_light_bed_lets_three_quarters_of_it_through() {
    let mut frame = solid((0xff, 0xff, 0xff));
    draw(&mut frame, &plate(rgba(0xff, 0x00, 0x00, QUARTER)));

    assert_levels(
        &frame,
        (0xff, 0xbf, 0xbf, 0xff),
        "a quarter of red over white",
    );
}

/// And over a bed that is itself translucent, which is the only case where the
/// alpha out is neither of the two that went in: a quarter, plus half of the
/// three quarters left over, is 0.627 — 160 of 255. Every colour is then divided
/// by that, because a straight-alpha pixel states its colour as if it were
/// opaque.
#[test]
fn two_translucent_layers_come_out_at_the_alpha_they_add_up_to() {
    let mut frame = translucent((0xff, 0x00, 0x00), 0x80);
    draw(&mut frame, &plate(rgba(0x00, 0x00, 0xff, QUARTER)));

    assert_levels(
        &frame,
        (0x99, 0x00, 0x66, 0xa0),
        "a quarter of blue over half of red",
    );
}

/// A pixel to the level, allowing one for the rounding back to a byte. Every
/// mutation of the mixing this is here to notice moves a channel by tens.
#[track_caller]
fn assert_levels(frame: &Frame, expected: (u8, u8, u8, u8), what: &str) {
    let found = pixel(frame, INSIDE.0, INSIDE.1);
    let close = |a: u8, b: u8| a.abs_diff(b) <= 1;
    assert!(
        close(found.0, expected.0)
            && close(found.1, expected.1)
            && close(found.2, expected.2)
            && close(found.3, expected.3),
        "{what}: expected {expected:?}, found {found:?}"
    );
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba {
    Rgba::new(r, g, b, a)
}

/// Half the raster across and down, centred, so its middle is nowhere near an
/// edge.
fn plate(fill: Rgba) -> Figure {
    Figure {
        outline: Outline::Rectangle {
            bounds: Boxed {
                size: (32.0, 32.0),
                anchor: Anchor::default(),
            },
            radius: 0.0,
        },
        fill: Some(fill),
        border: None,
    }
}
