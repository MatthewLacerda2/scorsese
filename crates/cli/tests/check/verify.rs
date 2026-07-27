//! `--verify`: the half of the answer that costs I/O.
//!
//! Existence is always checked. Re-hashing is opt-in, so the default run stays
//! cheap enough to sit inside an edit loop — and everything hashing finds is a
//! warning, which never changes the exit code either way.

use crate::common::documents::assets::video;
use crate::common::documents::{clip, media_file, project};
use crate::common::run_in;

/// A project whose one file no longer hashes to what was recorded at import.
fn changed_project(label: &str) -> std::path::PathBuf {
    let dir = project(label, &[video("boat")], &[clip("c1", "boat", 0)]);
    media_file(&dir, "boat", b"not what was imported");
    dir
}

#[test]
fn a_changed_file_goes_unnoticed_without_verify() {
    let run = run_in(&changed_project("changed-default"), &["check"]);

    assert!(!run.failed, "{}", run.output);
    run.says("no problems, nothing to warn about");
    run.says("(hashes not checked — pass --verify to re-hash every file)");
}

#[test]
fn verify_reports_a_changed_file_as_a_warning() {
    let run = run_in(&changed_project("changed-verify"), &["check", "--verify"]);

    assert!(
        !run.failed,
        "changed media still renders — it must not fail:\n{}",
        run.output
    );
    run.says("warning: asset `boat`");
    run.says("changed since import");
    run.says("no problems, 1 warning(s)");
    run.silent_about("hashes not checked");
}

#[test]
fn verify_leaves_a_healthy_project_healthy() {
    let dir = project("verify-healthy", &[video("boat")], &[clip("c1", "boat", 0)]);
    media_file(&dir, "boat", b"");

    let run = run_in(&dir, &["check", "--verify"]);
    assert!(!run.failed, "{}", run.output);
    run.says("no problems, nothing to warn about");
}
