//! Volume as an ordinary keyframed property, and where a clip opens in its
//! source.

use crate::common::audio::{
    assert_audible, assert_silent, level, soundtrack, tone_asset, with_volume,
};
use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{audio_track, clip, project, video_track};
use crate::{picture, render};

#[test]
fn a_volume_ramp_actually_ramps() {
    let tools = tools();
    let dir = fixture_dir("ramp");
    let beep = tone_asset(&tools, &dir, "beep", 440, 3.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 3), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 90)]),
            audio_track(
                "a1",
                // Silent at the start of the clip, full by its end. Ordinary
                // keyframes: there is no fade mechanism, and this is the proof.
                vec![with_volume(
                    clip("m1", "beep", 0, 90),
                    &[(0, 0.0), (90, 1.0)],
                )],
            ),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    let (early, middle, late) = (
        level(&heard, 0.1, 0.3),
        level(&heard, 1.4, 1.6),
        level(&heard, 2.7, 2.9),
    );
    assert!(
        early < middle && middle < late,
        "the ramp should climb throughout: {early}, then {middle}, then {late}"
    );
    assert_audible(late, "the end of the ramp");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_ramp_is_timed_from_the_clip_rather_than_the_timeline() {
    let tools = tools();
    let dir = fixture_dir("ramp-offset");
    let beep = tone_asset(&tools, &dir, "beep", 440, 2.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 3), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 90)]),
            audio_track(
                "a1",
                // The clip starts a second in, and its ramp starts with it. Timed
                // from the timeline instead, the whole fade would be over before
                // the clip was ever heard.
                vec![with_volume(
                    clip("m1", "beep", 30, 60),
                    &[(0, 0.0), (60, 1.0)],
                )],
            ),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    // Timed from the clip, the ramp has barely started here and is finished
    // there. Timed from the timeline, it would already be half up when the clip
    // was first heard — so the gap between these two is the whole assertion,
    // and a level near zero at 1.1s is not: a ramp that has run 5% of its
    // length is quiet, not silent.
    let (opening, closing) = (level(&heard, 1.1, 1.3), level(&heard, 2.7, 2.9));
    assert_audible(closing, "the clip's own last moments");
    assert!(
        opening * 4.0 < closing,
        "the ramp should start with the clip, not the timeline: {opening} then {closing}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn volume_zero_is_how_a_clip_is_muted() {
    let tools = tools();
    let dir = fixture_dir("mute");
    let beep = tone_asset(&tools, &dir, "beep", 440, 2.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 2), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            // A single keyframe holds for the whole clip, so muting needs no
            // flag on the clip and no concept of its own.
            audio_track(
                "a1",
                vec![with_volume(clip("m1", "beep", 0, 60), &[(0, 0.0)])],
            ),
        ],
    );

    let (out, report) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_silent(level(&heard, 0.2, 1.8), "a muted clip");
    assert_eq!(
        report.seconds_of_audio,
        Some(2.0),
        "muted is not the same as absent: the stream is still there"
    );
    std::fs::remove_dir_all(&dir).ok();
}
