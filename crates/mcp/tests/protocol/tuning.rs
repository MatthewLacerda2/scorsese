//! Tuning a recipe: one number, by name, and nothing else moved.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// Runs a tool and fails the test with what it said if it refused.
fn ok(name: &str, arguments: serde_json::Value) -> String {
    let (text, failed) = said(&call(name, arguments));
    assert!(!failed, "{name} refused: {text}");
    text
}

/// A song to tune, and the recipe file it left behind.
fn song(label: &str) -> std::path::PathBuf {
    let dir = project(label);
    ok(
        "synth_new",
        json!({ "project": dir, "name": "theme", "kind": "song" }),
    );
    dir
}

/// The point of the whole operation: the track's level moves and the music
/// does not, without a single note crossing the wire.
#[test]
fn a_tracks_gain_moves_and_the_score_around_it_does_not() {
    let dir = song("tune-gain");
    let before = std::fs::read_to_string(dir.join("recipes/theme.json")).expect("read");

    let said = ok(
        "synth_set",
        json!({ "project": dir, "recipe": "recipes/theme.json",
                "field": "gain", "track": "lead", "value": 0.64 }),
    );
    assert!(
        said.contains("0.9"),
        "it must say what it moved from: {said}"
    );
    assert!(said.contains("0.64"), "and to: {said}");

    let after = std::fs::read_to_string(dir.join("recipes/theme.json")).expect("read");
    assert!(after.contains("\"gain\": 0.64"), "got {after}");
    assert!(after.contains("\"bpm\": 96.0"), "the tempo is untouched");
    for note in ["C3", "E3", "G3", "A2"] {
        assert!(after.contains(note), "the notes survive: {note} is gone");
    }
    assert_ne!(after, before, "something changed");
    std::fs::remove_dir_all(dir).ok();
}

/// The song's own numbers need no track, and a patch's are the same shape.
#[test]
fn a_songs_own_number_and_a_patchs_own_number_are_set_by_name_alone() {
    let dir = song("tune-own");
    ok(
        "synth_set",
        json!({ "project": dir, "recipe": "recipes/theme.json",
                "field": "bpm", "value": 72 }),
    );
    let after = std::fs::read_to_string(dir.join("recipes/theme.json")).expect("read");
    assert!(after.contains("\"bpm\": 72.0"), "got {after}");

    ok("synth_new", json!({ "project": dir, "name": "zap" }));
    ok(
        "synth_set",
        json!({ "project": dir, "recipe": "recipes/zap.json",
                "field": "velocity", "value": 0.5 }),
    );
    let patch = std::fs::read_to_string(dir.join("recipes/zap.json")).expect("read");
    assert!(patch.contains("\"velocity\": 0.5"), "got {patch}");
    std::fs::remove_dir_all(dir).ok();
}

/// The contract `project_write` and `synth_write` already keep: a refusal
/// leaves the file exactly as it was, whatever was wrong with the request.
#[test]
fn every_refusal_leaves_the_recipe_byte_for_byte_as_it_was() {
    let dir = song("tune-refuse");
    let path = dir.join("recipes/theme.json");
    let before = std::fs::read_to_string(&path).expect("read");

    let wrong = [
        // A track the song does not have.
        json!({ "field": "gain", "track": "bass", "value": 0.5 }),
        // A field this shape does not have — `duration` is a patch's.
        json!({ "field": "duration", "value": 2.0 }),
        // A seed is a whole number, and not a negative one.
        json!({ "field": "seed", "value": -3.5 }),
        // `gain` is a track's, and no track was named.
        json!({ "field": "gain", "value": 0.5 }),
    ];
    for mut arguments in wrong {
        arguments["project"] = json!(dir);
        arguments["recipe"] = json!("recipes/theme.json");
        let (text, failed) = said(&call("synth_set", arguments.clone()));
        assert!(failed, "{arguments} must be refused, got {text}");
        assert!(text.contains("nothing written"), "got {text}");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            before,
            "{arguments} changed the file it refused"
        );
    }
    std::fs::remove_dir_all(dir).ok();
}

/// Staleness by hash survives a partial edit: nobody marks anything, the
/// recipe simply hashes to a file that is not there yet.
#[test]
fn setting_one_number_is_enough_to_make_the_next_bake_redo_it() {
    let dir = project("tune-stale");
    ok("synth_new", json!({ "project": dir, "name": "zap" }));
    ok("synth_bake", json!({ "project": dir }));

    ok(
        "synth_set",
        json!({ "project": dir, "recipe": "recipes/zap.json",
                "field": "duration", "value": 0.9 }),
    );
    let again = ok("synth_bake", json!({ "project": dir }));
    assert!(
        again.contains("zap — baked"),
        "the tuned recipe must be redone: {again}"
    );
    std::fs::remove_dir_all(dir).ok();
}
