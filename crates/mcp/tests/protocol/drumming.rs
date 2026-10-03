//! The instrument library over the protocol: find one, use it, own the copy.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

fn ok(name: &str, arguments: serde_json::Value) -> String {
    let (text, failed) = said(&call(name, arguments));
    assert!(!failed, "{name} refused: {text}");
    text
}

#[test]
fn the_kit_lists_itself_and_shows_one_patch() {
    let dir = project("kit-list");
    let listed = ok("synth_kit", json!({ "project": dir }));
    for name in [
        "kit:kick",
        "kit:snare",
        "kit:hat",
        "kit:crash",
        "kit:epiano",
    ] {
        assert!(listed.contains(name), "{name} missing from {listed}");
    }
    let kick = ok("synth_kit", json!({ "project": dir, "instrument": "kick" }));
    assert!(
        kick.contains("pitch_env"),
        "a kick falls onto its pitch: {kick}"
    );
    let (text, failed) = said(&call(
        "synth_kit",
        json!({ "project": dir, "instrument": "cowbell" }),
    ));
    assert!(failed && text.contains("synth_kit"), "got {text}");
    std::fs::remove_dir_all(dir).ok();
}

/// The decision #511 records: a song names a kit instrument, and what lands
/// on disk is the patch itself — so the recipe bakes, and nothing in the
/// project depends on the library afterwards.
#[test]
fn writing_a_kit_name_copies_the_instrument_into_the_recipe() {
    let dir = project("kit-write");
    ok(
        "synth_new",
        json!({ "project": dir, "name": "beat", "kind": "song" }),
    );
    let song = r#"{ "recipe": "song", "bpm": 120,
      "tracks": [{ "name": "kick", "patch": "kit:kick" }, { "name": "hat", "patch": "kit:hat" }],
      "patterns": { "a": { "beats": 4, "notes": [
        { "track": "kick", "steps": "x---x---", "div": 0.5 },
        { "track": "hat", "steps": "--x---x-", "div": 0.5 } ] } },
      "arrangement": ["a"] }"#;
    let wrote = ok(
        "synth_write",
        json!({ "project": dir, "recipe": "recipes/beat.json", "document": song }),
    );
    assert!(
        wrote.contains("kick, hat"),
        "it says what it copied: {wrote}"
    );

    let on_disk = std::fs::read_to_string(dir.join("recipes/beat.json")).expect("written");
    assert!(!on_disk.contains("kit:"), "the name is gone: {on_disk}");
    assert!(
        on_disk.contains("pitch_env"),
        "the kick is inline: {on_disk}"
    );

    let baked = ok("synth_bake", json!({ "project": dir }));
    assert!(baked.contains("beat — baked"), "got {baked}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_one_shot_can_start_from_a_kit_instrument() {
    let dir = project("kit-new");
    let started = ok(
        "synth_new",
        json!({ "project": dir, "name": "hit", "instrument": "kit:snare" }),
    );
    assert!(started.contains("recipes/hit.json"), "got {started}");
    let recipe = std::fs::read_to_string(dir.join("recipes/hit.json")).expect("written");
    assert!(
        recipe.contains("\"color\": \"pink\""),
        "the snare's own patch: {recipe}"
    );
    std::fs::remove_dir_all(dir).ok();
}
