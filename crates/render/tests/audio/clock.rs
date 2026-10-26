//! A source's sound on the source's own clock.
//!
//! The mix decodes to raw samples, which carry no timestamps; these pin that a
//! stream starting away from its container's zero still lands where the
//! picture says it does.

use crate::common::audio::{
    assert_audible, assert_silent, late_talking_picture, level, soundtrack,
};
use crate::common::ffmpeg::tools;
use crate::common::scratch;
use crate::common::{clip_from, project, sped, video_track};
use crate::render;
use scorsese_core::Clip;

#[test]
fn sound_that_starts_after_its_picture_keeps_its_place_on_the_files_clock() {
    // #1601: raw samples carry no timestamps, so a file whose audio stream
    // starts half a second after its picture was mixed half a second early —
    // lips and sound apart by the offset, at every seek.
    let tools = tools();
    let dir = scratch::dir("embedded-late");
    let late = late_talking_picture(&tools, &dir, "phone", 2, 440, 0.5);
    let project = project(
        vec![late],
        vec![
            video_track("v1", vec![Clip::at("c1", "phone", 0, 45)]),
            // The same file again, entered 0.2 s (six frames) in.
            video_track("v2", vec![clip_from("c2", "phone", 45, 45, 6)]),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_silent(level(&heard, 0.05, 0.4), "the half second before its sound");
    assert_audible(level(&heard, 0.6, 1.4), "its sound, at half a second");
    assert_silent(
        level(&heard, 1.55, 1.75),
        "the same gap, 0.2 s shorter after a seek of 0.2 s",
    );
    assert_audible(level(&heard, 1.9, 2.7), "its sound, 0.3 s into the clip");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_late_sound_sped_up_keeps_its_place_on_the_sped_clock() {
    // The speed acts on the aligned stream: at 2× the half second of silence
    // before the sound is a quarter second, as it is in the picture.
    let tools = tools();
    let dir = scratch::dir("embedded-late-sped");
    let late = late_talking_picture(&tools, &dir, "phone", 2, 440, 0.5);
    let project = project(
        vec![late],
        vec![video_track(
            "v1",
            vec![sped(2.0, Clip::at("c1", "phone", 0, 30))],
        )],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_silent(
        level(&heard, 0.03, 0.2),
        "the quarter second before its sound",
    );
    assert_audible(level(&heard, 0.32, 0.7), "its sound, at a quarter second");
    std::fs::remove_dir_all(&dir).ok();
}
