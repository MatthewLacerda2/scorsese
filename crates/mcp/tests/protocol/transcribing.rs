//! `synth_import` and `synth_export`: a MIDI file in as a song recipe, and
//! back out, over the protocol.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// Format 0, 96 ticks a beat, C4 then E4 for a beat each — written byte by
/// byte so the fixture cannot agree with the parser about a mistake.
const TUNE: &[u8] = &[
    b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, //
    b'M', b'T', b'r', b'k', 0, 0, 0, 20, //
    0x00, 0x90, 60, 100, 0x60, 0x80, 60, 0, //
    0x00, 0x90, 64, 100, 0x60, 0x80, 64, 0, //
    0x00, 0xFF, 0x2F, 0x00,
];

/// A relative `path` is the project's, like every other path in this surface —
/// and what comes back is an ordinary recipe the rest of the loop reads.
#[test]
fn a_midi_file_beside_the_project_becomes_a_recipe_that_bakes() {
    let dir = project("transcribe");
    std::fs::write(dir.join("tune.mid"), TUNE).expect("write the fixture");

    let (text, failed) = said(&call(
        "synth_import",
        json!({ "project": dir, "path": "tune.mid", "name": "rag" }),
    ));
    assert!(!failed, "synth_import refused: {text}");
    assert!(text.contains("rag — synth_audio, sketch"), "{text}");
    assert!(text.contains("recipes/rag.json"), "{text}");
    assert!(text.contains("2 notes"), "{text}");

    let (recipe, _) = said(&call(
        "synth_read",
        json!({ "project": dir, "recipe": "recipes/rag.json" }),
    ));
    assert!(recipe.contains("\"E4\""), "{recipe}");
    let (baked, failed) = said(&call("synth_bake", json!({ "project": dir })));
    assert!(!failed && baked.contains("rag — baked"), "{baked}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_file_that_is_not_midi_is_refused_and_nothing_is_added() {
    let dir = project("transcribe-refused");
    std::fs::write(dir.join("notes.mid"), b"not midi").expect("write");
    let (text, failed) = said(&call(
        "synth_import",
        json!({ "project": dir, "path": "notes.mid" }),
    ));
    assert!(failed, "{text}");
    assert!(text.contains("not a readable MIDI file"), "{text}");
    assert!(!dir.join("recipes/notes.json").exists());
    std::fs::remove_dir_all(dir).ok();
}

/// A relative `out` is the project's too, and what lands there reads back in
/// as the notes that went out.
#[test]
fn a_song_goes_back_out_as_midi_beside_the_project() {
    let dir = project("export");
    std::fs::write(dir.join("tune.mid"), TUNE).expect("write the fixture");
    said(&call(
        "synth_import",
        json!({ "project": dir, "path": "tune.mid", "name": "rag" }),
    ));

    let (text, failed) = said(&call(
        "synth_export",
        json!({ "project": dir, "asset": "rag", "out": "out/rag.mid",
                "drums": ["track-1"] }),
    ));
    assert!(!failed, "synth_export refused: {text}");
    assert!(
        text.contains("rag — written as MIDI to out/rag.mid"),
        "{text}"
    );
    assert!(text.contains("drums on channel 10: track-1"), "{text}");
    assert!(
        dir.join("out/rag.mid").is_file(),
        "resolved against the project"
    );

    let (again, failed) = said(&call(
        "synth_import",
        json!({ "project": dir, "path": "out/rag.mid", "name": "again" }),
    ));
    assert!(!failed && again.contains("2 notes"), "{again}");

    let (text, failed) = said(&call(
        "synth_export",
        json!({ "project": dir, "asset": "rag", "drums": "track-1" }),
    ));
    assert!(failed && text.contains("list of strings"), "{text}");
    std::fs::remove_dir_all(dir).ok();
}
