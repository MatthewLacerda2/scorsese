//! Making sound over the protocol: the write-bake-listen-adjust loop.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// Runs a tool and fails the test with what it said if it refused.
fn ok(name: &str, arguments: serde_json::Value) -> String {
    let (text, failed) = said(&call(name, arguments));
    assert!(!failed, "{name} refused: {text}");
    text
}

/// The whole loop, end to end and costing nothing: start a sound, read it,
/// change it, bake it, and find the file on disk.
#[test]
fn a_sound_can_be_started_edited_and_baked_without_leaving_the_protocol() {
    let dir = project("compose");

    let started = ok(
        "synth_new",
        json!({ "project": dir, "name": "theme", "kind": "song" }),
    );
    assert!(started.contains("recipes/theme.json"), "got {started}");

    let recipe = ok(
        "synth_read",
        json!({ "project": dir, "recipe": "recipes/theme.json" }),
    );
    assert!(recipe.contains("\"recipe\": \"song\""), "got {recipe}");

    let slower = recipe.replace("\"bpm\": 96.0", "\"bpm\": 72.0");
    ok(
        "synth_write",
        json!({ "project": dir, "recipe": "recipes/theme.json", "document": slower }),
    );

    let baked = ok("synth_bake", json!({ "project": dir }));
    assert!(baked.contains("theme — baked"), "got {baked}");
    assert!(baked.contains("$0.00"), "it must say what it cost: {baked}");

    let generated = std::fs::read_dir(dir.join("generated")).expect("read generated/");
    assert!(generated.count() > 0, "something reached generated/");
    std::fs::remove_dir_all(dir).ok();
}

/// The property the content-addressed cache buys: re-baking an unchanged
/// recipe does no work, and an assistant calling it twice costs nothing.
#[test]
fn baking_twice_renders_nothing_the_second_time() {
    let dir = project("compose-cache");
    ok("synth_new", json!({ "project": dir, "name": "zap" }));
    ok("synth_bake", json!({ "project": dir }));

    let again = ok("synth_bake", json!({ "project": dir }));
    assert!(
        again.contains("0 rendered"),
        "a second bake of unchanged recipes must render nothing: {again}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// Nothing marks the asset stale — the bake is named for the recipe's hash, so
/// changing the recipe changes which file it wants and the next bake redoes it.
#[test]
fn editing_a_recipe_is_enough_to_make_the_next_bake_redo_it() {
    let dir = project("compose-stale");
    ok("synth_new", json!({ "project": dir, "name": "zap" }));
    ok("synth_bake", json!({ "project": dir }));

    let recipe = ok(
        "synth_read",
        json!({ "project": dir, "recipe": "recipes/zap.json" }),
    );
    let edited = recipe.replace("\"note\": \"C4\"", "\"note\": \"G2\"");
    assert_ne!(edited, recipe, "the fixture's note should be C4");
    ok(
        "synth_write",
        json!({ "project": dir, "recipe": "recipes/zap.json", "document": edited }),
    );

    let again = ok("synth_bake", json!({ "project": dir }));
    assert!(
        again.contains("zap — baked"),
        "the edited recipe must be redone: {again}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The reply carries the mix broken up by instrument, not only by time. A
/// client that cannot hear can be told a mix is bottom-heavy and can do
/// nothing about it until it is told which layer is the bottom.
#[test]
fn a_bake_reply_says_which_track_owns_the_energy() {
    let dir = project("compose-tracks");
    ok(
        "synth_new",
        json!({ "project": dir, "name": "theme", "kind": "song" }),
    );
    let voice = |name: &str, wave: &str| {
        json!({ "name": name, "gain": 1.0, "patch": {
            "source": { "kind": "osc_stack", "oscs": [{ "wave": wave }] },
            "amp": { "a": 0.01, "d": 0.1, "s": 0.7, "r": 0.2 } } })
    };
    let duet = json!({
        "recipe": "song", "bpm": 120,
        "tracks": [voice("sub", "sine"), voice("bell", "triangle")],
        "patterns": { "a": { "beats": 2, "notes": [
            { "track": "sub", "note": "E1", "start": 0, "dur": 2 },
            { "track": "bell", "note": "E5", "start": 0, "dur": 2 }] } },
        "arrangement": ["a"]
    });
    ok(
        "synth_write",
        json!({ "project": dir, "recipe": "recipes/theme.json",
                "document": duet.to_string() }),
    );

    let baked = ok("synth_bake", json!({ "project": dir }));
    for row in ["\n  sub ", "\n  bell "] {
        assert!(baked.contains(row), "no `{row}` row in: {baked}");
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn checking_a_recipe_says_what_it_is_without_rendering_it() {
    let dir = project("compose-check");
    ok("synth_new", json!({ "project": dir, "name": "zap" }));

    let checked = ok(
        "synth_check",
        json!({ "project": dir, "recipe": "recipes/zap.json" }),
    );
    assert!(checked.contains("patch"), "got {checked}");
    let generated = std::fs::read_dir(dir.join("generated")).expect("read generated/");
    assert_eq!(generated.count(), 0, "check must render nothing");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_document_that_is_not_a_recipe_is_refused_and_nothing_is_written() {
    let dir = project("compose-refuse");
    ok("synth_new", json!({ "project": dir, "name": "zap" }));
    let before = std::fs::read_to_string(dir.join("recipes/zap.json")).expect("read");

    let (text, failed) = said(&call(
        "synth_write",
        json!({ "project": dir, "recipe": "recipes/zap.json", "document": "{ nonsense" }),
    ));
    assert!(failed && text.contains("nothing written"), "got {text}");
    assert_eq!(
        std::fs::read_to_string(dir.join("recipes/zap.json")).expect("read"),
        before,
        "the recipe on disk must be untouched"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A recipe path is a path, and a path in this project stays inside it. The
/// argument must not be a second, looser way into the filesystem.
#[test]
fn a_recipe_path_may_not_leave_the_project() {
    let dir = project("compose-escape");
    for escape in ["../outside.json", "/etc/hostname"] {
        let (text, failed) = said(&call(
            "synth_read",
            json!({ "project": dir, "recipe": escape }),
        ));
        assert!(failed, "`{escape}` must be refused");
        assert!(text.contains("recipe path"), "got {text}");
    }
    std::fs::remove_dir_all(dir).ok();
}
