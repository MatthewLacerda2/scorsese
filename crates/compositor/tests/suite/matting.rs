//! A layer drawn through another layer's alpha — a track matte.
//!
//! What has to hold: the masked layer shows where the matte is opaque and
//! nowhere else, or the reverse when inverted; the matte itself is never
//! painted; a translucent matte is a translucent reveal; and the matte is
//! drawn with its own transform, so moving it moves the reveal.

use scorsese_compositor::{Compositor, CpuCompositor, Frame, Layer, Matte, Properties, Resolution};
use scorsese_core::Rgba;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

fn raster() -> Resolution {
    Resolution::new(16, 8).expect("a legal raster")
}

fn filled(color: Rgba) -> Frame {
    let mut frame = Frame::black(raster());
    frame.fill(color);
    frame
}

/// White on the left half, transparent on the right.
fn left_half() -> Frame {
    let mut frame = filled(Rgba::WHITE);
    for (at, pixel) in frame.bytes_mut().chunks_exact_mut(4).enumerate() {
        if at % 16 >= 8 {
            pixel.copy_from_slice(&[0, 0, 0, 0]);
        }
    }
    frame
}

fn pixel(frame: &Frame, x: usize, y: usize) -> [u8; 4] {
    let at = (y * 16 + x) * 4;
    frame.bytes()[at..at + 4].try_into().expect("four bytes")
}

/// Red, a full-frame opaque layer at identity — exactly the layer the
/// compositor would otherwise *copy* rather than draw — through `matte`.
fn through(matte: Layer<'_>, invert: bool) -> Frame {
    let red = filled(Rgba::new(255, 0, 0, 255));
    let layer = Layer {
        matte: Some(Box::new(Matte {
            layer: matte,
            invert,
        })),
        ..Layer::plain(&red)
    };
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .composite(&mut canvas, &[layer])
        .expect("composites");
    canvas
}

#[test]
fn a_layer_shows_only_where_its_matte_is() {
    let matte = left_half();
    let canvas = through(Layer::plain(&matte), false);
    assert_eq!(pixel(&canvas, 2, 4), RED, "revealed on the left");
    assert_eq!(pixel(&canvas, 13, 4), BLACK, "hidden on the right");
}

/// And never is the matte's own white painted anywhere — it is a mask, not a
/// layer.
#[test]
fn inverted_it_shows_only_where_the_matte_is_not() {
    let matte = left_half();
    let canvas = through(Layer::plain(&matte), true);
    assert_eq!(pixel(&canvas, 2, 4), BLACK, "a hole on the left");
    assert_eq!(pixel(&canvas, 13, 4), RED, "shown on the right");
}

/// A matte faded to half is a half reveal: coverage, not colour — the red
/// stays red, only less of it lands.
#[test]
fn a_translucent_matte_is_a_translucent_reveal() {
    let matte = filled(Rgba::WHITE);
    let half = Layer {
        properties: Properties {
            opacity: 0.5,
            ..Properties::default()
        },
        ..Layer::plain(&matte)
    };
    let [r, g, b, _] = pixel(&through(half, false), 8, 4);
    assert!(
        (126..=130).contains(&r) && g == 0 && b == 0,
        "half red: {r} {g} {b}"
    );
}

/// Nothing drawn is nothing revealed — and, inverted, nothing cut out.
#[test]
fn an_invisible_matte_hides_everything_or_nothing() {
    let matte = filled(Rgba::WHITE);
    let gone = || Layer {
        properties: Properties {
            opacity: 0.0,
            ..Properties::default()
        },
        ..Layer::plain(&matte)
    };
    assert_eq!(pixel(&through(gone(), false), 4, 4), BLACK);
    assert_eq!(pixel(&through(gone(), true), 4, 4), RED);
}

/// The matte is drawn where its own transform puts it: slid right by half the
/// frame, the left-half matte now reveals the right half.
#[test]
fn the_matte_moves_with_its_own_transform() {
    let matte = left_half();
    let slid = Layer {
        properties: Properties {
            position: (0.5, 0.0),
            ..Properties::default()
        },
        ..Layer::plain(&matte)
    };
    let canvas = through(slid, false);
    assert_eq!(pixel(&canvas, 2, 4), BLACK, "the left is covered no more");
    assert_eq!(pixel(&canvas, 13, 4), RED, "the right is revealed");
}

/// A masked layer composites over what is beneath it like any other: the
/// hidden part leaves the lower layer showing, not black.
#[test]
fn what_the_matte_hides_leaves_the_layer_beneath() {
    let blue = filled(Rgba::new(0, 0, 255, 255));
    let red = filled(Rgba::new(255, 0, 0, 255));
    let matte = left_half();
    let masked = Layer {
        matte: Some(Box::new(Matte {
            layer: Layer::plain(&matte),
            invert: false,
        })),
        ..Layer::plain(&red)
    };
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .composite(&mut canvas, &[Layer::plain(&blue), masked])
        .expect("composites");
    assert_eq!(pixel(&canvas, 2, 4), RED);
    assert_eq!(pixel(&canvas, 13, 4), [0, 0, 255, 255]);
}
