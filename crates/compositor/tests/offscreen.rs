//! Compositing onto a transparent canvas, so the result can be a layer itself —
//! the seam a group is drawn through.
//!
//! Two properties make it a seam rather than a second compositor: where nothing
//! is drawn the canvas stays **transparent**, so whatever the result is later
//! composited over shows through; and what comes out is **straight** alpha,
//! the form every layer's source is read in. A canvas left premultiplied would
//! come out dark wherever it was translucent, the moment it was composited.

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer, Resolution};
use scorsese_core::Rgba;

fn raster() -> Resolution {
    Resolution::new(16, 16).expect("a legal raster")
}

/// A layer of one colour covering the whole raster.
fn filled(color: Rgba) -> Frame {
    let mut frame = Frame::black(raster());
    frame.fill(color);
    frame
}

fn first_pixel(frame: &Frame) -> [u8; 4] {
    let bytes = frame.bytes();
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

#[test]
fn nothing_drawn_is_nothing_at_all() {
    let mut canvas = filled(Rgba::WHITE);
    CpuCompositor::new()
        .offscreen(&mut canvas, &[])
        .expect("composites");
    assert_eq!(first_pixel(&canvas), [0, 0, 0, 0]);
}

/// Half-opaque red on a transparent canvas is half-opaque red, in straight
/// alpha — full red, alpha 128 — and not the dark 128-level red a
/// premultiplied canvas would read as.
#[test]
fn a_translucent_layer_comes_out_straight() {
    let red = filled(Rgba::new(255, 0, 0, 128));
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .offscreen(&mut canvas, &[Layer::plain(&red)])
        .expect("composites");
    let [r, g, b, a] = first_pixel(&canvas);
    assert!(r >= 254 && g == 0 && b == 0, "straight red: {r} {g} {b}");
    assert!((127..=129).contains(&a), "half coverage: {a}");
}

/// An opaque layer over a translucent one covers it completely — what makes a
/// faded group's overlapping members not show through each other.
#[test]
fn the_upper_of_two_layers_covers_the_lower() {
    let red = filled(Rgba::new(255, 0, 0, 255));
    let blue = filled(Rgba::new(0, 0, 255, 255));
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .offscreen(&mut canvas, &[Layer::plain(&red), Layer::plain(&blue)])
        .expect("composites");
    assert_eq!(first_pixel(&canvas), [0, 0, 255, 255]);
}
