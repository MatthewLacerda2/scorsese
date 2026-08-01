//! A clip's speed, heard.
//!
//! Two questions, and the format has committed to an answer for both: sound
//! runs at the same rate as the picture it belongs to, and its pitch goes with
//! it. The second is the one worth a test — it is a decision rather than an
//! inevitability, and holding the pitch is what the deferred alternative would
//! have done.

use crate::common::audio::{assert_audible, assert_silent, level, pitch, soundtrack, tone_asset};
use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{audio_track, clip, project, sped, video_track};
use crate::{picture, render};

#[test]
fn a_sped_clip_reaches_the_far_end_of_its_source_sooner() {
    let tools = tools();
    let dir = fixture_dir("speed-sound");
    // A second of silence, then a second of tone — a source that sounds
    // different depending on how far into it you are. A flat one would prove
    // nothing: a tone played faster is still a tone.
    let beep = tone_asset(&tools, &dir, "beep", 440, 1.0, 0.8, ",adelay=1000:all=1");
    let project = project(
        vec![picture(&tools, &dir, 2), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            // Two seconds of source in one second of timeline.
            audio_track("a1", vec![sped(2.0, clip("m1", "beep", 0, 30))]),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_silent(level(&heard, 0.05, 0.4), "the source's silent opening");
    assert_audible(
        level(&heard, 0.6, 0.95),
        "its tone, which at 1× would still be half a second away",
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The decision this pins: playing a clip faster raises it. It is what Filmora
/// does and what a person who picked 2× expects to hear, and it is what makes
/// the mixer's job arithmetic rather than a time-stretch algorithm.
#[test]
fn playing_a_clip_faster_takes_its_pitch_up_with_it() {
    let tools = tools();
    let dir = fixture_dir("speed-pitch");
    let beep = tone_asset(&tools, &dir, "beep", 440, 2.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 2), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track(
                "a1",
                vec![
                    clip("m1", "beep", 0, 30),
                    sped(2.0, clip("m2", "beep", 30, 30)),
                ],
            ),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    let plain = pitch(&heard, 0.1, 0.9);
    let quick = pitch(&heard, 1.1, 1.9);
    assert!(
        (400.0..=480.0).contains(&plain),
        "the ordinary clip should be about 440 Hz, and read {plain}"
    );
    assert!(
        (800.0..=960.0).contains(&quick),
        "at 2× it should be about an octave up, and read {quick}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The other direction, and the same rule. A slowed clip is lower, and the far
/// end of its source is somewhere the clip never reaches.
#[test]
fn playing_a_clip_slower_takes_its_pitch_down() {
    let tools = tools();
    let dir = fixture_dir("speed-slow");
    let beep = tone_asset(&tools, &dir, "beep", 880, 2.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 2), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track("a1", vec![sped(0.5, clip("m1", "beep", 0, 60))]),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    let heard_at = pitch(&heard, 0.2, 1.8);
    assert!(
        (400.0..=480.0).contains(&heard_at),
        "at 0.5× an 880 Hz tone should be about 440, and read {heard_at}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
