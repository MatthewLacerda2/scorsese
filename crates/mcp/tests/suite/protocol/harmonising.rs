//! The harmony report over the protocol: `synth_check` prints it whole, and
//! `synth_bake` adds one line only when there is something to say.

use super::fixture::project;
use crate::protocol::{call, said};
use serde_json::json;

/// A bass on F2 under a pad's E-major chord, for the second half of the bar.
const RUB: &str = r#"{
  "recipe": "song", "bpm": 120, "key": "E major",
  "tracks": [
    { "name": "bass", "patch": { "source": { "kind": "osc_stack", "oscs": [{ "wave": "saw" }] },
        "amp": { "a": 0.01, "d": 0.1, "s": 0.8, "r": 0.1 } } },
    { "name": "pad", "patch": { "source": { "kind": "osc_stack", "oscs": [{ "wave": "sine" }] },
        "amp": { "a": 0.2, "d": 0.1, "s": 0.8, "r": 0.5 } } }
  ],
  "patterns": { "chorus": { "beats": 4, "notes": [
    { "track": "pad", "chord": "E", "oct": 3, "start": 0, "dur": 4 },
    { "track": "bass", "note": "E2", "start": 0, "dur": 2 },
    { "track": "bass", "note": "F2", "start": 2, "dur": 2 } ] } },
  "arrangement": ["chorus"]
}"#;

fn ok(name: &str, arguments: serde_json::Value) -> String {
    let (text, failed) = said(&call(name, arguments));
    assert!(!failed, "{name} refused: {text}");
    text
}

#[test]
fn check_and_bake_both_name_a_rub_and_a_clean_song_adds_nothing_to_a_bake() {
    let dir = project("harmony");
    ok(
        "synth_new",
        json!({ "project": dir, "name": "theme", "kind": "song" }),
    );
    let clean = ok("synth_bake", json!({ "project": dir }));
    assert!(
        !clean.contains("harmony"),
        "a clean song's bake grows: {clean}"
    );

    let recipe = "recipes/theme.json";
    ok(
        "synth_write",
        json!({ "project": dir, "recipe": recipe, "document": RUB }),
    );
    let checked = ok("synth_check", json!({ "project": dir, "recipe": recipe }));
    assert!(
        checked.contains("2 beats: bass F2 against pad E3, chorus beat 2"),
        "got {checked}"
    );
    assert!(
        checked.contains("bass 50% of its held notes"),
        "got {checked}"
    );

    let baked = ok("synth_bake", json!({ "project": dir }));
    assert!(
        baked.contains(
            "harmony: 1 clash, longest 2 beats: bass F2 against pad E3, chorus beat 2; \
             out of E major: bass 50%"
        ),
        "got {baked}"
    );
    std::fs::remove_dir_all(dir).ok();
}
