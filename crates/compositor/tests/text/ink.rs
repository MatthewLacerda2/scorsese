//! Reading a drawn frame back as something a test can assert on.
//!
//! Glyph shapes are the golden fixtures' business — a picture is the only
//! honest reference for one. What these tests assert is everything *around* the
//! shapes: that ink appeared at all, in the colour asked for, inside the box it
//! was given, on as many lines as it should have taken.
#![allow(dead_code)]

use scorsese_compositor::text::Style;
use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Resolution};
use scorsese_core::{Rgba, TextAlign};

/// Wide enough for a sentence to wrap two or three times at the sizes below.
pub(crate) const WIDTH: u32 = 320;
pub(crate) const HEIGHT: u32 = 180;

/// A transparent frame, which is what a text layer starts as.
pub(crate) fn canvas() -> Frame {
    let mut frame = Frame::black(Resolution::new(WIDTH, HEIGHT).expect("a legal raster"));
    frame.fill_transparent();
    frame
}

/// A style with everything but the parts a test is about held still.
pub(crate) fn style(size: f32, color: Rgba) -> Style {
    Style {
        size,
        color,
        align: TextAlign::Center,
        line_height: size * 1.25,
        max_width: WIDTH as f32 * 0.9,
    }
}

/// One pixel as `(r, g, b, a)`.
pub(crate) fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8, u8) {
    let at = (y as usize * frame.resolution().width() as usize + x as usize) * BYTES_PER_PIXEL;
    let bytes = &frame.bytes()[at..at + BYTES_PER_PIXEL];
    (bytes[0], bytes[1], bytes[2], bytes[3])
}

/// How many pixels the glyphs touched at all.
pub(crate) fn count(frame: &Frame) -> usize {
    frame
        .bytes()
        .chunks_exact(BYTES_PER_PIXEL)
        .filter(|pixel| pixel[3] > 0)
        .count()
}

/// The box the ink occupies, as `(left, top, right, bottom)` inclusive.
pub(crate) fn bounds(frame: &Frame) -> Option<(u32, u32, u32, u32)> {
    let width = frame.resolution().width();
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for (index, pixel) in frame.bytes().chunks_exact(BYTES_PER_PIXEL).enumerate() {
        if pixel[3] == 0 {
            continue;
        }
        let (x, y) = (index as u32 % width, index as u32 / width);
        found = Some(match found {
            None => (x, y, x, y),
            Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
        });
    }
    found
}

/// How many bands of rows carry ink, separated by at least one blank row —
/// which is how many lines of text were drawn.
pub(crate) fn lines(frame: &Frame) -> usize {
    let width = frame.resolution().width() as usize;
    let mut lines = 0;
    let mut inside = false;
    for row in frame.bytes().chunks_exact(width * BYTES_PER_PIXEL) {
        let inked = row.chunks_exact(BYTES_PER_PIXEL).any(|pixel| pixel[3] > 0);
        if inked && !inside {
            lines += 1;
        }
        inside = inked;
    }
    lines
}

/// The most solid pixel drawn — the middle of a stem, where coverage is full
/// and the colour is the colour that was asked for rather than a blend of it.
pub(crate) fn most_solid(frame: &Frame) -> (u8, u8, u8, u8) {
    frame
        .bytes()
        .chunks_exact(BYTES_PER_PIXEL)
        .map(|pixel| (pixel[0], pixel[1], pixel[2], pixel[3]))
        .max_by_key(|pixel| pixel.3)
        .expect("a frame has pixels")
}
