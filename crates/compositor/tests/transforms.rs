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

/// A bar as wide as the canvas and a quarter as tall, resting across the
/// middle. Uniform colour cannot show a turn — a square rotated about its own
/// centre covers the same square — so the shape has to be asymmetric before
/// rotation means anything visible at all.
fn bar() -> scorsese_compositor::Frame {
    common::solid_of(RED, SIZE, SIZE / 4)
}

/// The anchor. Turned about its own centre, a quarter turn stands the bar
/// upright through the middle of the canvas. Turned about a corner it would
/// swing away and the centre would go black.
#[test]
fn a_turn_is_about_the_layers_own_centre() {
    let source = bar();
    let across = composited(&[with(&source, Properties::default())]);
    assert_pixel(&across, (4, SIZE / 2), RED, "resting, the bar runs across");
    assert_pixel(&across, (SIZE / 2, 4), BLACK, "and nothing above it");

    let upright = composited(&[with(
        &source,
        Properties {
            rotation: 90.0,
            ..Properties::default()
        },
    )]);
    assert_pixel(&upright, CENTRE, RED, "a turn about the centre stays there");
    assert_pixel(&upright, (SIZE / 2, 4), RED, "the bar now runs up and down");
    assert_pixel(&upright, (4, SIZE / 2), BLACK, "and no longer across");
}

/// The sign, which a quarter turn cannot show — ±90° both stand the bar
/// upright. At 45° it can lie along only one diagonal, and which one is the
/// whole question. Positive is clockwise, so the right-hand end goes **down**.
#[test]
fn positive_rotation_turns_clockwise() {
    let source = bar();
    let canvas = composited(&[with(
        &source,
        Properties {
            rotation: 45.0,
            ..Properties::default()
        },
    )]);
    let (cx, cy) = (SIZE / 2, SIZE / 2);
    assert_pixel(&canvas, (cx + 12, cy + 12), RED, "right end swung down");
    assert_pixel(&canvas, (cx + 12, cy - 12), BLACK, "not up");
}
