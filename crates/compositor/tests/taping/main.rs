//! What running a layer through a tape does to its pixels.
//!
//! **A two-colour plate, because a step edge makes the arithmetic exact.** Most
//! of what a tape does is a displacement or a running average along a row, and
//! both of those are exactly computable over a step: every expected byte below
//! is a number worked out from what `vhs/` documents rather than one read off a
//! run and blessed.
//!
//! The golden fixture holds the *displacements* — the wobble, the scanlines,
//! the torn band — because those are structure and survive an encode. It does
//! not hold the chroma smear, which moves nothing a luma SSIM can see, and it
//! carries no snow at all, which is grain's finding a second time. Both of
//! those are pinned here instead, before an encoder has seen them, and
//! `docs/golden-renders.md` has the measurement that decides which is which.
//!
//! **The properties come from a [`Clip`] and never from a literal**, because
//! the seed that carries the frame into the tape is resolved by
//! [`Properties::at`] and by nothing else. A test that set the fields by hand
//! would assert the arithmetic and never that a clip's own `vhs` reaches it.

mod colour;
mod lines;
mod paths;
mod snow;
mod tracking;

use scorsese_compositor::{
    BYTES_PER_PIXEL, Compositor, CpuCompositor, Frame, Layer, Properties, Resolution,
};
use scorsese_core::{AssetId, Clip, ClipId, Frames, Vhs};

/// The square these tests work on. 64 across, so a smear of five columns is a
/// small fraction of the row and the step at its middle is nowhere near either
/// edge.
pub(crate) const SIZE: u32 = 64;

/// The column the plate changes colour at.
pub(crate) const STEP: u32 = 32;

/// The two halves of the plate, opaque. Chosen so that neither their brightness
/// nor their colour differences are anywhere near equal — the smear runs on the
/// colour differences alone, so a plate whose halves shared one would hide a
/// channel going the wrong way.
pub(crate) const LEFT: [u8; 4] = [200, 40, 40, 255];
pub(crate) const RIGHT: [u8; 4] = [40, 40, 200, 255];

/// A plate that is [`LEFT`] up to [`STEP`] and [`RIGHT`] from there on, the same
/// on every row — so a pixel's row never enters the arithmetic unless something
/// put it there.
pub(crate) fn plate() -> Frame {
    let mut frame = Frame::black(raster());
    for (index, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let x = index as u32 % SIZE;
        pixel.copy_from_slice(if x < STEP { &LEFT } else { &RIGHT });
    }
    frame
}

/// A plate of one colour, for the artefacts that need no edge to show on.
pub(crate) fn flat(colour: [u8; 4]) -> Frame {
    let mut frame = Frame::black(raster());
    for pixel in frame.bytes_mut().chunks_exact_mut(BYTES_PER_PIXEL) {
        pixel.copy_from_slice(&colour);
    }
    frame
}

/// One instant of a clip carrying `vhs`, resolved the way a render resolves it.
pub(crate) fn instant(vhs: Vhs, clip: &str, t: u32) -> Properties {
    let clip = Clip {
        vhs,
        ..Clip::new(
            ClipId::new(clip),
            AssetId::new("a"),
            Frames::ZERO,
            Frames(30),
        )
    };
    Properties::at(&clip, Frames(u64::from(t)))
}

/// A layer with a tape on it and no geometry at all, so what lands on the
/// canvas is `vhs/`'s arithmetic and nothing a rasteriser did on the way.
pub(crate) fn taped(source: &Frame, properties: Properties) -> Frame {
    let layer = Layer {
        properties,
        ..Layer::plain(source)
    };
    let mut canvas = Frame::black(raster());
    CpuCompositor::new()
        .composite(&mut canvas, &[layer])
        .expect("compositing succeeds");
    canvas
}

/// One pixel as `(r, g, b, a)`.
pub(crate) fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let at = (y as usize * SIZE as usize + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    (bytes[0], bytes[1], bytes[2], bytes[3])
}

/// The Rec.709 brightness of a pixel, which several tests assert an artefact
/// leaves alone.
pub(crate) fn luma(pixel: (u8, u8, u8, u8)) -> f64 {
    0.2126 * f64::from(pixel.0) + 0.7152 * f64::from(pixel.1) + 0.0722 * f64::from(pixel.2)
}

/// How far this row of a taped [`plate`] was displaced sideways, in pixels.
///
/// The plate is one step, and a displacement of whole pixels leaves it one
/// step — so the column it changes at, less the column it started at, is the
/// whole of what moved. Signed: positive is rightward.
pub(crate) fn shift_of(frame: &Frame, y: u32) -> i32 {
    let first = pixel(frame, 0, y);
    let found = (0..SIZE)
        .find(|&x| pixel(frame, x, y) != first)
        .unwrap_or(SIZE);
    found as i32 - STEP as i32
}

pub(crate) fn raster() -> Resolution {
    Resolution::new(SIZE, SIZE).expect("a legal raster")
}
