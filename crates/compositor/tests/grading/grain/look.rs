//! What grain does to a picture: film rather than a video sensor.

use crate::{SIZE, composited, pixel, solid};

use super::{MID, colour_at, grain, grained, grained_raster};

#[test]
fn no_grain_is_the_picture_exactly_as_it_arrived() {
    assert!(grain(0.0).is_neutral(), "a grain of zero changes nothing");
    let source = solid(MID);
    assert_eq!(
        composited(&[grained(&source, 0.0, 7)]).bytes(),
        source.bytes()
    );
}

#[test]
fn grain_moves_pixels_and_moves_each_one_differently() {
    let canvas = composited(&[grained(&solid(MID), 1.0, 7)]);
    let mut seen = std::collections::BTreeSet::new();
    for y in 0..SIZE {
        seen.insert(pixel(&canvas, y % SIZE, y).0);
    }
    assert!(
        seen.len() > SIZE as usize / 2,
        "the noise should differ pixel to pixel, found {} values",
        seen.len()
    );
}

#[test]
fn the_grain_is_one_value_on_all_three_channels() {
    // Monochrome is film; noise drawn per channel is a video sensor. The two
    // look different, and only one of them is what this is for.
    let canvas = composited(&[grained(&solid(MID), 1.0, 21)]);
    for y in 0..SIZE {
        let (r, g, b, _) = pixel(&canvas, y % SIZE, y);
        let moved = |found: u8, from: u8| i32::from(found) - i32::from(from);
        assert_eq!(
            (moved(r, MID.0), moved(g, MID.1)),
            (moved(b, MID.2), moved(b, MID.2)),
            "row {y} moved by different amounts per channel"
        );
    }
}

#[test]
fn grain_falls_away_at_both_ends_of_the_range() {
    // Uniform noise across the whole range is what reads as digital noise added
    // in post, because it puts texture into blown highlights and crushed blacks
    // where a photochemical image has none.
    for extreme in [(0, 0, 0), (255, 255, 255)] {
        let source = solid(extreme);
        assert_eq!(
            composited(&[grained(&source, 1.0, 5)]).bytes(),
            source.bytes(),
            "{extreme:?} should be left alone"
        );
    }
}

#[test]
fn a_grain_past_one_is_the_same_as_a_full_one() {
    // The vignette's rule, for the same reason: past the point where more is a
    // look anybody asked for, the number stops meaning anything new.
    let source = solid(MID);
    assert_eq!(
        composited(&[grained(&source, 1.0, 3)]).bytes(),
        composited(&[grained(&source, 4.0, 3)]).bytes()
    );
    // And below zero there is no grain in the other direction to give.
    assert_eq!(
        composited(&[grained(&source, -1.0, 3)]).bytes(),
        source.bytes()
    );
}

#[test]
fn grain_adds_texture_and_not_exposure() {
    // Centred on zero: a grained shot is textured, never brighter or darker.
    // Noise drawn from the wrong half of the range would lift the whole
    // picture, which every other test here would wave straight through.
    let canvas = composited(&[grained(&solid(MID), 0.3, 11)]);
    let mean = canvas
        .bytes()
        .chunks_exact(4)
        .map(|pixel| f64::from(pixel[0]))
        .sum::<f64>()
        / f64::from(SIZE * SIZE);
    assert!(
        (mean - f64::from(MID.0)).abs() < 1.0,
        "the mean red is {mean:.2} where the source is {}",
        MID.0
    );
}

#[test]
fn the_grain_is_coarser_on_a_taller_layer() {
    // Grain has a **size**, and it is a size on the film rather than a count of
    // pixels: a 4K delivery of an edit must not come out finer-grained than the
    // 1080p one, the same way a `blur` measured in pixels would be wrong the
    // first time a project was delivered at another size. So above 1080 a cell
    // covers more than one pixel and the pixels inside it share a value.
    //
    // Nothing at 64×64 can see this, which is exactly why it is here: a cell
    // there is one pixel, and dividing by one is the same arithmetic as
    // multiplying by it.
    let tall = grained_raster(4, 2160, 1.0, 4);
    let at = |x, y| colour_at(&tall, x, y);
    assert_eq!(at(0, 0), at(1, 0), "at 2160 tall a cell is two pixels wide");
    assert_eq!(at(0, 0), at(0, 1), "and two pixels tall");
    assert_ne!(at(0, 0), at(2, 0), "the next cell across is its own");
    assert_ne!(at(0, 0), at(0, 2), "and so is the next one down");
}
