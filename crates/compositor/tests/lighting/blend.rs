//! How a layer lands on what is beneath it: the four blends, on numbers small
//! enough to work out by hand.

use scorsese_compositor::Properties;
use scorsese_core::Blend;

use crate::{CANVAS, MIDDLE, about, composited, filled, lit};

const BENEATH: [u8; 4] = [100, 50, 0, 255];
const ABOVE: [u8; 4] = [100, 100, 100, 255];

/// The whole canvas in `ABOVE`, blended onto the whole canvas in `BENEATH` —
/// which is also the one arrangement the compositor would otherwise *copy*
/// rather than blend, so each of these proves the copy path steps aside.
fn landed(blend: Blend, above: [u8; 4]) -> scorsese_compositor::Frame {
    let beneath = filled(BENEATH, CANVAS);
    let above = filled(above, CANVAS);
    composited(&[
        lit(&beneath, Properties::default()),
        lit(
            &above,
            Properties {
                blend,
                ..Properties::default()
            },
        ),
    ])
}

#[test]
fn each_blend_is_its_textbook_formula() {
    let at = (MIDDLE, MIDDLE);
    about(
        &landed(Blend::Normal, ABOVE),
        at,
        [100, 100, 100],
        "normal covers",
    );
    about(&landed(Blend::Add, ABOVE), at, [200, 150, 100], "add sums");
    // 1 − (1 − a)(1 − b): 100 and 100 make 161, 50 and 100 make 130.
    about(&landed(Blend::Screen, ABOVE), at, [161, 130, 100], "screen");
    // a·b: 100 and 100 make 39, 50 and 100 make 20.
    about(&landed(Blend::Multiply, ABOVE), at, [39, 20, 0], "multiply");
}

#[test]
fn add_clips_at_white_and_the_frame_stays_opaque() {
    let bright = [200, 250, 255, 255];
    about(
        &landed(Blend::Add, bright),
        (0, 0),
        [255, 255, 255],
        "clipped",
    );
}

/// Where the layer is transparent it contributes nothing, whatever the mode —
/// a shape's empty surround set to `add` must not light the frame.
#[test]
fn a_transparent_pixel_blends_to_nothing() {
    for blend in [Blend::Add, Blend::Screen, Blend::Multiply] {
        let frame = landed(blend, [255, 255, 255, 0]);
        about(&frame, (MIDDLE, MIDDLE), [100, 50, 0], blend.as_str());
    }
}
