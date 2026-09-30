//! A glow: the layer blurred into a halo, in its own colours or a tint, and
//! drawn under it.

use scorsese_compositor::{
    Compositor, CpuCompositor, Frame, MAX_GLOW_INTENSITY, Properties, Resolution,
};
use scorsese_core::{Glow, Rgba};

use crate::{MIDDLE, PAST, about, canvas, composited, lit, pixel, square};

const RED: [u8; 3] = [255, 0, 0];

/// One pixel of blur on the 16-tall square, so the halo is exactly three
/// pixels deep.
fn one_pixel(intensity: f64) -> Glow {
    Glow {
        color: None,
        radius: 1.0 / 16.0,
        intensity,
    }
}

fn glowing(source: &Frame, glow: Glow) -> Frame {
    composited(&[lit(
        source,
        Properties {
            glow: Some(glow),
            ..Properties::default()
        },
    )])
}

#[test]
fn a_glow_is_the_layers_own_colour_around_it_and_not_on_it() {
    let frame = glowing(&square(RED), one_pixel(1.0));
    about(&frame, (MIDDLE, MIDDLE), RED, "the layer itself, untouched");
    let [r, g, b, _] = pixel(&frame, PAST, MIDDLE);
    assert!(
        r > 0 && g == 0 && b == 0,
        "red light past the edge: {r} {g} {b}"
    );
    let [r, ..] = pixel(&frame, PAST + 2, MIDDLE);
    assert!(r > 0, "still some two pixels past the raster: {r}");
    about(
        &frame,
        (PAST + 3, MIDDLE),
        [0; 3],
        "and none three radii out",
    );
}

#[test]
fn a_tinted_glow_is_the_tint_and_leaves_the_layer_alone() {
    let green = Glow {
        color: Some(Rgba::opaque(0, 255, 0)),
        ..one_pixel(1.0)
    };
    let frame = glowing(&square(RED), green);
    about(&frame, (MIDDLE, MIDDLE), RED, "the layer over its halo");
    let [r, g, b, _] = pixel(&frame, PAST, MIDDLE);
    assert!(r == 0 && g > 0 && b == 0, "green light: {r} {g} {b}");
}

/// Intensity brightens, and clamps at both ends where it is drawn — so an
/// overshooting easing flashes brighter rather than going negative.
#[test]
fn intensity_brightens_and_clamps() {
    let red = square(RED);
    let at = |glow| pixel(&glowing(&red, glow), PAST + 1, MIDDLE)[0];
    assert!(at(one_pixel(2.0)) > at(one_pixel(1.0)), "more is brighter");
    assert_eq!(at(one_pixel(100.0)), at(one_pixel(MAX_GLOW_INTENSITY)));
    let dark = glowing(&red, one_pixel(-1.0));
    assert_eq!(dark, composited(&[lit(&red, Properties::default())]));
    let nan = glowing(&red, one_pixel(f64::NAN));
    assert_eq!(nan, dark, "a non-number is no glow either");
}

/// A group is one picture by the time it meets its glow: two separate members
/// drawn offscreen, then lit once, both glow.
#[test]
fn a_glow_on_a_group_lights_every_member() {
    let member = square(RED);
    let place = |x: f64| {
        lit(
            &member,
            Properties {
                position: (x, 0.0),
                ..Properties::default()
            },
        )
    };
    let mut group = Frame::black(canvas());
    CpuCompositor::new()
        .offscreen(&mut group, &[place(-0.25), place(0.25)])
        .expect("compositing succeeds");
    assert_eq!(group.resolution(), Resolution::new(64, 64).expect("legal"));

    // Each member is 16 across, centred 16 either side of the middle: the left
    // one ends at pixel 23, the right one starts at 40. The glow is measured
    // against the group's own height, 64, so one pixel of it is 1/64.
    let frame_lit = glowing(
        &group,
        Glow {
            radius: 1.0 / 64.0,
            ..one_pixel(1.0)
        },
    );
    for x in [24, 39] {
        let [r, ..] = pixel(&frame_lit, x, MIDDLE);
        assert!(
            r > 0,
            "light beside the member ending or starting there: {r}"
        );
    }
    assert_eq!(pixel(&frame_lit, MIDDLE, MIDDLE)[0], 0, "and dark between");
    // A sixteenth of the *group's* height is four pixels, twelve deep, which
    // reaches the middle from both sides; a sixteenth of a member's would be
    // one pixel and leave it dark.
    let wide = glowing(&group, one_pixel(1.0));
    assert!(
        pixel(&wide, MIDDLE, MIDDLE)[0] > 0,
        "measured against the group"
    );
}
