//! `duck_music`: its defaults are core's, and each knob reaches the dip.

use super::fixture::project;
use crate::{call, said};
use scorsese_core::Project;
use serde_json::{Value, json};

/// Calls `duck_music` on a fresh fixture with `extra` arguments, and hands
/// back the reply and the music clip's ducked points afterwards.
fn duck(label: &str, extra: Value) -> (String, bool, Vec<(u64, f64)>) {
    let dir = project(label);
    let mut arguments = json!({ "project": dir, "music": "music" });
    for (key, value) in extra.as_object().expect("an object") {
        arguments[key] = value.clone();
    }
    let (text, failed) = said(&call("duck_music", arguments));
    let project = Project::load(&dir).expect("reload");
    let points = project.tracks[1].clips[0]
        .keyframes
        .iter()
        .filter(|track| track.is_generated_by("duck"))
        .flat_map(|track| track.keyframes.iter().map(|k| (k.t.get(), k.value)))
        .collect();
    std::fs::remove_dir_all(dir).ok();
    (text, failed, points)
}

#[test]
fn ducking_writes_keyframes_and_says_how_many() {
    let dir = project("duck");
    let (text, failed) = said(&call(
        "duck_music",
        json!({ "project": dir, "music": "music" }),
    ));
    assert!(!failed, "{text}");
    assert!(text.contains("1 clip(s) ducked"), "got {text}");
    let on_disk = std::fs::read_to_string(dir.join("project.json")).expect("read back");
    assert!(on_disk.contains(r#""by": "duck""#), "got {on_disk}");
    std::fs::remove_dir_all(dir).ok();
}

/// The defaults live in `scorsese-core` (#1071): `duck_music` with no knobs
/// writes exactly what core's own default dip plans on the same project.
#[test]
fn ducking_with_no_knobs_is_cores_default_dip() {
    use scorsese_core::{Dip, Project, PropertyPath, TrackId, Under, duck_track};
    let dir = project("duck-defaults");
    let mut expected = Project::load(&dir).expect("load");
    let under = Under::new(TrackId::new("music"));
    let dip = Dip::default_for(expected.timeline_fps.as_f64());
    let volume = PropertyPath::new(scorsese_render::audio::path::VOLUME);
    duck_track(&mut expected, &under, &volume, dip).expect("the music track");

    let (text, failed) = said(&call(
        "duck_music",
        json!({ "project": dir, "music": "music" }),
    ));
    assert!(!failed, "{text}");
    let written = Project::load(&dir).expect("reload");
    assert_eq!(written.tracks, expected.tracks);
    std::fs::remove_dir_all(dir).ok();
}

/// `between` sets the resting level, and a range ducks where no clip is —
/// in place of the narration track, since naming anything replaces the
/// default of every other audio track.
#[test]
fn between_and_a_range_shape_the_dip() {
    let (text, failed, points) = duck(
        "duck-range",
        json!({ "between": 0.6, "ranges": [[10.0, 12.0]] }),
    );
    assert!(!failed, "{text}");
    assert_eq!(
        points,
        vec![(291, 0.6), (300, 0.25), (360, 0.25), (378, 0.6)]
    );
}

/// A named clip covers wherever it sits; `min_gap_seconds` only matters
/// between two lines, so one line dips exactly as the default does.
#[test]
fn a_named_clip_and_a_min_gap_are_accepted() {
    let (text, failed, points) = duck(
        "duck-clip",
        json!({ "clips": ["v1c"], "min_gap_seconds": 0.1 }),
    );
    assert!(!failed, "{text}");
    assert_eq!(points, vec![(51, 1.0), (60, 0.25), (150, 0.25), (168, 1.0)]);
}

/// A typo'd clip and a backwards range are refused, and nothing is written.
#[test]
fn what_cannot_be_ducked_under_is_refused() {
    let (text, failed, points) = duck("duck-typo", json!({ "clips": ["v9"] }));
    assert!(failed && text.contains("`v9`"), "got {text}");
    assert!(points.is_empty());

    let (text, failed, _) = duck("duck-backwards", json!({ "ranges": [[5.0, 2.0]] }));
    assert!(failed && text.contains("forward"), "got {text}");
}
