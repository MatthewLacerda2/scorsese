//! Grain through a whole render, which is where its determinism has to hold.
//!
//! The compositor's own tests pin the arithmetic at the byte, before an encoder
//! has seen it. These pin the two claims only a real render can make: that
//! rendering the same project twice delivers the same frames, and that a
//! `--range` out of the middle of a timeline puts the same grain on the same
//! timeline frame the whole render would have. The second is the one a
//! plausible mistake breaks — a noise field seeded from the render's own output
//! counter passes every single-render test and quietly makes every partial
//! render a different picture.
//!
//! **Heavy on purpose.** x264 at `-crf 18` will quantise a light grain away
//! entirely on a flat plate, which is a fact about delivering noise through a
//! lossy codec rather than about this code — so these render an amount nothing
//! can mistake for encoder noise, and assert distances rather than exact
//! colours.

use std::path::Path;

use scorsese_core::{Fps, Grade, Project};
use scorsese_render::{Frame, FrameRange, Resolution, frames};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};
use crate::{RASTER, colour_asset, render};

/// Far more grain than anybody would author, so that what survives the encoder
/// is unmistakably the grain and not the encoder.
const AMOUNT: f64 = 0.5;

/// A mid-grey plate under grain and nothing else — grey because the grain is
/// weighted toward the midtones, so this is where there is most of it.
fn grained(dir: &Path, amount: f64) -> Project {
    let plate = colour_asset(&tools(), dir, "gray", "64x64", 1);
    let mut shot = clip("c-plate", "gray", 0, 30);
    shot.grade = Grade {
        grain: amount,
        ..Grade::NEUTRAL
    };
    project(vec![plate], vec![video_track("v1", vec![shot])])
}

/// One frame of a rendered file, as pixels.
fn frame_of(file: &Path, index: u64) -> Frame {
    let raster = Resolution::new(RASTER.0, RASTER.1).expect("the fixture raster is a legal one");
    frames::extract(&tools(), file, index, raster).expect("the frame comes back out")
}

/// The mean absolute difference per channel between two frames, in levels.
fn apart(one: &Frame, other: &Frame) -> f64 {
    let total: u64 = one
        .bytes()
        .iter()
        .zip(other.bytes())
        .map(|(a, b)| u64::from(a.abs_diff(*b)))
        .sum();
    total as f64 / one.bytes().len() as f64
}

#[test]
fn the_same_project_renders_the_same_grain_twice() {
    // Frame for frame and byte for byte, because the noise is a hash and not a
    // generator: nothing is seeded from a clock, an address, or the order the
    // workers happened to take their jobs in.
    let tools = tools();
    let (first, second) = (fixture_dir("grain-once"), fixture_dir("grain-again"));
    let one = render(
        &tools,
        &grained(&first, AMOUNT),
        &first,
        FrameRange::ALL,
        Fps::THIRTY,
    )
    .0;
    let two = render(
        &tools,
        &grained(&second, AMOUNT),
        &second,
        FrameRange::ALL,
        Fps::THIRTY,
    )
    .0;

    for frame in [0, 7, 29] {
        assert_eq!(
            frame_of(&one, frame).bytes(),
            frame_of(&two, frame).bytes(),
            "frame {frame} differs between two renders of one project"
        );
    }
    std::fs::remove_dir_all(&first).ok();
    std::fs::remove_dir_all(&second).ok();
}

#[test]
fn the_grain_moves_from_one_frame_to_the_next() {
    // The source is one flat colour throughout, so anything differing between
    // two frames is the grain having moved. Static grain reads as dirt on the
    // lens rather than as film, and is what a frame index that never reached
    // the grade would produce.
    let dir = fixture_dir("grain-moves");
    let out = render(
        &tools(),
        &grained(&dir, AMOUNT),
        &dir,
        FrameRange::ALL,
        Fps::THIRTY,
    )
    .0;

    let moved = apart(&frame_of(&out, 0), &frame_of(&out, 1));
    assert!(
        moved > 2.0,
        "frames 0 and 1 are only {moved:.2} levels apart"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_partial_render_puts_the_same_grain_on_the_same_timeline_frame() {
    let tools = tools();
    let (whole, part) = (fixture_dir("grain-whole"), fixture_dir("grain-part"));
    let all = render(
        &tools,
        &grained(&whole, AMOUNT),
        &whole,
        FrameRange::ALL,
        Fps::THIRTY,
    )
    .0;
    let range: FrameRange = "20:30".parse().expect("a range");
    let (some, report) = render(&tools, &grained(&part, AMOUNT), &part, range, Fps::THIRTY);
    assert_eq!(report.frames, 10, "ten frames, not thirty");

    // Not exact equality, and it cannot be: a ten-frame encode and a
    // thirty-frame one make different decisions about the same pixels. What is
    // asserted is that they are far closer to each other than either is to a
    // *neighbouring* frame of the same render — the whole difference between a
    // grain hashed from the clip's own elapsed frame and one counted off the
    // output.
    let same = apart(&frame_of(&all, 20), &frame_of(&some, 0));
    let next = apart(&frame_of(&all, 20), &frame_of(&all, 21));
    assert!(
        same < next / 2.0,
        "timeline frame 20 should carry one grain however much of the timeline \
         was rendered: {same:.2} levels apart, where the next frame along is {next:.2}"
    );
    std::fs::remove_dir_all(&whole).ok();
    std::fs::remove_dir_all(&part).ok();
}

#[test]
fn a_clip_with_no_grain_renders_the_picture_it_always_did() {
    // The other half of the format claim, and what lets every reference frame
    // in the golden set keep meaning what it meant: a project that says nothing
    // about grain is untouched by the feature.
    let dir = fixture_dir("grain-none");
    let out = render(
        &tools(),
        &grained(&dir, 0.0),
        &dir,
        FrameRange::ALL,
        Fps::THIRTY,
    )
    .0;

    assert_eq!(
        frame_of(&out, 0).bytes(),
        frame_of(&out, 1).bytes(),
        "an ungrained flat plate is the same picture on every frame"
    );
    std::fs::remove_dir_all(&dir).ok();
}
