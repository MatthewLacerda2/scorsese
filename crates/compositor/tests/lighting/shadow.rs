//! A drop shadow: the layer's alpha, moved, tinted, and drawn under it.

use scorsese_compositor::Properties;
use scorsese_core::{Rgba, Shadow};

use crate::{MIDDLE, PAST, about, composited, filled, lit, square};

/// Grey, so a black shadow shows on it and a white square does too.
const BED: [u8; 3] = [128, 128, 128];

/// A hard black shadow half the square's height down and to the right — eight
/// whole pixels each way, so it covers pixels 32 to 47 of both axes.
fn hard() -> Shadow {
    Shadow {
        color: Rgba::BLACK,
        offset_x: 0.5,
        offset_y: 0.5,
        softness: 0.0,
        opacity: 1.0,
    }
}

fn cast(shadow: Shadow) -> scorsese_compositor::Frame {
    let bed = filled([BED[0], BED[1], BED[2], u8::MAX], crate::CANVAS);
    let white = square([255, 255, 255]);
    composited(&[
        lit(&bed, Properties::default()),
        lit(
            &white,
            Properties {
                shadow: Some(shadow),
                ..Properties::default()
            },
        ),
    ])
}

/// Where it falls, and that the layer is on top of it.
#[test]
fn a_shadow_falls_down_and_right_under_the_layer() {
    let frame = cast(hard());
    about(
        &frame,
        (MIDDLE, MIDDLE),
        [255; 3],
        "the square is over its shadow",
    );
    about(
        &frame,
        (PAST + 4, PAST + 4),
        [0; 3],
        "the shadow past the square",
    );
    about(
        &frame,
        (PAST + 4, 28),
        BED,
        "not beside the square: it fell down too",
    );
    about(&frame, (20, 20), BED, "nothing up and to the left");
}

/// Past the square's own 16×16 raster, and not cut off there — the reason the
/// lit picture is padded.
#[test]
fn a_shadow_reaches_past_the_layers_own_raster() {
    let frame = cast(hard());
    for at in [PAST, PAST + 7] {
        about(&frame, (at, at), [0; 3], "outside the square's raster");
    }
    about(
        &frame,
        (PAST + 8, PAST + 8),
        BED,
        "and not a pixel past its offset",
    );
}

/// Opacity and the colour's alpha both scale it, and an overshoot clamps.
#[test]
fn a_shadows_strength_is_its_opacity_clamped() {
    let half = cast(Shadow {
        opacity: 0.5,
        ..hard()
    });
    about(&half, (PAST + 4, PAST + 4), [64; 3], "half over grey");
    let overshot = cast(Shadow {
        opacity: 1.3,
        ..hard()
    });
    about(&overshot, (PAST + 4, PAST + 4), [0; 3], "clamped to full");
    let none = cast(Shadow {
        opacity: -0.2,
        ..hard()
    });
    about(&none, (PAST + 4, PAST + 4), BED, "clamped to none");
}

/// Softness spreads the edge by exactly what `blur` would: three radii.
#[test]
fn a_soft_shadow_fades_over_three_radii() {
    // One pixel of blur on a 16-tall layer, so the edge spreads three pixels
    // either side of where the hard one fell.
    let frame = cast(Shadow {
        softness: 1.0 / 16.0,
        ..hard()
    });
    let edge = PAST + 8;
    let [r, ..] = crate::pixel(&frame, edge, PAST + 4);
    assert!(
        r > 0 && r < 128,
        "just past the hard edge, part shadow: {r}"
    );
    about(
        &frame,
        (edge + 3, PAST + 4),
        BED,
        "three radii out, none at all",
    );
}
