//! Turning a layer about one of its own axes.
//!
//! A flip is a negative scale about the layer's centre, so most of it falls out
//! of arithmetic that was already there. What does *not* fall out is which axis
//! goes with which: a turn about the **vertical** axis changes the
//! **horizontal** extent, and every test here that names an axis is holding
//! that crossover in place.

mod common;

use scorsese_compositor::{BYTES_PER_PIXEL, Frame, Properties};

use common::{BLACK, BLUE, CENTRE, RED, SIZE, assert_pixel, composited, raster, solid, with};

/// Sixty degrees, whose cosine is exactly a half — so the layer ends up half
/// its width or half its height, and lands on whole pixels either way.
const HALF_TURNED: f64 = 60.0;

/// A frame painted per pixel. A flip is only *visible* on a layer asymmetric
/// across the axis it turns about: a uniform one is identical to its own
/// mirror, so 180° on it would assert nothing at all.
fn painted(colour: impl Fn(u32, u32) -> (u8, u8, u8)) -> Frame {
    let mut frame = Frame::black(raster());
    for (at, pixel) in frame
        .bytes_mut()
        .chunks_exact_mut(BYTES_PER_PIXEL)
        .enumerate()
    {
        let (red, green, blue) = colour(at as u32 % SIZE, at as u32 / SIZE);
        pixel.copy_from_slice(&[red, green, blue, u8::MAX]);
    }
    frame
}

fn flipped(source: &Frame, flip: (f64, f64)) -> Frame {
    composited(&[with(
        source,
        Properties {
            flip,
            ..Properties::default()
        },
    )])
}

/// The crossover, in the direction that reads as a page-turn.
#[test]
fn a_turn_about_the_vertical_axis_narrows_the_layer() {
    let canvas = flipped(&solid(RED), (0.0, HALF_TURNED));
    assert_pixel(&canvas, CENTRE, RED, "the narrowed layer");
    assert_pixel(&canvas, (4, SIZE / 2), BLACK, "vacated on the left");
    assert_pixel(&canvas, (SIZE - 4, SIZE / 2), BLACK, "and on the right");
    assert_pixel(&canvas, (SIZE / 2, 4), RED, "full height, unsquashed");
}

/// The same crossover the other way round. Its own test rather than a second
/// assertion above, because a build with the two axes swapped would pass one of
/// them, and a failure should say which way round it went wrong.
#[test]
fn a_turn_about_the_horizontal_axis_squashes_it_instead() {
    let canvas = flipped(&solid(RED), (HALF_TURNED, 0.0));
    assert_pixel(&canvas, CENTRE, RED, "the squashed layer");
    assert_pixel(&canvas, (SIZE / 2, 4), BLACK, "vacated above");
    assert_pixel(&canvas, (SIZE / 2, SIZE - 4), BLACK, "and below");
    assert_pixel(&canvas, (4, SIZE / 2), RED, "full width, unnarrowed");
}

/// Edge on is *absent*, not a line of smeared colour down the middle. This is
/// the frame a mid-flip substitution swaps its clips on, so anything left
/// visible here would show up as a flash in the finished cut.
#[test]
fn edge_on_the_layer_is_gone_entirely() {
    for flip in [(0.0, 90.0), (90.0, 0.0), (0.0, -90.0)] {
        let canvas = flipped(&solid(RED), flip);
        assert_pixel(&canvas, CENTRE, BLACK, "edge on, at the centre");
        assert_pixel(&canvas, (SIZE / 2 - 1, SIZE / 2), BLACK, "and beside it");
    }
}

/// The back of the card. Nothing implements a backface: `cos 180°` is `−1`, and
/// a negative scale about the centre is already a mirror.
#[test]
fn half_a_turn_about_the_vertical_axis_mirrors_left_to_right() {
    let source = painted(|x, _| if x < SIZE / 2 { RED } else { BLUE });

    let front = flipped(&source, (0.0, 0.0));
    assert_pixel(&front, (4, SIZE / 2), RED, "face on, red on the left");
    assert_pixel(&front, (SIZE - 4, SIZE / 2), BLUE, "and blue on the right");

    let back = flipped(&source, (0.0, 180.0));
    assert_pixel(&back, (4, SIZE / 2), BLUE, "mirrored, blue on the left");
    assert_pixel(&back, (SIZE - 4, SIZE / 2), RED, "and red on the right");
}

/// The other axis mirrors the other way, and the layer stays over the middle of
/// the frame rather than swinging off it — which is what a flip anchored on an
/// edge would do.
#[test]
fn half_a_turn_about_the_horizontal_axis_mirrors_top_to_bottom() {
    let source = painted(|x, y| match (y < SIZE / 2, x < SIZE / 2) {
        (true, _) => BLACK,
        (false, true) => RED,
        (false, false) => BLUE,
    });

    let back = flipped(&source, (180.0, 0.0));
    assert_pixel(&back, (4, 4), RED, "what was at the bottom is now on top");
    assert_pixel(&back, (4, SIZE - 4), BLACK, "and the black is beneath");
    assert_pixel(&back, (SIZE - 4, 4), BLUE, "left to right is untouched");
}

/// All the way round is where it started — no wrapping and no special case,
/// only `cos 360°` being `1` — so the compositor may copy it rather than
/// rasterise it.
#[test]
fn a_full_turn_is_the_layer_as_it_arrived() {
    let round = Properties {
        flip: (360.0, -360.0),
        ..Properties::default()
    };
    assert!(round.is_identity(), "{round:?}");
    assert_eq!(round.effective_scale(), (1.0, 1.0));
}
