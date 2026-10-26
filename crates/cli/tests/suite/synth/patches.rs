//! A song that names its patch by path: the patch file is part of what the
//! bake is addressed by, so editing it rebakes the song (#672).

use std::path::Path;

use crate::common::{new_project, run_in};
use serde_json::Value;

/// Starts a project whose song plays its first track through
/// `recipes/bass.json` instead of carrying that patch inline.
fn naming_a_patch(label: &str) -> std::path::PathBuf {
    let dir = new_project(label);
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();

    let song = dir.join("recipes/theme.json");
    let mut document: Value = read(&song);
    let track = &mut document["tracks"][0]["patch"];
    let patch = std::mem::replace(track, Value::from("recipes/bass.json"));
    assert!(patch.is_object(), "the starter's patch is inline: {patch}");
    write(&dir.join("recipes/bass.json"), &patch);
    write(&song, &document);
    dir
}

fn read(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).expect("a recipe");
    serde_json::from_str(&text).expect("recipe JSON")
}

fn write(path: &Path, value: &Value) {
    let text = serde_json::to_string_pretty(value).expect("JSON");
    std::fs::write(path, text).expect("write a recipe");
}

#[test]
fn editing_a_named_patch_rebakes_the_song() {
    let dir = naming_a_patch("synth-named-patch");
    run_in(&dir, &["synth", "bake"]).ok().says("1 rendered");
    run_in(&dir, &["synth", "bake"])
        .ok()
        .says("0 rendered, 1 cached");

    let bass = dir.join("recipes/bass.json");
    let mut patch = read(&bass);
    patch["amp"]["r"] = Value::from(0.777);
    write(&bass, &patch);

    let again = run_in(&dir, &["synth", "bake"]).ok();
    again.says("1 rendered");
    again.silent_about("already baked");
}
