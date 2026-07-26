//! Where a clip opens in its own media, and what happens when it runs out.

use scorsese_render::Note;

use crate::common::audio::{
    assert_audible, assert_silent, level, soundtrack, tone_asset, with_volume,
};
use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{audio_track, clip, clip_from, project, video_track};
use crate::{picture, render};

#[test]
fn source_in_skips_into_the_audio_rather_than_the_timeline() {
    let tools = tools();
    let dir = fixture_dir("source-in");
    // A second of silence, then a second of tone.
    let beep = tone_asset(&tools, &dir, "beep", 440, 1.0, 0.8, ",adelay=1000:all=1");
    let project = project(
        vec![picture(&tools, &dir, 2), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track(
                "a1",
                vec![
                    // The fixture's own silent half...
                    clip("m1", "beep", 0, 30),
                    // ...and its tone, reached by skipping a second in.
                    clip_from("m2", "beep", 30, 30, 30),
                ],
            ),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_silent(level(&heard, 0.1, 0.9), "the source's silent opening");
    assert_audible(level(&heard, 1.1, 1.9), "the source's tone, skipped to");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_bed_carries_on_across_a_cut_rather_than_restarting() {
    let tools = tools();
    let dir = fixture_dir("resume");
    // A second of silence, then a second of tone — a bed that sounds different
    // depending on how far into it you are. A flat one would hide this entirely:
    // restarting a constant tone is indistinguishable from carrying on with it.
    let bed = tone_asset(&tools, &dir, "bed", 440, 1.0, 0.8, ",adelay=1000:all=1");
    let marker = tone_asset(&tools, &dir, "marker", 880, 1.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 2), bed, marker],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track("a1", vec![clip("m1", "bed", 0, 60)]),
            // Muted, and here only to put a cut halfway through the bed: a clip
            // entering on another track splits the timeline, and the bed has to
            // pick up where it left off rather than starting again.
            audio_track(
                "a2",
                vec![with_volume(clip("m2", "marker", 30, 30), &[(0, 0.0)])],
            ),
        ],
    );

    let (out, _) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_silent(level(&heard, 0.1, 0.9), "the bed's own silent opening");
    assert_audible(
        level(&heard, 1.1, 1.9),
        "the bed's tone, which only a bed that carried on ever reaches",
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_clip_outlasting_its_source_goes_silent_and_says_so() {
    let tools = tools();
    let dir = fixture_dir("short");
    let beep = tone_asset(&tools, &dir, "beep", 440, 1.0, 0.8, "");
    let project = project(
        vec![picture(&tools, &dir, 2), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            // Two seconds of clip over one second of source.
            audio_track("a1", vec![clip("m1", "beep", 0, 60)]),
        ],
    );

    let (out, report) = render(&tools, &project, &dir);
    let heard = soundtrack(&tools, &out);

    assert_audible(level(&heard, 0.2, 0.8), "the part the source covers");
    assert_silent(level(&heard, 1.2, 1.8), "the part it does not");
    let missing = report.notes.iter().find_map(|note| match note {
        Note::AudioRanShort { clip, missing_ms } if clip == "m1" => Some(*missing_ms),
        _ => None,
    });
    let missing =
        missing.unwrap_or_else(|| panic!("expected a short-source note: {:?}", report.notes));
    assert!(
        (900..=1100).contains(&missing),
        "about a second should be missing, not {missing}ms"
    );
    std::fs::remove_dir_all(&dir).ok();
}
