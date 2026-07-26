//! The sound that came in the video file, rather than beside it.

use crate::common::audio::{
    SHOT_LEVEL, assert_audible, assert_silent, level, soundtrack, talking_picture, tone_asset,
    with_volume,
};
use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{audio_track, clip, project, video_track};
use crate::{picture, render};

#[test]
fn a_video_clips_own_sound_is_heard() {
    // Import a video with dialogue on it, put it on the timeline, render: the
    // dialogue is in the file. Before this it was silently gone, with nothing
    // said about it.
    let tools = tools();
    let dir = fixture_dir("embedded");
    let interview = talking_picture(&tools, &dir, "interview", 2, 440);
    let project = project(
        vec![interview],
        vec![video_track("v1", vec![clip("c1", "interview", 0, 60)])],
    );

    let (out, report) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_audible(level(&heard, 0.2, 1.8), "a video clip's own sound");
    assert_eq!(report.seconds_of_audio, Some(2.0));
    assert!(
        report.notes.is_empty(),
        "nothing to warn about: {:?}",
        report.notes
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn volume_zero_mutes_a_video_clip_like_any_other() {
    // No new field and no new concept: muting a talking head under a voiceover
    // is the same keyframe that mutes a music bed.
    let tools = tools();
    let dir = fixture_dir("embedded-mute");
    let interview = talking_picture(&tools, &dir, "interview", 2, 440);
    let project = project(
        vec![interview],
        vec![video_track(
            "v1",
            vec![with_volume(clip("c1", "interview", 0, 60), &[(0, 0.0)])],
        )],
    );

    let (out, report) = render(&tools, &project, &dir);

    assert_silent(level(&soundtrack(&tools, &out), 0.2, 1.8), "a muted shot");
    assert_eq!(
        report.seconds_of_audio,
        Some(2.0),
        "muted is not absent: the stream is still there"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_silent_video_contributes_nothing_and_warns_about_nothing() {
    // The other half of the promise. A file with no audio stream is not a
    // problem to report — it is a silent film, and reporting it every render
    // would train an agent to ignore the notes that matter.
    let tools = tools();
    let dir = fixture_dir("embedded-silent");
    let project = project(
        vec![picture(&tools, &dir, 1)],
        vec![video_track("v1", vec![clip("c1", "picture", 0, 30)])],
    );

    let (out, report) = render(&tools, &project, &dir);

    assert_eq!(report.seconds_of_audio, None, "no soundtrack at all");
    assert!(soundtrack(&tools, &out).is_empty());
    assert!(
        report.notes.is_empty(),
        "nothing to warn about: {:?}",
        report.notes
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_shots_own_sound_and_a_music_bed_play_together() {
    // The mix does not care which track a sound came from: a video clip is one
    // more thing to add, and two of them at once are summed like any others.
    let tools = tools();
    let dir = fixture_dir("embedded-and-bed");
    let interview = talking_picture(&tools, &dir, "interview", 2, 440);
    // At the shot's own level, so the two together stay inside full scale and
    // this measures the sum rather than the clipper.
    let bed = tone_asset(&tools, &dir, "bed", 880, 2.0, SHOT_LEVEL, "");
    let project = project(
        vec![interview, bed],
        vec![
            video_track("v1", vec![clip("c1", "interview", 0, 60)]),
            // The bed covers only the second half, so the first half is the
            // shot's own sound on its own.
            audio_track("a1", vec![clip("m1", "bed", 30, 30)]),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    let alone = level(&heard, 0.2, 0.8);
    let together = level(&heard, 1.2, 1.8);
    assert_audible(alone, "the shot's own sound");
    assert!(
        together > alone * 1.2,
        "a bed over a shot should be louder than the shot alone: {alone} then {together}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_shot_whose_sound_starts_later_is_cut_where_its_own_clip_is() {
    // A video clip's boundaries cut the mix now, the same way an audio clip's
    // always did — it is one more audible thing on the timeline.
    let tools = tools();
    let dir = fixture_dir("embedded-placed");
    let interview = talking_picture(&tools, &dir, "interview", 1, 440);
    let project = project(
        vec![picture(&tools, &dir, 3), interview],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 90)]),
            // On a second video track, a second in and a second long.
            video_track("v2", vec![clip("c2", "interview", 30, 30)]),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_silent(level(&heard, 0.2, 0.8), "before the shot");
    assert_audible(level(&heard, 1.2, 1.8), "during it");
    assert_silent(level(&heard, 2.2, 2.8), "after it");
    std::fs::remove_dir_all(&dir).ok();
}
