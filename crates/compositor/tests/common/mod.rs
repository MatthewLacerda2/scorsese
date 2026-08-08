//! Shared fixtures. Each test file uses a different slice of this, so unused
//! items here are expected rather than dead.
#![allow(dead_code)]

use scorsese_compositor::{
    BYTES_PER_PIXEL, Compositor, CpuCompositor, Frame, Layer, Properties, Resolution,
};
use scorsese_core::{AssetId, Clip, ClipId, Easing, Frames, Keyframe, KeyframeTrack, PropertyPath};

/// The raster the tests work at: big enough for a centred half-scale layer to
/// land on whole pixels, small enough to reason about by hand.
pub(crate) const SIZE: u32 = 64;

/// The centre of the canvas, and a point well inside its top-left corner.
pub(crate) const CENTRE: (u32, u32) = (SIZE / 2, SIZE / 2);
pub(crate) const CORNER: (u32, u32) = (4, 4);

pub(crate) const BLACK: (u8, u8, u8) = (0, 0, 0);
pub(crate) const RED: (u8, u8, u8) = (255, 0, 0);
pub(crate) const BLUE: (u8, u8, u8) = (0, 0, 255);
pub(crate) const HALF_RED: (u8, u8, u8) = (128, 0, 0);

pub(crate) fn raster() -> Resolution {
    Resolution::new(SIZE, SIZE).expect("a legal raster")
}

/// A frame of one colour, fully opaque.
pub(crate) fn solid(colour: (u8, u8, u8)) -> Frame {
    translucent(colour, u8::MAX)
}

/// A frame of one colour at one alpha, in **straight** alpha as everything
/// outside the compositor speaks it.
pub(crate) fn translucent(colour: (u8, u8, u8), alpha: u8) -> Frame {
    let mut frame = Frame::black(raster());
    fill(&mut frame, colour, alpha);
    frame
}

/// An opaque frame at a size of its own — what a clip kept at its native size
/// arrives as, which is not necessarily the canvas's.
pub(crate) fn solid_of(colour: (u8, u8, u8), width: u32, height: u32) -> Frame {
    let mut frame = Frame::black(Resolution::source(width, height).expect("a legal source raster"));
    fill(&mut frame, colour, u8::MAX);
    frame
}

fn fill(frame: &mut Frame, colour: (u8, u8, u8), alpha: u8) {
    for pixel in frame.bytes_mut().chunks_exact_mut(BYTES_PER_PIXEL) {
        pixel.copy_from_slice(&[colour.0, colour.1, colour.2, alpha]);
    }
}

/// Composites layers onto a fresh canvas.
pub(crate) fn composited(layers: &[Layer<'_>]) -> Frame {
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .composite(&mut canvas, layers)
        .expect("compositing succeeds");
    canvas
}

/// A layer with properties other than the defaults.
pub(crate) fn with(source: &Frame, properties: Properties) -> Layer<'_> {
    Layer {
        source,
        properties,
        anchor: scorsese_core::Anchor::default(),
    }
}

/// One pixel, as `(r, g, b, a)`.
pub(crate) fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let width = frame.resolution().width() as usize;
    let at = (y as usize * width + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    (bytes[0], bytes[1], bytes[2], bytes[3])
}

/// Asserts a pixel is about a colour, allowing for the rasteriser's rounding.
#[track_caller]
pub(crate) fn assert_pixel(frame: &Frame, at: (u32, u32), expected: (u8, u8, u8), what: &str) {
    let found = pixel(frame, at.0, at.1);
    let close = |a: u8, b: u8| a.abs_diff(b) <= 2;
    assert!(
        close(found.0, expected.0) && close(found.1, expected.1) && close(found.2, expected.2),
        "{what}: pixel {at:?} should be about {expected:?}, found {found:?}"
    );
    assert_eq!(
        found.3,
        u8::MAX,
        "{what}: a composited frame is opaque everywhere"
    );
}

/// A track holding one keyframe, so a property has a fixed value.
pub(crate) fn constant(property: &str, value: f64) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new(property),
        vec![Keyframe {
            t: Frames::ZERO,
            value,
            easing: Easing::Linear,
        }],
    )
}

pub(crate) fn clip(duration: u64) -> Clip {
    Clip::new(
        ClipId::new("c1"),
        AssetId::new("a1"),
        Frames::ZERO,
        Frames(duration),
    )
}

/// The opacity a clip's own keyframes give at `t`.
pub(crate) fn opacity_at(clip: &Clip, t: u64) -> f64 {
    Properties::at(clip, Frames(t)).opacity
}
