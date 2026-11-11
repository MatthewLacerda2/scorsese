//! The harmony report at the shell: `synth check` prints it whole, and `synth
//! bake` adds one line only when there is something to say.

use crate::common::{new_project, run_in};

/// A bass on F2 under a pad's E chord, and a hat told F3 throughout: its
/// patch is noise, named by file, so `check` has to find the project to know
/// the hat has no pitch to clash with.
const RUB: &str = r#"{
  "recipe": "song", "bpm": 120,
  "tracks": [ { "name": "bass", "patch": "recipes/tone.json" },
              { "name": "pad", "patch": "recipes/tone.json" },
              { "name": "hat", "patch": "recipes/hat.json" } ],
  "patterns": { "chorus": { "beats": 4, "notes": [
    { "track": "pad", "chord": "E", "oct": 3, "start": 0, "dur": 4 },
    { "track": "bass", "note": "F2", "start": 2, "dur": 2 },
    { "track": "hat", "note": "F3", "start": 0, "dur": 4 } ] } },
  "arrangement": ["chorus"]
}"#;

const TONE: &str = r#"{ "source": { "kind": "osc_stack", "oscs": [{ "wave": "saw" }] },
  "amp": { "a": 0.01, "d": 0.1, "s": 0.8, "r": 0.1 } }"#;

const HAT: &str = r#"{ "source": { "kind": "noise" },
  "amp": { "a": 0.001, "d": 0.05, "s": 0.0, "r": 0.05 } }"#;

#[test]
fn check_and_bake_both_name_a_rub_and_a_clean_bake_adds_nothing() {
    let dir = new_project("synth-harmony");
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    run_in(&dir, &["synth", "bake"])
        .ok()
        .silent_about("harmony");

    std::fs::write(dir.join("recipes/tone.json"), TONE).expect("write the patch");
    std::fs::write(dir.join("recipes/hat.json"), HAT).expect("write the hat");
    let recipe = dir.join("recipes/theme.json");
    std::fs::write(&recipe, RUB).expect("write the song");
    let path = recipe.to_str().expect("utf-8");
    let checked = run_in(&dir, &["synth", "check", path]).ok();
    checked.says("clashes: 1, 2 beats in all");
    checked.says("2 beats: bass F2 against pad E3, chorus beat 2");
    checked.silent_about("out of");

    run_in(&dir, &["synth", "bake"])
        .ok()
        .says("harmony: 1 clash, longest 2 beats: bass F2 against pad E3, chorus beat 2");
}
