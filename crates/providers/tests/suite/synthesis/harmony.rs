//! The harmony report read from a project: instruments resolved where the
//! project keeps them, so a hat named by file is known to be noise.

use crate::common::{project, synth_asset, write};
use scorsese_providers::synth::{asset_harmony, check, harmony};

const HAT: &str = r#"{
  "source": { "kind": "noise" },
  "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.05 }
}"#;

/// A pad on E3 with `other` holding F3 over it for the whole bar.
fn rubbing(other: &str) -> String {
    format!(
        r#"{{
  "recipe": "song", "bpm": 120,
  "tracks": [
    {{ "name": "pad", "patch": {{ "source": {{ "kind": "osc_stack", "oscs": [{{ "wave": "sine" }}] }},
        "amp": {{ "a": 0.2, "d": 0.1, "s": 0.8, "r": 0.5 }} }} }},
    {{ "name": "other", "patch": {other} }}
  ],
  "patterns": {{ "a": {{ "beats": 4, "notes": [
      {{ "track": "pad", "note": "E3", "start": 0, "dur": 4 }},
      {{ "track": "other", "note": "F3", "start": 0, "dur": 4 }} ] }} }},
  "arrangement": ["a"]
}}"#
    )
}

const LEAD: &str = r#"{ "source": { "kind": "osc_stack", "oscs": [{ "wave": "saw" }] },
  "amp": { "a": 0.01, "d": 0.1, "s": 0.8, "r": 0.1 } }"#;

#[test]
fn a_bake_line_names_the_rub_in_a_project_song() {
    let (dir, mut project) = project("harmony-rub");
    write(&dir, "recipes/tune.json", &rubbing(LEAD));
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    let found = asset_harmony(&project, &dir, &id).expect("a song");
    assert_eq!(
        found.headline().as_deref(),
        Some("harmony: 1 clash, longest 4 beats: pad E3 against other F3, a beat 0")
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_noise_instrument_named_by_file_is_resolved_and_skipped() {
    let (dir, mut project) = project("harmony-named-noise");
    write(&dir, "recipes/hat.json", HAT);
    write(&dir, "recipes/tune.json", &rubbing(r#""recipes/hat.json""#));
    let id = synth_asset(&mut project, "tune", "recipes/tune.json");
    let found = asset_harmony(&project, &dir, &id).expect("a song");
    assert!(found.is_clean(), "got {found}");
    // Without the project to resolve it in, the hat is read as pitched.
    let recipe = check(&rubbing(r#""recipes/hat.json""#)).expect("parses");
    assert_eq!(harmony(&recipe, None).expect("a song").clashes, 1);
    assert!(harmony(&recipe, Some(&dir)).expect("a song").is_clean());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_one_shot_has_no_harmony() {
    let recipe = check(
        r#"{ "recipe": "patch", "note": "C4", "duration": 0.4, "patch": {
      "source": { "kind": "noise" }, "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.05 } } }"#,
    )
    .expect("parses");
    assert!(harmony(&recipe, None).is_none());
}
