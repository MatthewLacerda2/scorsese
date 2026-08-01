//! `synth_survey`: the set of recipes, counted, over the protocol.
//!
//! A client with no ears cannot hear that six cues are one instrument six
//! times. It can be told the count, and this is the tool that tells it — so
//! what is asserted is that the count crosses the wire, not that a bake did.

use super::fixture::project;
use crate::{call, said};
use serde_json::json;

/// Runs a tool and fails the test with what it said if it refused.
fn ok(name: &str, arguments: serde_json::Value) -> String {
    let (text, failed) = said(&call(name, arguments));
    assert!(!failed, "{name} refused: {text}");
    text
}

/// A song whose loudest track is a plucked string.
fn cue(dir: &std::path::Path, name: &str, bpm: u32, cutoff: u32) {
    let recipe = format!(
        r#"{{ "recipe": "song", "bpm": {bpm},
              "tracks": [
                {{ "name": "arp", "gain": 0.85, "patch": {{
                     "source": {{ "kind": "karplus" }},
                     "amp": {{ "a": 0.01, "d": 0.2, "s": 0.3, "r": 0.2 }},
                     "filter": {{ "kind": "lowpass", "cutoff": {cutoff} }} }} }},
                {{ "name": "pad", "gain": 0.4, "patch": {{
                     "source": {{ "kind": "noise" }},
                     "amp": {{ "a": 0.2, "d": 0.1, "s": 0.8, "r": 0.4 }} }} }}
              ],
              "patterns": {{ "a": {{ "beats": 2, "notes": [
                {{ "track": "arp", "note": "E4", "start": 0, "dur": 1 }},
                {{ "track": "pad", "note": "C2", "start": 0, "dur": 2 }}] }} }},
              "arrangement": ["a"] }}"#
    );
    std::fs::write(dir.join(format!("recipes/{name}.json")), recipe).expect("write the recipe");
}

/// Two cues, and the reply that says one source kind is the loudest in both.
#[test]
fn a_survey_counts_the_sources_across_every_song_in_the_project() {
    let dir = project("survey-set");
    for (name, bpm, cutoff) in [("km", 96, 4600), ("calado", 132, 4400)] {
        ok(
            "synth_new",
            json!({ "project": dir, "name": name, "kind": "song" }),
        );
        cue(&dir, name, bpm, cutoff);
    }

    let said = ok("synth_survey", json!({ "project": dir }));
    assert!(said.contains("cutoff 4600 Hz"), "{said}");
    assert!(said.contains("gain 0.85"), "{said}");
    assert!(said.contains("2 songs"), "{said}");
    assert!(said.contains("in 2, loudest in 2"), "{said}");
    assert!(said.contains("96-132 bpm"), "{said}");
    // The fixture's own recipe is a one-shot patch, and a one-shot has no
    // tempo, no arrangement and no mix to be the loudest thing in.
    assert!(!said.contains("3 songs"), "one-shots are not songs: {said}");
    std::fs::remove_dir_all(dir).ok();
}

/// One song is not a set. The report says nothing, and the reply says only how
/// many there are — a tool call cannot come back empty.
#[test]
fn a_project_of_one_song_gets_no_rows() {
    let dir = project("survey-one");
    ok(
        "synth_new",
        json!({ "project": dir, "name": "solo", "kind": "song" }),
    );
    let said = ok("synth_survey", json!({ "project": dir }));
    assert!(said.starts_with("1 song"), "{said}");
    assert!(!said.contains("bpm"), "no rows at all: {said}");
    std::fs::remove_dir_all(dir).ok();
}
