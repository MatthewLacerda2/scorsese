//! Stacking layers, and the canvas they stack onto.

mod common;

use scorsese_compositor::{Layer, Properties};

use common::{BLACK, BLUE, CENTRE, CORNER, RED, assert_pixel, composited, solid, with};

#[test]
fn no_layers_is_black() {
    // A gap in a timeline composites nothing, and nothing is what black means.
    let canvas = composited(&[]);
    assert_pixel(&canvas, CENTRE, BLACK, "an empty canvas");
    assert_pixel(&canvas, (0, 0), BLACK, "an empty canvas");
}

#[test]
fn an_untransformed_opaque_layer_arrives_byte_for_byte() {
    // The plain-cut case. Asserting the *bytes* rather than a colour matters:
    // it is what lets a render with no effects stay identical to one that never
    // went near a compositor, so existing golden references keep their meaning.
    let source = solid(RED);
    let canvas = composited(&[Layer::plain(&source)]);
    assert_eq!(canvas.bytes(), source.bytes());
}

#[test]
fn layers_stack_with_the_first_at_the_bottom() {
    // Matching the order video tracks appear in a project: earlier is lower.
    let bottom = solid(RED);
    let top = solid(BLUE);
    let canvas = composited(&[
        Layer::plain(&bottom),
        with(
            &top,
            Properties {
                scale: (0.5, 0.5),
                ..Properties::default()
            },
        ),
    ]);
    assert_pixel(&canvas, CENTRE, BLUE, "the upper layer covers");
    assert_pixel(&canvas, CORNER, RED, "the lower shows around it");
}

#[test]
fn an_upper_layers_opacity_blends_with_the_one_below() {
    let bottom = solid(RED);
    let top = solid(BLUE);
    let canvas = composited(&[
        Layer::plain(&bottom),
        with(
            &top,
            Properties {
                opacity: 0.5,
                ..Properties::default()
            },
        ),
    ]);
    assert_pixel(&canvas, CENTRE, (128, 0, 128), "half blue on red");
}
