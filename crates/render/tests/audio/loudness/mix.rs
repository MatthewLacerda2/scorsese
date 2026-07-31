//! The delivered soundtrack's own level.

use crate::common::audio::tone_asset;
use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{audio_track, clip, project, video_track};
use crate::{picture, render};

use super::{SLACK, sine_mean_dbfs};

#[test]
fn a_tone_of_known_amplitude_is_reported_where_it_should_be() {
    let tools = tools();
    let dir = fixture_dir("loudness-known");
    // Half scale: far enough from both the rail and the noise floor that
    // neither could account for the answer.
    let beep = tone_asset(&tools, &dir, "beep", 440, 2.0, 0.5, "");
    let project = project(
        vec![picture(&tools, &dir, 2), beep],
        vec![
            video_track("v1", vec![clip("c1", "picture", 0, 60)]),
            audio_track("a1", vec![clip("m1", "beep", 0, 60)]),
        ],
    );

    let (_, report) = render(&tools, &project, &dir);
    let levels = report
        .levels
        .expect("a render with sound reports its level");

    let mean = levels.mix.mean_dbfs.expect("the mix is audible");
    let expected = sine_mean_dbfs(0.5);
    assert!(
        (mean - expected).abs() < SLACK,
        "a half-scale sine should read near {expected:.1} dBFS, and read {mean:.1}"
    );
    assert!(
        !levels.mix.is_clipping(),
        "half scale is nowhere near the rail"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Twice the amplitude is 6 dB louder, and the report has to say so — that
/// difference is the whole of how the two quiet scores were diagnosed.
#[test]
fn a_quieter_mix_reads_quieter_by_the_amount_it_is_quieter() {
    let tools = tools();
    let mut heard = Vec::new();
    for (name, amplitude) in [("loud", 0.5), ("quiet", 0.25)] {
        let dir = fixture_dir(&format!("loudness-{name}"));
        let beep = tone_asset(&tools, &dir, "beep", 440, 2.0, amplitude, "");
        let project = project(
            vec![picture(&tools, &dir, 2), beep],
            vec![
                video_track("v1", vec![clip("c1", "picture", 0, 60)]),
                audio_track("a1", vec![clip("m1", "beep", 0, 60)]),
            ],
        );
        let (_, report) = render(&tools, &project, &dir);
        heard.push(
            report
                .levels
                .expect("reported")
                .mix
                .mean_dbfs
                .expect("audible"),
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    let difference = heard[0] - heard[1];
    assert!(
        (difference - 6.02).abs() < SLACK,
        "halving the amplitude should read about 6 dB down, and read {difference:.1}"
    );
}
