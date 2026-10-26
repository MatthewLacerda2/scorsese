//! A patch file over the protocol (#1119): the instrument several songs share
//! by naming its path, written and checked without a file tool.

use super::fixture::project;
use crate::protocol::{call, said};
use serde_json::{Value, json};

/// Runs a tool and fails the test with what it said if it refused.
fn ok(name: &str, arguments: Value) -> String {
    let (text, failed) = said(&call(name, arguments));
    assert!(!failed, "{name} refused: {text}");
    text
}

/// A bass patch, bare: no `recipe` tag, because a song names it by path.
fn bass(wave: &str) -> String {
    json!({
        "source": { "kind": "osc_stack", "oscs": [{ "wave": wave }] },
        "amp": { "a": 0.01, "d": 0.1, "s": 0.7, "r": 0.2 }
    })
    .to_string()
}

/// The whole of what the issue asked for: a client with no file tools writes
/// the shared patch, a song names it, and the bake plays it — and rewriting
/// the patch alone is enough for the next bake to redo the song.
#[test]
fn a_patch_file_written_over_the_protocol_is_played_by_the_song_that_names_it() {
    let dir = project("patch-file");
    ok(
        "synth_new",
        json!({ "project": dir, "name": "theme", "kind": "song" }),
    );

    let written = ok(
        "synth_write",
        json!({ "project": dir, "recipe": "recipes/bass.json", "document": bass("saw") }),
    );
    assert!(written.contains("a patch file"), "got {written}");
    let checked = ok(
        "synth_check",
        json!({ "project": dir, "recipe": "recipes/bass.json" }),
    );
    assert!(
        checked.contains("a patch file, and it parses"),
        "got {checked}"
    );
    let read = ok(
        "synth_read",
        json!({ "project": dir, "recipe": "recipes/bass.json" }),
    );
    assert_eq!(read, bass("saw"), "written as given");

    let song = json!({
        "recipe": "song", "bpm": 120,
        "tracks": [{ "name": "bass", "patch": "recipes/bass.json" }],
        "patterns": { "a": { "beats": 2, "notes": [
            { "track": "bass", "note": "E1", "start": 0, "dur": 2 }] } },
        "arrangement": ["a"]
    });
    ok(
        "synth_write",
        json!({ "project": dir, "recipe": "recipes/theme.json", "document": song.to_string() }),
    );
    let baked = ok("synth_bake", json!({ "project": dir }));
    assert!(baked.contains("theme — baked"), "got {baked}");

    ok(
        "synth_write",
        json!({ "project": dir, "recipe": "recipes/bass.json", "document": bass("square") }),
    );
    let again = ok("synth_bake", json!({ "project": dir }));
    assert!(
        again.contains("theme — baked"),
        "the song must be redone: {again}"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A document with no tag is held to a patch's rules, and a refusal says why
/// it was read that way — the likeliest cause is a recipe that lost its tag.
#[test]
fn an_untagged_document_that_is_not_a_patch_is_refused_and_nothing_is_written() {
    let dir = project("patch-refuse");
    let silent = json!({
        "source": { "kind": "osc_stack", "oscs": [] },
        "amp": { "a": 0.01, "d": 0.1, "s": 0.7, "r": 0.2 }
    });
    let tagless_song = json!({ "bpm": 120, "tracks": [], "patterns": {}, "arrangement": [] });
    for document in [silent, tagless_song] {
        let (text, failed) = said(&call(
            "synth_write",
            json!({ "project": dir, "recipe": "recipes/bass.json",
                    "document": document.to_string() }),
        ));
        assert!(failed && text.contains("nothing written"), "got {text}");
        assert!(
            text.contains("`recipe` tag"),
            "it must say how it was read: {text}"
        );
    }
    assert!(
        !dir.join("recipes/bass.json").exists(),
        "nothing reached disk"
    );
    std::fs::remove_dir_all(dir).ok();
}
