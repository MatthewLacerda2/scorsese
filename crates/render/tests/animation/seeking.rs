//! A held animation opened part-way through is where the whole render has it.
//!
//! A decode of a held gif that begins anywhere but its clip's start — a still,
//! a partial render, a segment a cut on another track opens — is told how far
//! into the animation that instant falls. Before it was, each of them started
//! the gif from its first frame, so a preview showed a frame the delivered file
//! did not contain at that instant and a re-render of a range was a different
//! edit (#374).
//!
//! The fixture is red for half a second and blue for the next, at 30fps: frames
//! 0–14 red, 15–29 blue, and from 30 the same again.

use scorsese_core::Frames;
use scorsese_render::{FrameRange, Renderer};

use crate::common::ffmpeg::{fixture_dir, mean_rgb, tools};
use crate::common::{clip, project, video_track};
use crate::{BLUE, RED, assert_colour, gif, rendered_frame, settings};

/// The mean colour of a frame in memory, in the terms [`mean_rgb`] uses.
fn mean_of(frame: &scorsese_render::Frame) -> (u8, u8, u8) {
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

/// The issue's own assertion: a still at frame N is frame N of the file, for a
/// source that moves — in the first loop, and in a later one, which is where
/// the animation's measured length comes in.
#[test]
fn a_still_of_an_animated_image_is_the_frame_the_render_writes() {
    let tools = tools();
    let dir = fixture_dir("animated-still");
    let asset = gif(&tools, &dir, "wave", &["red", "blue"]);
    let project = project(
        vec![asset],
        vec![video_track("v1", vec![clip("c1", "wave", 0, 60)])],
    );
    let renderer = Renderer::new(&tools, settings());

    for (at, expected) in [(5, RED), (20, BLUE), (40, RED), (50, BLUE)] {
        let still = renderer
            .still(&project, &dir, Frames(at))
            .expect("the frame is inside the edit");
        let what = format!("the still at frame {at}");
        assert_colour(mean_of(&still), expected, &what);
        assert_colour(
            mean_of(&still),
            rendered_frame(&project, &dir, &tools, at),
            &what,
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// A range re-rendered is a repair, so it has to be the same frames the whole
/// render put there — continuing the animation, not restarting it.
#[test]
fn a_partial_render_continues_the_animation_where_the_full_one_has_it() {
    let tools = tools();
    let dir = fixture_dir("animated-partial");
    let asset = gif(&tools, &dir, "wave", &["red", "blue"]);
    let project = project(
        vec![asset],
        vec![video_track("v1", vec![clip("c1", "wave", 0, 60)])],
    );
    let part = dir.join("part.mp4");
    Renderer::new(&tools, settings())
        .render(
            &project,
            &dir,
            FrameRange::new(Frames(18), Some(Frames(48))).expect("a range"),
            &part,
        )
        .expect("the partial render succeeds");

    for offset in [0, 5, 13, 25] {
        let what = format!("frame {offset} of the range, {} of the edit", 18 + offset);
        assert_colour(
            mean_rgb(&tools, &part, offset),
            rendered_frame(&project, &dir, &tools, 18 + offset),
            &what,
        );
    }
    assert_colour(mean_rgb(&tools, &part, 0), BLUE, "the range opens mid-blue");
    std::fs::remove_dir_all(&dir).ok();
}

/// A clip on another track ending under the gif splits it into two segments,
/// and the second one opens mid-clip. The picture must not notice the cut.
#[test]
fn a_cut_on_another_track_does_not_restart_the_animation() {
    let tools = tools();
    let dir = fixture_dir("animated-cut");
    let wave = gif(&tools, &dir, "wave", &["red", "blue"]);
    let under = gif(&tools, &dir, "under", &["green"]);
    let project = project(
        vec![wave, under],
        vec![
            video_track("v1", vec![clip("c0", "under", 0, 20)]),
            video_track("v2", vec![clip("c1", "wave", 0, 45)]),
        ],
    );

    assert_colour(
        rendered_frame(&project, &dir, &tools, 20),
        BLUE,
        "the first frame after the cut is still blue",
    );
    assert_colour(
        rendered_frame(&project, &dir, &tools, 32),
        RED,
        "and the loop comes round when the gif says, not when the cut did",
    );
    std::fs::remove_dir_all(&dir).ok();
}
