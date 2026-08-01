//! A script the project names and the disk does not have.
//!
//! A warning, never a problem, for the same reason a missing generated file is
//! one: the render is not one pixel different for it, and refusing would mean a
//! project that lost its brief could no longer be finished. Worth saying loudly
//! all the same — the script is the only part of an edit that cannot be
//! reconstructed by looking at the film.

use std::path::Path;

use crate::common::documents::assets::video;
use crate::common::documents::{clip, media_file, project};
use crate::common::run_in;

/// Splices a `script` field into a project document already on disk.
fn pointing_at(dir: &Path, script: &str) {
    let file = dir.join("project.json");
    let document = std::fs::read_to_string(&file).expect("read the document");
    let annotated = document.replace(
        r#""name": "probe","#,
        &format!(r#""name": "probe", "script": "{script}","#),
    );
    std::fs::write(&file, annotated).expect("write the document");
}

fn checked(label: &str, script: &str) -> crate::common::Run {
    let dir = project(label, &[video("boat")], &[clip("c1", "boat", 0)]);
    media_file(&dir, "boat", b"");
    pointing_at(&dir, script);
    run_in(&dir, &["check"])
}

#[test]
fn a_script_that_is_not_there_warns_but_passes() {
    let run = checked("script-gone", "script.md");
    assert!(
        !run.failed,
        "a project that lost its brief still renders:\n{}",
        run.output
    );
    run.says("warning: the project's script `script.md` is not there");
    run.says("no problems, 1 warning(s)");
}

#[test]
fn a_script_that_is_there_says_nothing_at_all() {
    let dir = project("script-present", &[video("boat")], &[clip("c1", "boat", 0)]);
    media_file(&dir, "boat", b"");
    pointing_at(&dir, "script.md");
    std::fs::write(dir.join("script.md"), "the brief").expect("write the script");

    let run = run_in(&dir, &["check"]);
    assert!(!run.failed, "{}", run.output);
    run.says("no problems, nothing to warn about");
}

/// A badly-shaped path is validation's to refuse, and it should be said once
/// rather than twice — the file check stays quiet about a path that was never
/// legal in the first place.
#[test]
fn an_escaping_script_path_is_one_complaint_and_not_two() {
    let run = checked("script-escape", "../elsewhere.md");
    assert!(run.failed, "{}", run.output);
    run.says("escapes the project root");
    run.silent_about("warning: the project's script");
}
