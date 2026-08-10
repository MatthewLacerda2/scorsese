//! The vignette: how far the light falls off, where that is measured from, and
//! what it must never touch.

use scorsese_compositor::Layer;
use scorsese_core::Grade;

use crate::{SIZE, composited, layer, pixel, solid};

/// A grade with the vignette turned and nothing else.
fn vignette(to: f64) -> Grade {
    Grade {
        vignette: to,
        ..Grade::NEUTRAL
    }
}

/// A white layer under a vignette of `strength`, read at one pixel.
fn white_under(strength: f64, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let source = solid((255, 255, 255));
    pixel(&composited(&[layer(&source, vignette(strength))]), x, y)
}

/// On a 64-pixel square the outermost pixel's centre sits `63/64` of the way
/// out along both axes, so it keeps `1 − (63/64)²` — `0.031` — of its light;
/// the middle of an edge is `0.4846` of the way to a corner and keeps
/// `0.5154`; the centre keeps all but a rounding error of it.
///
/// Squared and not linear, which is what makes it read as a vignette: a linear
/// ramp would take a fifth of the light out of the middle of the frame, and the
/// middle of the frame is what a vignette exists to leave alone.
#[test]
fn a_full_vignette_falls_off_by_the_squared_distance_to_a_corner() {
    assert_eq!(white_under(1.0, 0, 0), (8, 8, 8, 255));
    assert_eq!(white_under(1.0, SIZE - 1, SIZE - 1), (8, 8, 8, 255));
    assert_eq!(white_under(1.0, 0, SIZE / 2), (131, 131, 131, 255));
    assert_eq!(white_under(1.0, SIZE / 2, SIZE / 2), (255, 255, 255, 255));
}

/// Half the vignette takes half the light the whole one took, at every pixel.
#[test]
fn the_strength_scales_that_falloff_and_nothing_else() {
    assert_eq!(white_under(0.5, 0, 0), (131, 131, 131, 255));
    assert_eq!(white_under(0.5, 0, SIZE / 2), (193, 193, 193, 255));
}

/// Past `1.0` there is no more light to take, so it is clamped there rather
/// than pushing the corners past black and out the other side.
#[test]
fn a_vignette_past_one_is_the_same_as_a_full_one() {
    assert_eq!(white_under(4.0, 0, 0), (8, 8, 8, 255));
}

/// And below zero there is none to give back: a vignette darkens or does
/// nothing, and never brightens the corners.
#[test]
fn a_negative_vignette_leaves_every_pixel_alone() {
    assert_eq!(white_under(-1.0, 0, 0), (255, 255, 255, 255));
    assert_eq!(white_under(-1.0, SIZE / 2, SIZE / 2), (255, 255, 255, 255));
}

/// It darkens toward the corners rather than fading out, which is only
/// visible once there is something underneath: a vignette that ate the alpha
/// would let the red track below show through the corners of the white one
/// above it. The corner is the same near-black 8 it is over nothing at all,
/// with no red in it whatsoever.
#[test]
fn a_vignetted_layer_darkens_its_own_corners_without_thinning_them() {
    let below = solid((255, 0, 0));
    let above = solid((255, 255, 255));
    let canvas = composited(&[Layer::plain(&below), layer(&above, vignette(1.0))]);

    assert_eq!(pixel(&canvas, 0, 0), (8, 8, 8, 255));
    assert_eq!(pixel(&canvas, SIZE / 2, SIZE / 2), (255, 255, 255, 255));
}
