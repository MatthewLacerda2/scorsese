//! What the two measures do and do not see.
//!
//! No ffmpeg here: this is arithmetic on frames built by hand. Between them the
//! two numbers have to catch every way a render can go wrong, and these tests
//! are where that claim is kept honest — most of them name a failure that the
//! *other* measure, or a more obvious version of the same measure, waves
//! through.

use scorsese_golden::{Tolerance, compare};
use scorsese_render::{Frame, Resolution};

fn raster(size: u32) -> Resolution {
    Resolution::new(size, size).expect("a legal raster")
}

fn solid(size: u32, red: u8, green: u8, blue: u8) -> Frame {
    let mut frame = Frame::black(raster(size));
    for pixel in frame.bytes_mut().chunks_exact_mut(4) {
        pixel.copy_from_slice(&[red, green, blue, u8::MAX]);
    }
    frame
}

/// Paints a square of `size` pixels into the top-left corner of a frame.
fn with_patch(frame: &mut Frame, size: usize, colour: (u8, u8, u8)) {
    let width = frame.resolution().width() as usize;
    for row in 0..size {
        for column in 0..size {
            let at = (row * width + column) * 4;
            frame.bytes_mut()[at..at + 3].copy_from_slice(&[colour.0, colour.1, colour.2]);
        }
    }
}

#[test]
fn a_frame_matches_itself_exactly() {
    let frame = solid(16, 200, 100, 50);
    let difference = compare(&frame, &frame).expect("same size");
    assert!(
        (difference.ssim - 1.0).abs() < 1e-9,
        "ssim was {}",
        difference.ssim
    );
    assert_eq!(difference.mean_error, 0.0);
    assert!(Tolerance::default().accepts(&difference));
}

#[test]
fn encoder_noise_on_flat_colour_is_tolerated() {
    // The tolerance exists for this: a couple of levels of drift from a
    // different x264 build is not a regression and must not read as one. The
    // green channel going from 0 to 1 is the case that makes textbook SSIM
    // constants unusable here.
    let difference = compare(&solid(16, 252, 0, 0), &solid(16, 250, 1, 0)).expect("same size");
    assert!(
        Tolerance::default().accepts(&difference),
        "a two-level shift should pass, scored {difference:?}"
    );
}

#[test]
fn noise_on_flat_black_is_tolerated() {
    // Every gap in a timeline renders black, so this is the most common frame in
    // the whole fixture set and the one most exposed to a scale-relative measure.
    let difference = compare(&solid(16, 0, 0, 0), &solid(16, 1, 1, 1)).expect("same size");
    assert!(
        Tolerance::default().accepts(&difference),
        "one level off black should pass, scored {difference:?}"
    );
}

#[test]
fn a_wrong_colour_is_caught() {
    let difference = compare(&solid(16, 254, 0, 0), &solid(16, 0, 0, 254)).expect("same size");
    assert!(
        !Tolerance::default().accepts(&difference),
        "red against blue must fail, scored {difference:?}"
    );
}

#[test]
fn a_uniform_brightness_shift_is_caught_by_mean_error_alone() {
    // Structure is unchanged, so SSIM barely moves. This is the failure mean
    // error is here for.
    let difference = compare(&solid(16, 200, 0, 0), &solid(16, 220, 0, 0)).expect("same size");
    assert!(
        difference.ssim > 0.95,
        "nothing structural changed: ssim {}",
        difference.ssim
    );
    assert!(
        difference.mean_error > 2.0,
        "mean error sees it: {}",
        difference.mean_error
    );
    assert!(!Tolerance::default().accepts(&difference));
}

#[test]
fn two_colours_of_the_same_brightness_are_caught_by_mean_error() {
    // Red at 255 and green at 130 have almost the same luma, so a measure built
    // on brightness cannot tell them apart. Comparing channels is what stops a
    // compositor painting the wrong colour and passing.
    let difference = compare(&solid(16, 255, 0, 0), &solid(16, 0, 130, 0)).expect("same size");
    assert!(
        difference.mean_error > 2.0,
        "the colours differ enormously: {}",
        difference.mean_error
    );
    assert!(!Tolerance::default().accepts(&difference));
}

#[test]
fn a_small_wrong_patch_is_caught_by_the_worst_block() {
    // 16 wrong pixels in 4096. Mean error is a third of a level and the *average*
    // block SSIM is 0.985 — both would pass. The block containing the patch
    // collapses, which is why the worst block is what gets gated.
    let clean = solid(64, 0, 0, 0);
    let mut spoiled = clean.clone();
    with_patch(&mut spoiled, 4, (255, 0, 0));

    let difference = compare(&spoiled, &clean).expect("same size");
    assert!(
        difference.mean_error < 1.0,
        "the average is barely touched: {}",
        difference.mean_error
    );
    assert!(
        !Tolerance::default().accepts(&difference),
        "and it still must not pass: {difference:?}"
    );
}

#[test]
fn frames_of_different_sizes_are_an_error_not_a_score() {
    let small = Frame::black(raster(16));
    let large = Frame::black(raster(32));
    let error = compare(&small, &large).expect_err("must not score these");
    assert_eq!(error.actual, small.resolution());
    assert_eq!(error.expected, large.resolution());
}
