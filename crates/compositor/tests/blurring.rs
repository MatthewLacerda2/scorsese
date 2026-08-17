//! What softening a layer does to its pixels — and the three things it must
//! not do: darken the edges of the layer, halo a soft edge, or be skipped.

mod common;

use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Layer, Properties, Resolution, path};
use scorsese_core::{Frames, Grade};

use common::{BLUE, RED, SIZE, composited, constant, pixel, raster, solid, with};

/// Where the badge's edges sit: a centred square half the raster across.
const EDGE: u32 = SIZE / 4;

/// A blur of a tenth of the layer's height — six pixels at this raster, which
/// is wide enough to see and narrow enough to leave the corners alone.
const SOFT: f64 = 0.1;

fn softened(blur: f64) -> Properties {
    Properties {
        blur,
        ..Properties::default()
    }
}

/// Red on the left, blue on the right, opaque, filling the canvas: one hard
/// edge in the middle and nothing else to confuse a reading.
fn split() -> Frame {
    let mut frame = Frame::black(raster());
    for (index, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let colour = if index as u32 % SIZE < SIZE / 2 {
            RED
        } else {
            BLUE
        };
        pixel.copy_from_slice(&[colour.0, colour.1, colour.2, u8::MAX]);
    }
    frame
}

/// Transparent everywhere but a centred square, with black stored under the
/// transparent part — which is what a blur in straight alpha would smear into
/// the visible edge.
fn badge(colour: (u8, u8, u8)) -> Frame {
    let mut frame = Frame::black(Resolution::source(SIZE, SIZE).expect("a legal source raster"));
    for (index, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let (x, y) = (index as u32 % SIZE, index as u32 / SIZE);
        let inside = (EDGE..SIZE - EDGE).contains(&x) && (EDGE..SIZE - EDGE).contains(&y);
        let written = if inside {
            [colour.0, colour.1, colour.2, u8::MAX]
        } else {
            [0, 0, 0, 0]
        };
        pixel.copy_from_slice(&written);
    }
    frame
}

#[test]
fn a_uniform_layer_comes_through_a_blur_unchanged() {
    // The edge clamp, asserted where it is visible: a window running off the
    // layer reads the nearest pixel on it, so a flat colour averages to itself
    // everywhere. Treating off-layer as black instead would draw a dark border
    // around every blurred layer, and the corners are where it would be worst.
    let source = solid(RED);
    let out = composited(&[with(&source, softened(SOFT))]);
    for at in [(0, 0), (SIZE - 1, 0), (0, SIZE - 1), (SIZE - 1, SIZE - 1)] {
        assert_eq!(pixel(&out, at.0, at.1), (255, 0, 0, 255), "corner {at:?}");
    }
}

#[test]
fn a_hard_edge_comes_out_soft_and_the_rest_stays_put() {
    let source = split();
    let out = composited(&[with(&source, softened(SOFT))]);

    let middle = pixel(&out, SIZE / 2, SIZE / 2);
    assert!(
        middle.0 > 40 && middle.2 > 40,
        "the edge should carry both colours, found {middle:?}"
    );
    // Three passes of a six-pixel window reach eighteen pixels, so a point four
    // from the left is nowhere near the edge and must not have moved.
    assert_eq!(pixel(&out, 4, SIZE / 2), (255, 0, 0, 255));
}

#[test]
fn a_blurred_layer_is_never_copied_whole() {
    // The copy path memcpys a layer that would rasterise to its own pixels, and
    // a blurred clip with no transform on it satisfies every other condition
    // for that — so without this it would render sharp, silently, on exactly
    // the commonest case: a full-frame plate with one number on it.
    assert!(!softened(SOFT).is_identity(), "a blur is not a copy");
    assert!(
        softened(0.0).is_identity() && softened(-1.0).is_identity(),
        "nothing to do is still a copy"
    );

    let source = split();
    assert_ne!(
        composited(&[with(&source, softened(SOFT))]).bytes(),
        source.bytes(),
        "the blurred layer came through untouched"
    );
}

#[test]
fn a_radius_under_half_a_pixel_is_a_no_op() {
    // Not merely invisible: it short-circuits, rather than running six passes
    // over a whole layer to move no pixel anywhere.
    let source = split();
    assert_eq!(
        composited(&[with(&source, softened(0.001))]).bytes(),
        source.bytes()
    );
}

#[test]
fn a_soft_edge_over_a_bed_carries_no_dark_halo() {
    // The classic blur bug, and the reason the blur runs after premultiplying:
    // averaged in straight alpha, the black stored under the transparent
    // surround is dragged into the visible edge and rings the badge in it.
    let bed = solid(RED);
    let badge = badge(BLUE);
    let out = composited(&[Layer::plain(&bed), with(&badge, softened(SOFT))]);

    let edge = pixel(&out, EDGE, SIZE / 2);
    let light = u32::from(edge.0) + u32::from(edge.2);
    assert!(
        light >= 235,
        "the edge should be red giving way to blue and no darker than either, \
         found {edge:?}"
    );
    assert_eq!(edge.1, 0, "nothing here is green");
}

#[test]
fn blur_is_a_baseline_and_a_track_takes_it_over() {
    let baseline = Properties::over(Grade::NEUTRAL, 0.03, &[], Frames::ZERO);
    assert!((baseline.blur - 0.03).abs() < f64::EPSILON);

    let animated = Properties::over(
        Grade::NEUTRAL,
        0.03,
        &[constant(path::BLUR, 0.5)],
        Frames::ZERO,
    );
    assert!((animated.blur - 0.5).abs() < f64::EPSILON);
}
