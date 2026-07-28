//! One frame, composited and handed back — what a preview asks for.
//!
//! What matters here is not that a still looks like something, but that it is
//! the *same* picture the delivered file carries. A preview drawn any other way
//! would be a preview that can lie, and then looking at it proves nothing — so
//! the first test below compares a still against the frame a real render put in
//! a real file, at the same instant.

use scorsese_core::{Fps, Frames};
use scorsese_render::{Frame, FrameRange, Renderer};

use crate::common::ffmpeg::{fixture_dir, mean_rgb, tools};
use crate::common::{clip, project, sketch_asset, video_track};
use crate::{GREEN, RED, assert_colour, colour_asset, render, settings};

/// The average colour of a frame that is still in memory, in the same terms
/// [`mean_rgb`] reports one that reached a file — so the two can be compared.
fn mean_of(frame: &Frame) -> (u8, u8, u8) {
    let pixels = frame.bytes();
    let mean = |channel: usize| -> u8 {
        let total: u64 = pixels
            .iter()
            .skip(channel)
            .step_by(4)
            .map(|&value| u64::from(value))
            .sum();
        (total / (pixels.len() as u64 / 4)) as u8
    };
    (mean(0), mean(1), mean(2))
}

/// The whole point of the still living inside the renderer. Not "a still is
/// red" — that would pass for a preview with its own idea of red — but "a still
/// is what the encode wrote", which only one compositing path can satisfy.
#[test]
fn a_still_is_the_frame_the_render_writes_at_the_same_instant() {
    let tools = tools();
    let dir = fixture_dir("still-agrees");
    let project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 1)],
        vec![video_track("v1", vec![clip("c1", "red", 0, 20)])],
    );

    let (out, _) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);
    let still = Renderer::new(&tools, settings(Fps::THIRTY))
        .still(&project, &dir, Frames(10))
        .expect("frame 10 is inside the edit");

    assert_colour(mean_of(&still), RED, "the still");
    assert_colour(mean_of(&still), mean_rgb(&tools, &out, 10), "the still");
    assert_eq!(still.resolution(), settings(Fps::THIRTY).resolution);
    std::fs::remove_dir_all(&dir).ok();
}

/// Scrubbing across a cut is the thing a preview exists for, so the frame asked
/// for has to be the frame drawn — not the segment's first, and not the
/// timeline's.
#[test]
fn a_still_shows_whichever_side_of_a_cut_it_is_asked_for() {
    let tools = tools();
    let dir = fixture_dir("still-cut");
    let project = project(
        vec![
            colour_asset(&tools, &dir, "red", "64x64", 1),
            colour_asset(&tools, &dir, "green", "64x64", 1),
        ],
        vec![video_track(
            "v1",
            vec![clip("c1", "red", 0, 10), clip("c2", "green", 10, 10)],
        )],
    );
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY));

    let before = renderer
        .still(&project, &dir, Frames(5))
        .expect("frame 5 is on the first clip");
    let after = renderer
        .still(&project, &dir, Frames(15))
        .expect("frame 15 is on the second");

    assert_colour(mean_of(&before), RED, "before the cut");
    assert_colour(mean_of(&after), GREEN, "after the cut");
    std::fs::remove_dir_all(&dir).ok();
}

/// A preview of an unrealised cut costs nothing: no provider, no file, no
/// money. The card comes from the same code the render draws, so this needs no
/// special case anywhere in the preview.
#[test]
fn a_sketch_stills_as_its_card_with_nothing_generated() {
    let tools = tools();
    let dir = fixture_dir("still-sketch");
    let project = project(
        vec![sketch_asset("veo1")],
        vec![video_track("v1", vec![clip("c1", "veo1", 0, 20)])],
    );

    let still = Renderer::new(&tools, settings(Fps::THIRTY))
        .still(&project, &dir, Frames(4))
        .expect("a sketch clip stills");

    let (red, green, blue) = mean_of(&still);
    let lean = red.abs_diff(green).max(green.abs_diff(blue));
    assert!(
        lean <= 6 && (20..=140).contains(&red),
        "expected a gray card, found {:?}",
        (red, green, blue)
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// There is no such instant in the edit, so there is no picture of it. Handing
/// back black would be inventing one, and a caller that cannot tell the
/// difference is a caller that will show black and call it the film.
#[test]
fn a_frame_past_the_end_has_no_still() {
    let tools = tools();
    let dir = fixture_dir("still-past-end");
    let project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 1)],
        vec![video_track("v1", vec![clip("c1", "red", 0, 20)])],
    );

    let refused = Renderer::new(&tools, settings(Fps::THIRTY)).still(&project, &dir, Frames(20));

    assert!(refused.is_err(), "frame 20 is past a 20-frame edit");
    std::fs::remove_dir_all(&dir).ok();
}
