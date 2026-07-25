//! What position, scale, and opacity do to a single layer.

mod common;

use scorsese_compositor::{Layer, Properties};

use common::{
    BLACK, CENTRE, CORNER, HALF_RED, RED, SIZE, assert_pixel, composited, solid, translucent, with,
};

#[test]
fn opacity_blends_towards_the_black_beneath() {
    let source = solid(RED);
    let canvas = composited(&[with(
        &source,
        Properties {
            opacity: 0.5,
            ..Properties::default()
        },
    )]);
    assert_pixel(&canvas, CENTRE, HALF_RED, "half-opaque red");
}

#[test]
fn a_fully_transparent_layer_is_skipped_rather_than_drawn() {
    let source = solid(RED);
    let canvas = composited(&[with(
        &source,
        Properties {
            opacity: 0.0,
            ..Properties::default()
        },
    )]);
    assert_pixel(&canvas, CENTRE, BLACK, "opacity zero");
}

#[test]
fn a_layers_own_alpha_is_honoured_as_well_as_its_opacity() {
    // Straight alpha in, premultiplied inside the rasteriser, straight out. If
    // that conversion were skipped the colour would come out too bright.
    let source = translucent(RED, 128);
    let canvas = composited(&[Layer::plain(&source)]);
    assert_pixel(&canvas, CENTRE, HALF_RED, "half-alpha red");
}

#[test]
fn position_moves_the_layer_and_leaves_black_behind_it() {
    let source = solid(RED);
    let canvas = composited(&[with(
        &source,
        Properties {
            position: (f64::from(SIZE) / 2.0, 0.0),
            ..Properties::default()
        },
    )]);
    assert_pixel(&canvas, (4, CENTRE.1), BLACK, "vacated by the move");
    assert_pixel(&canvas, (SIZE - 4, CENTRE.1), RED, "moved into");
}

#[test]
fn scale_shrinks_about_the_layers_own_centre() {
    // Centre-anchored, so scaling a clip does not also slide it into a corner —
    // which is what every editor does and what an author will expect.
    let source = solid(RED);
    let canvas = composited(&[with(
        &source,
        Properties {
            scale: (0.5, 0.5),
            ..Properties::default()
        },
    )]);
    assert_pixel(&canvas, CENTRE, RED, "the shrunken layer");
    assert_pixel(&canvas, CORNER, BLACK, "outside it");
    assert_pixel(&canvas, (SIZE - 4, SIZE - 4), BLACK, "and the far corner");
}

#[test]
fn a_zero_scale_layer_is_skipped_rather_than_drawn() {
    let source = solid(RED);
    let canvas = composited(&[with(
        &source,
        Properties {
            scale: (0.0, 1.0),
            ..Properties::default()
        },
    )]);
    assert_pixel(&canvas, CENTRE, BLACK, "scaled to nothing");
}
