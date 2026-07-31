//! Which clip put the level there.

use crate::common::audio::{soundtrack, tone_asset};
use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{audio_track, clip, project, video_track};
use crate::{picture, render};

/// The per-clip numbers are what say *which* clip: in the noise-swell defect
/// the mix was unremarkable and one track was the problem.
#[test]
fn each_audible_clip_is_reported_on_its_own() {
    let tools = tools();
    let dir = fixture_dir("loudness-per-clip");
    let loud = tone_asset(&tools, &dir, "loud", 220, 2.0, 0.5, "");
    let quiet = tone_asset(&tools, &dir, "quiet", 880, 2.0, 0.05, "");
    let project = project(
        vec![picture(&tools, &dir, 2), loud, quiet],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track("a1", vec![clip("m-loud", "loud", 0, 60)]),
            audio_track("a2", vec![clip("m-quiet", "quiet", 0, 60)]),
        ],
    );

    let (_, report) = render(&tools, &project, &dir);
    let levels = report.levels.expect("reported");
    let of = |id: &str| {
        levels
            .clips
            .iter()
            .find(|(clip, _)| clip == id)
            .unwrap_or_else(|| panic!("`{id}` should have a level, got {:?}", levels.clips))
            .1
            .mean_dbfs
            .expect("audible")
    };

    assert!(
        of("m-loud") - of("m-quiet") > 15.0,
        "the quiet clip must read far under the loud one: \
         loud {:.1}, quiet {:.1}",
        of("m-loud"),
        of("m-quiet")
    );
    // And the mix is dominated by the loud one, which is exactly why the mix
    // number alone would not have found the quiet one.
    let mix = levels.mix.whole.loudness.mean_dbfs.expect("audible");
    assert!(
        (mix - of("m-loud")).abs() < 3.0,
        "mix {mix:.1} should sit near the loud clip at {:.1}",
        of("m-loud")
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A project with no audio at all has nothing to measure, which is not the same
/// as measuring silence.
#[test]
fn a_silent_project_reports_no_level_rather_than_a_silent_one() {
    let tools = tools();
    let dir = fixture_dir("loudness-none");
    let project = project(
        vec![picture(&tools, &dir, 2)],
        vec![video_track("v1", vec![clip("c1", "picture", 0, 60)])],
    );

    let (out, report) = render(&tools, &project, &dir);
    assert!(report.levels.is_none(), "no audio stream, nothing to say");
    assert!(soundtrack(&tools, &out).is_empty() || report.seconds_of_audio.is_none());
    std::fs::remove_dir_all(&dir).ok();
}
