//! Layers that are not the size of the canvas.
//!
//! Every layer used to be exactly the canvas's size, because the decoder scaled
//! it there. A clip asking for its source's native size breaks that, and where
//! such a layer *rests* — before any position keyframe moves it — is this
//! file's whole subject.

mod common;

use scorsese_compositor::{Layer, Properties};

use common::{BLACK, BLUE, CENTRE, CORNER, RED, assert_pixel, composited, pixel, solid, solid_of};

/// A 32x32 layer on the 64x64 canvas occupies 16..48 when it rests centred.
const SMALL: u32 = 32;
const FIRST: u32 = 16;
const LAST: u32 = 47;

#[test]
fn a_layer_smaller_than_the_canvas_rests_in_the_middle_of_it() {
    let badge = solid_of(RED, SMALL, SMALL);
    let canvas = composited(&[Layer::plain(&badge)]);

    assert_pixel(&canvas, CENTRE, RED, "the layer itself");
    assert_pixel(&canvas, CORNER, BLACK, "the canvas around it");
    // The edges exactly, not just the middle: resting one pixel out is the
    // failure this catches, and a centre sample cannot see it.
    assert_pixel(&canvas, (FIRST, FIRST), RED, "the layer's first pixel");
    assert_pixel(&canvas, (LAST, LAST), RED, "the layer's last pixel");
    assert_pixel(&canvas, (FIRST - 1, FIRST - 1), BLACK, "just outside it");
    assert_pixel(&canvas, (LAST + 1, LAST + 1), BLACK, "just past it");
}

#[test]
fn a_small_layer_leaves_the_tracks_below_it_showing() {
    // The difference between an overlay and a card: a native-size layer covers
    // the pixels it has and nothing else, so a bed under it stays visible.
    let bed = solid(BLUE);
    let badge = solid_of(RED, SMALL, SMALL);
    let canvas = composited(&[Layer::plain(&bed), Layer::plain(&badge)]);

    assert_pixel(&canvas, CENTRE, RED, "the badge");
    assert_pixel(&canvas, CORNER, BLUE, "the bed around it");
}

#[test]
fn position_offsets_a_small_layer_from_where_it_rests() {
    // Position is a fraction of the canvas, offsetting the layer from where it
    // rests — which for a native layer is the middle rather than the top-left.
    // An eighth of this 64-pixel canvas is eight pixels, so the arithmetic is
    // visible in the assertions rather than hidden in a rounder number.
    let badge = solid_of(RED, SMALL, SMALL);
    let canvas = composited(&[Layer {
        properties: Properties {
            position: (0.125, 0.0),
            ..Properties::default()
        },
        ..Layer::plain(&badge)
    }]);

    assert_pixel(&canvas, (FIRST + 8, CENTRE.1), RED, "moved right by eight");
    assert_pixel(&canvas, (FIRST + 7, CENTRE.1), BLACK, "vacated by the move");
    assert_pixel(&canvas, (LAST + 8, CENTRE.1), RED, "its trailing edge");
}

#[test]
fn a_layer_larger_than_the_canvas_is_clipped_by_it() {
    // Native size means native size, including when the source is bigger than
    // the raster. Drawing off the edges is the compositor's business, not a
    // reason to refuse the render.
    let huge = solid_of(RED, common::SIZE * 2, common::SIZE * 2);
    let canvas = composited(&[Layer::plain(&huge)]);

    assert_pixel(&canvas, CENTRE, RED, "the middle of the source");
    assert_pixel(&canvas, (0, 0), RED, "and every edge of the canvas");
    assert_pixel(
        &canvas,
        (common::SIZE - 1, common::SIZE - 1),
        RED,
        "as well",
    );
}

#[test]
fn an_odd_sized_layer_lands_on_whole_pixels_rather_than_between_them() {
    // 33 wide on a 64 canvas cannot be centred exactly. Rounding to a whole
    // pixel keeps the layer crisp; the half-pixel alternative would run every
    // edge through the bilinear filter and leave nothing exactly its own
    // colour — which is what this asserts is not happening.
    let odd = solid_of(RED, 33, 33);
    let canvas = composited(&[Layer::plain(&odd)]);

    assert_eq!(
        pixel(&canvas, CENTRE.0, CENTRE.1),
        (RED.0, RED.1, RED.2, u8::MAX),
        "an odd-sized layer should be exactly its own colour, not blended"
    );
}
