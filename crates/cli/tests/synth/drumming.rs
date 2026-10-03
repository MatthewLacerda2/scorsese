//! The instrument library from the command line: list it, copy it in, bake.

use crate::common::{new_project, reload, run_in};

const BEAT: &str = r#"{ "recipe": "song", "bpm": 120,
  "tracks": [{ "name": "kick", "patch": "kit:kick" }, { "name": "bass", "patch": "kit:bass" }],
  "patterns": { "a": { "beats": 4, "notes": [
    { "track": "kick", "steps": "x---x---", "div": 0.5 },
    { "track": "bass", "note": "E2", "start": 0, "dur": 1 } ] } },
  "arrangement": ["a"] }"#;

#[test]
fn kit_lists_every_instrument_and_prints_one() {
    let dir = new_project("synth-kit-list");
    let listed = run_in(&dir, &["synth", "kit"]).ok();
    for name in ["kit:kick", "kit:snare", "kit:hat", "kit:crash", "kit:bass"] {
        listed.says(name);
    }
    run_in(&dir, &["synth", "kit", "brass"])
        .ok()
        .says("\"algorithm\": \"twin\"");
    run_in(&dir, &["synth", "kit", "kazoo"]).says("scorsese synth kit");
}

/// A name that was never copied in is refused at the bake, with the way to
/// copy it — never resolved against the library, which a project must not
/// depend on.
#[test]
fn a_kit_name_bakes_only_once_it_is_copied_in() {
    let dir = new_project("synth-kit-copy");
    run_in(&dir, &["synth", "new", "beat", "--kind", "song"]).ok();
    std::fs::write(dir.join("recipes/beat.json"), BEAT).expect("write the beat");

    let refused = run_in(&dir, &["synth", "bake"]);
    assert!(refused.failed, "{}", refused.output);
    refused.says("--copy-into");

    let recipe = dir.join("recipes/beat.json");
    let recipe = recipe.to_str().expect("a UTF-8 temp path");
    run_in(&dir, &["synth", "kit", "--copy-into", recipe])
        .ok()
        .says("kick, bass");
    let copied = std::fs::read_to_string(dir.join("recipes/beat.json")).expect("rewritten");
    assert!(!copied.contains("kit:"), "{copied}");

    run_in(&dir, &["synth", "bake"]).ok().says("beat — baked");
}

#[test]
fn new_can_start_from_an_instrument() {
    let dir = new_project("synth-kit-new");
    run_in(&dir, &["synth", "new", "boom", "--instrument", "kick"]).ok();
    let recipe = std::fs::read_to_string(dir.join("recipes/boom.json")).expect("written");
    assert!(
        recipe.contains("pitch_env"),
        "the kick's own patch: {recipe}"
    );
    run_in(&dir, &["synth", "bake"]).ok();
    assert!(reload(&dir).assets[0].path.is_some(), "it baked");
}
