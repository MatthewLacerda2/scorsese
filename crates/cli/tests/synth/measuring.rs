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
