//! How far light reaches past the layer's own raster, at a radius wide enough
//! that a padding short of three radii would show — the box #585 fixed.
//!
//! The other tests here blur by one pixel, where `3 · r` and `3 + r` are both
//! three: a padding computed wrongly lands on the right answer by coincidence.
//! Four pixels is twelve deep, and anything short of twelve clips the light at
//! a straight edge before it has faded out.

use scorsese_compositor::Properties;
use scorsese_core::{Glow, Rgba, Shadow};

use crate::{MIDDLE, PAST, composited, filled, lit, pixel, square};

/// Four pixels of blur on the 16-tall square.
const FOUR: f64 = 4.0 / 16.0;

#[test]
fn a_wide_glow_fades_out_over_all_three_radii() {
    let red = square([255, 0, 0]);
    let glow = Glow {
        color: None,
        radius: FOUR,
        intensity: 1.0,
    };
    let frame = composited(&[lit(
        &red,
        Properties {
            glow: Some(glow),
            ..Properties::default()
        },
    )]);
    let [far, ..] = pixel(&frame, PAST + 9, MIDDLE);
    assert!(far > 0, "light ten pixels past the raster: {far}");
    let [beyond, ..] = pixel(&frame, PAST + 12, MIDDLE);
    assert_eq!(beyond, 0, "and none three radii out");
}

/// The shadow's reach is its softness *and* its offset: eight pixels down
/// and right, then twelve more of fading edge.
#[test]
fn a_wide_soft_shadow_fades_out_past_its_offset() {
    const BED: u8 = 128;
    let bed = filled([BED, BED, BED, u8::MAX], crate::CANVAS);
    let white = square([255; 3]);
    let shadow = Shadow {
        color: Rgba::BLACK,
        offset_x: 0.5,
        offset_y: 0.5,
        softness: FOUR,
        opacity: 1.0,
    };
    let frame = composited(&[
        lit(&bed, Properties::default()),
        lit(
            &white,
            Properties {
                shadow: Some(shadow),
                ..Properties::default()
            },
        ),
    ]);
    let edge = PAST + 8;
    let [far, ..] = pixel(&frame, edge + 9, PAST + 4);
    assert!(far < BED, "shadow ten pixels past its hard edge: {far}");
    let [beyond, ..] = pixel(&frame, edge + 12, PAST + 4);
    assert_eq!(beyond, BED, "and none three radii out");
}
