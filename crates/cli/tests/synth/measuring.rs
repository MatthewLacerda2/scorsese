//! `scorsese level`, over the bakes `scorsese synth` just made.
//!
//! The two commands are tested together because that is the loop they are for:
//! bake a recipe, measure what came out, change the recipe, measure again
//! against the last one. A person can listen instead; an unattended agent
//! cannot, and these are what it has.

use std::path::Path;

use crate::common::{new_project, reload, run_in};

/// Where the project's one bake landed.
fn bake_of(dir: &Path) -> String {
    let project = reload(dir);
    let asset = project.assets.first().expect("one asset");
    asset
        .path
        .as_ref()
        .expect("baked media")
        .resolve(dir)
        .display()
        .to_string()
}

/// A project with one baked song, and the file it produced.
fn baked(label: &str) -> (std::path::PathBuf, String) {
    let dir = new_project(label);
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    run_in(&dir, &["synth", "bake"]).ok();
    let file = bake_of(&dir);
    (dir, file)
}

/// A bake says how it came out at the moment it is made, so the level is in
/// front of whoever asked for it without a second command.
#[test]
fn a_bake_reports_its_own_level_and_where_its_energy_sits() {
    let dir = new_project("level-bake");
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    let run = run_in(&dir, &["synth", "bake"]).ok();
    run.says("dBFS");
    run.says("low");
    run.says("high");
}

/// A song of two instruments, one deep and one bright, written over the
/// starter recipe.
fn duet(dir: &Path) {
    let voice = |name: &str, wave: &str| {
        format!(
            r#"{{ "name": "{name}", "gain": 1.0, "patch": {{
                 "source": {{ "kind": "osc_stack", "oscs": [{{ "wave": "{wave}" }}] }},
                 "amp": {{ "a": 0.01, "d": 0.1, "s": 0.7, "r": 0.2 }} }} }}"#
        )
    };
    let recipe = format!(
        r#"{{ "recipe": "song", "bpm": 120, "tracks": [{}, {}],
              "patterns": {{ "a": {{ "beats": 2, "notes": [
                {{ "track": "sub", "note": "E1", "start": 0, "dur": 2 }},
                {{ "track": "bell", "note": "E5", "start": 0, "dur": 2 }}] }} }},
              "arrangement": ["a"] }}"#,
        voice("sub", "sine"),
        voice("bell", "triangle")
    );
    std::fs::write(dir.join("recipes/theme.json"), recipe).expect("write the recipe");
}

/// The half a summary cannot answer: the mix is bottom-heavy, and *this* is
/// the instrument that made it so. Printed every time, because a report that
/// has to be asked for is one an unattended agent never sees.
#[test]
fn a_bake_says_which_track_is_taking_up_the_room() {
    let dir = new_project("level-tracks");
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    duet(&dir);
    let run = run_in(&dir, &["synth", "bake"]).ok();
    run.says("sub ");
    run.says("bell");
    // The share of each track's energy below 250 Hz, off its own row.
    let low = |row: &str| -> u32 {
        run.output
            .lines()
            .find(|line| line.trim_start().starts_with(row))
            .and_then(|line| line.split("low").nth(1))
            .and_then(|rest| rest.split('%').next())
            .and_then(|share| share.trim().parse().ok())
            .unwrap_or_else(|| panic!("no low share on the {row} row:\n{}", run.output))
    };
    assert!(
        low("sub") > 90,
        "a 41 Hz sine is the bass, not {}",
        low("sub")
    );
    assert!(low("bell") < 10, "and the bell is not, at {}", low("bell"));
}

/// And any finished file can be asked afterwards, which is what makes this
/// work on a delivered render as readily as on a bake.
#[test]
fn a_finished_file_can_be_measured_on_its_own() {
    let (dir, file) = baked("level-file");
    let run = run_in(&dir, &["level", &file]).ok();
    run.says("dBFS");
    run.says("mid");
}

/// The comparison, and the case that has to come out clean: a file against
/// itself moved in no field at all.
#[test]
fn a_file_compared_with_itself_is_reported_as_unchanged() {
    let (dir, file) = baked("level-same");
    let run = run_in(&dir, &["level", &file, "--against", &file]).ok();
    run.says("vs");
    run.says("nothing measurably changed");
}

/// A file that is not media is refused with something to act on, rather than
/// reported as a silence — "there is no sound here" and "this is not a sound
/// file" are different findings.
#[test]
fn a_file_that_is_not_media_is_refused_rather_than_measured() {
    let dir = new_project("level-refused");
    let document = dir.join("project.json").display().to_string();
    let run = run_in(&dir, &["level", &document]);
    assert!(run.failed, "not media, so not measurable:\n{}", run.output);
    run.says("measuring");
}
