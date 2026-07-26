//! Where sound lands on the timeline, and what happens when several land at
//! once.

use scorsese_core::Frames;
use scorsese_render::Note;

use crate::common::audio::{assert_audible, assert_silent, level, soundtrack, tone_asset};
use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{audio_track, clip, project, video_track};
use crate::{picture, render};

#[test]
fn a_clip_is_heard_where_it_sits_and_nowhere_else() {
    let tools = tools();
    let dir = fixture_dir("placement");
    let beep = tone_asset(&tools, &dir, "beep", 440, 1.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 3), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 90)]),
            // One second in, one second long, on a three-second timeline.
            audio_track("a1", vec![clip("m1", "beep", 30, 30)]),
        ],
    );

    let (out, report) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_silent(level(&heard, 0.2, 0.8), "before the clip");
    assert_audible(level(&heard, 1.2, 1.8), "during the clip");
    assert_silent(level(&heard, 2.2, 2.8), "after the clip");
    assert_eq!(report.seconds_of_audio, Some(3.0));
    assert!(report.notes.is_empty(), "nothing to warn about");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn two_tracks_playing_at_once_are_summed_rather_than_one_winning() {
    let tools = tools();
    let dir = fixture_dir("sum");
    // Quiet enough that two of them together stay inside full scale, so this
    // measures the sum rather than the clipper.
    let low = tone_asset(&tools, &dir, "low", 220, 2.0, 0.3, "");
    let high = tone_asset(&tools, &dir, "high", 880, 2.0, 0.3, "");
    let project = project(
        vec![picture(&tools, &dir, 2), low, high],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            // The second half of the timeline has both; the first has only one.
            audio_track("a1", vec![clip("m1", "low", 0, 60)]),
            audio_track("a2", vec![clip("m2", "high", 30, 30)]),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    let alone = level(&heard, 0.2, 0.8);
    let together = level(&heard, 1.2, 1.8);
    assert_audible(alone, "one track playing");
    assert!(
        together > alone * 1.2,
        "two tracks at once should be louder than one: {alone} then {together}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_hole_in_the_only_audio_track_is_silence_rather_than_a_shortened_soundtrack() {
    let tools = tools();
    let dir = fixture_dir("hole");
    let beep = tone_asset(&tools, &dir, "beep", 440, 1.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 3), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 90)]),
            audio_track(
                "a1",
                vec![clip("m1", "beep", 0, 30), clip("m2", "beep", 60, 30)],
            ),
        ],
    );

    let (out, report) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_audible(level(&heard, 0.2, 0.8), "the first beep");
    assert_silent(level(&heard, 1.2, 1.8), "the hole between them");
    assert_audible(level(&heard, 2.2, 2.8), "the second beep");
    // The hole occupies time rather than collapsing: the soundtrack still runs
    // the length of the picture.
    assert_eq!(report.seconds_of_audio, Some(3.0));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn audio_running_past_the_last_picture_is_cut_there_and_said_so() {
    let tools = tools();
    let dir = fixture_dir("trimmed");
    let beep = tone_asset(&tools, &dir, "beep", 440, 3.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 1), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 30)]),
            audio_track("a1", vec![clip("m1", "beep", 0, 90)]),
        ],
    );

    let (out, report) = render(&tools, &project, &dir);

    assert_eq!(
        report.notes,
        [Note::AudioTrimmed {
            audio_end: Frames(90),
            timeline_end: Frames(30),
        }]
    );
    assert_eq!(report.seconds_of_audio, Some(1.0));
    let heard = soundtrack(&tools, &out);
    assert!(
        heard.len() < 48_000 * 2,
        "the soundtrack should stop with the picture, not run to three seconds"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_project_with_no_audio_gets_a_file_with_no_soundtrack() {
    let tools = tools();
    let dir = fixture_dir("silent");
    let project = project(
        vec![picture(&tools, &dir, 1)],
        vec![video_track("v1", vec![clip("c1", "picture", 0, 30)])],
    );

    let (out, report) = render(&tools, &project, &dir);

    assert_eq!(report.seconds_of_audio, None);
    assert!(
        soundtrack(&tools, &out).is_empty(),
        "there should be no audio stream to decode at all"
    );
    std::fs::remove_dir_all(&dir).ok();
}
