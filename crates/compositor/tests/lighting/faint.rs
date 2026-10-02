//! A glow where there is very little light to spread: a dot much smaller than
//! the frame it is drawn on (#644), and a line a pixel wide at the brightest a
//! glow goes (#645). Both are the size a drawn layer really is — a raster as
//! big as the canvas, nearly all of it transparent.

use scorsese_compositor::{BYTES_PER_PIXEL, Frame, MAX_GLOW_INTENSITY, Properties, Resolution};
use scorsese_core::{Glow, Rgba};

use crate::{lit, pixel};

/// The size of both rasters here, and of the canvas they land on.
const SIDE: u32 = 64;

/// A transparent canvas-sized raster with the pixels `solid` says opaque white.
fn drawn(solid: impl Fn(u32, u32) -> bool) -> Frame {
    let mut frame = Frame::black(Resolution::new(SIDE, SIDE).expect("a legal raster"));
    for (at, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let (x, y) = (at as u32 % SIDE, at as u32 / SIDE);
        let level = if solid(x, y) { u8::MAX } else { 0 };
        pixel.copy_from_slice(&[level; 4]);
    }
    frame
}

/// `source` glowing white at `radius` and the brightest intensity there is,
/// onto a black canvas.
fn glowing(source: &Frame, radius: f64) -> Frame {
    let glow = Glow {
        color: Some(Rgba::opaque(255, 255, 255)),
        radius,
        intensity: MAX_GLOW_INTENSITY,
    };
    crate::composited(&[lit(
        source,
        Properties {
            glow: Some(glow),
            ..Properties::default()
        },
    )])
}

/// A three-pixel dot at a radius of the raster's whole height. Spread that
/// far, its light is a tenth of a level at the brightest, so the radius is held
/// to the dot's own size and the halo shows.
#[test]
fn a_small_dot_glows_at_a_radius_of_the_whole_frame() {
    let dot = drawn(|x, y| (31..34).contains(&x) && (31..34).contains(&y));
    let frame = glowing(&dot, 1.0);
    for (x, y) in [(34, 32), (36, 32), (32, 29)] {
        let [r, ..] = pixel(&frame, x, y);
        assert!(r > 16, "light round the dot at ({x}, {y}): {r}");
    }
    let [r, ..] = pixel(&frame, 45, 32);
    assert_eq!(r, 0, "and none past three of the dot's widths: {r}");
}

/// Across a one-pixel line glowing at intensity four, the halo falls away from
/// the line and its slope changes gradually. Rounded to whole levels between
/// the blur's passes, it came out as flat runs a gain's worth of levels apart
/// — a slope of nothing, then four, then nothing, which is the rectangles of
/// #645 seen along one row.
#[test]
fn a_thin_lines_halo_falls_off_without_steps() {
    let line = drawn(|x, _| x == 32);
    let frame = glowing(&line, 8.0 / f64::from(SIDE));
    let row: Vec<u8> = (33..SIDE).map(|x| pixel(&frame, x, 32)[0]).collect();
    assert!(row[0] > 0, "the line glows: {row:?}");
    let slope: Vec<i16> = row
        .windows(2)
        .map(|pair| i16::from(pair[0]) - i16::from(pair[1]))
        .collect();
    assert!(slope.iter().all(|&fall| fall >= 0), "falling: {row:?}");
    for pair in slope.windows(2) {
        assert!(
            (pair[0] - pair[1]).abs() <= 2,
            "with no step in it: {row:?}"
        );
    }
}
