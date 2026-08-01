//! `scorsese synth survey`: the one report that looks at a set rather than at
//! one bake.
//!
//! The finding it exists for is the one the per-bake numbers cannot reach — six
//! cues that each measure perfectly can still be one instrument played six
//! times — so what is asserted here is that the count arrives, and that it
//! costs no bake to get it.

use std::path::Path;

use crate::common::{new_project, run_in};

/// A song whose loudest track is a plucked string, over the starter recipe.
///
/// Written by hand rather than tuned through `synth set`, because the whole
/// point of the report is that it reads what is on the page — a fixture built
/// out of the thing under test would prove less.
fn cue(dir: &Path, name: &str, bpm: u32, cutoff: u32, note: &str) {
    let recipe = format!(
        r#"{{ "recipe": "song", "bpm": {bpm},
              "tracks": [
                {{ "name": "arp", "gain": 0.85, "patch": {{
                     "source": {{ "kind": "karplus" }},
                     "amp": {{ "a": 0.01, "d": 0.2, "s": 0.3, "r": 0.2 }},
                     "filter": {{ "kind": "lowpass", "cutoff": {cutoff} }} }} }},
                {{ "name": "pad", "gain": 0.4, "patch": {{
                     "source": {{ "kind": "osc_stack", "oscs": [{{ "wave": "saw" }}] }},
                     "amp": {{ "a": 0.2, "d": 0.1, "s": 0.8, "r": 0.4 }} }} }}
              ],
              "patterns": {{ "a": {{ "beats": 2, "notes": [
                {{ "track": "arp", "note": "{note}", "start": 0, "dur": 1 }},
                {{ "track": "pad", "note": "C2", "start": 0, "dur": 2 }}] }} }},
              "arrangement": ["a"] }}"#
    );
    std::fs::write(dir.join(format!("recipes/{name}.json")), recipe).expect("write the recipe");
}

/// A project of two cues that are the same instrument twice.
fn two_cues(label: &str) -> std::path::PathBuf {
    let dir = new_project(label);
    for (name, bpm, cutoff, note) in [("km", 96, 4600, "E4"), ("calado", 132, 4400, "G4")] {
        run_in(&dir, &["synth", "new", name, "--kind", "song"]).ok();
        cue(&dir, name, bpm, cutoff, note);
    }
    dir
}

/// Every column of the report, and the count under it that no single bake
/// could have produced.
#[test]
fn a_survey_says_what_each_song_is_made_of_and_counts_it_across_them() {
    let dir = two_cues("synth-survey");
    let run = run_in(&dir, &["synth", "survey"]).ok();

    run.says("km");
    run.says("96 bpm");
    run.says("cutoff 4600 Hz");
    run.says("gain 0.85");
    run.says("no filter");
    run.says("C2-E4");
    // The rollup: one source kind is the loudest thing in both pieces.
    run.says("2 songs");
    run.says("in 2, loudest in 2");
    run.says("96-132 bpm");
    std::fs::remove_dir_all(dir).ok();
}

/// It reads documents and nothing else — no bake happens, and none is needed
/// for the report to be complete.
#[test]
fn a_survey_costs_no_bake() {
    let dir = two_cues("synth-survey-free");
    run_in(&dir, &["synth", "survey"]).ok();
    let generated = std::fs::read_dir(dir.join("generated"))
        .expect("the generated directory")
        .count();
    assert_eq!(generated, 0, "a survey rendered something");
    std::fs::remove_dir_all(dir).ok();
}

/// One song is not a set: the command succeeds and prints nothing at all,
/// because a single row under a summary of it is the same sentence twice.
#[test]
fn a_project_of_one_song_is_surveyed_in_silence() {
    let dir = new_project("synth-survey-one");
    run_in(&dir, &["synth", "new", "solo", "--kind", "song"]).ok();
    let run = run_in(&dir, &["synth", "survey"]).ok();
    assert!(run.output.trim().is_empty(), "said: {}", run.output);
    std::fs::remove_dir_all(dir).ok();
}
