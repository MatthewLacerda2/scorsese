//! `scorsese duck`, end to end.
//!
//! What it asserts is the document afterwards: ducking's whole output is
//! keyframes, so the keyframes are the thing to look at. No ffmpeg, no audio,
//! and nothing generated — a duck can be planned over narration nobody has
//! paid for, which is most of the point.

#[path = "common/mod.rs"]
mod common;

use common::{documents, reload, run_in};
use scorsese_core::Project;

/// A project with a music bed under two lines of unrealised narration.
fn scored(label: &str) -> std::path::PathBuf {
    documents::written(
        label,
        &[
            documents::assets::sketch("shot"),
            documents::assets::narration("vo"),
            documents::assets::raw("bed", "audio", "wav"),
        ],
        &[
            documents::track("music", "audio", &[documents::lasting("m1", "bed", 0, 600)]),
            documents::track(
                "vo",
                "audio",
                &[
                    documents::lasting("v1", "vo", 60, 90),
                    documents::lasting("v2", "vo", 400, 60),
                ],
            ),
        ],
    )
}

/// The points of the one track ducking wrote on the music clip.
fn dip(project: &Project) -> Vec<(u64, f64)> {
    project.tracks[0].clips[0]
        .keyframes
        .iter()
        .find(|track| track.is_generated_by("duck"))
        .map(|track| {
            track
                .keyframes
                .iter()
                .map(|k| (k.t.get(), k.value))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn ducking_writes_volume_keyframes_under_the_narration() {
    let dir = scored("duck");
    run_in(&dir, &["duck", "--music", "music"]).ok().says("m1");

    let points = dip(&reload(&dir));
    assert_eq!(points.len(), 8, "two lines, two dips: {points:?}");
    // Fully down by the frame the first line starts, at 30 fps with the
    // default 0.3 s attack.
    assert_eq!(points[0], (51, 1.0));
    assert_eq!(points[1], (60, 0.25));
}

/// The narration here is a `sketch` nobody has generated. Ducking still works,
/// which is the argument for triggering on clip extents rather than on sound.
#[test]
fn it_works_on_narration_that_does_not_exist_yet() {
    let dir = scored("duck-sketch");
    let project = reload(&dir);
    assert!(
        project.assets.iter().any(|a| a.needs_generation()),
        "the fixture's narration is unrealised, or this proves nothing"
    );
    run_in(&dir, &["duck", "--music", "music"]).ok();
    assert!(!dip(&reload(&dir)).is_empty());
}

#[test]
fn the_depth_is_choosable() {
    let dir = scored("duck-depth");
    run_in(&dir, &["duck", "--music", "music", "--depth", "0.5"]).ok();
    assert!(dip(&reload(&dir)).iter().any(|(_, value)| *value == 0.5));
}

/// Re-running is the loop this exists for, so it has to be safe.
#[test]
fn re_running_replaces_its_own_work_and_nothing_else() {
    let dir = scored("duck-rerun");
    run_in(&dir, &["duck", "--music", "music"]).ok();
    let first = dip(&reload(&dir));
    run_in(&dir, &["duck", "--music", "music"]).ok();

    let project = reload(&dir);
    assert_eq!(dip(&project), first);
    assert_eq!(
        project.tracks[0].clips[0].keyframes.len(),
        1,
        "one track, not two"
    );
}

#[test]
fn a_track_that_is_not_there_fails_and_names_it() {
    let dir = scored("duck-missing");
    let run = run_in(&dir, &["duck", "--music", "nope"]);
    assert!(run.failed);
    run.says("nope");
}
