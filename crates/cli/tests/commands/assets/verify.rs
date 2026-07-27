//! `assets --verify` — the half of the answer that costs I/O.
//!
//! Existence is always checked; re-hashing is opt-in, because it reads every
//! file in the pool and turns an instant listing into seconds. That trade is
//! only defensible if the expensive answer is a *different* answer, which is
//! what the pair of tests below is for.

use crate::common::documents::{self, assets};

use super::{list, row_for};

/// A pool whose one file no longer hashes to what was recorded at import.
fn changed(label: &str) -> std::path::PathBuf {
    let dir = documents::project(
        label,
        &[assets::video("boat")],
        &[documents::clip("c1", "boat", 0)],
    );
    documents::media_file(&dir, "boat", b"not what was imported");
    dir
}

#[test]
fn a_changed_file_reads_as_healthy_until_the_hashes_are_checked() {
    let dir = changed("changed-default");
    let run = list(&dir, &[]).ok();

    assert!(row_for(&run, "boat").contains("ok"));
    run.says("hashes not checked");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn verify_reports_the_file_that_changed_behind_the_projects_back() {
    let dir = changed("changed-verify");
    let run = list(&dir, &["--verify"]).ok();

    assert!(row_for(&run, "boat").contains("CHANGED SINCE IMPORT"));
    run.says("1 needing attention");
    run.silent_about("hashes not checked");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn verify_leaves_a_pool_that_is_what_it_says_it_is_alone() {
    // The control: without this, the test above would pass just as well if
    // `--verify` reported everything as changed.
    let dir = documents::project(
        "verify-healthy",
        &[assets::video("boat")],
        &[documents::clip("c1", "boat", 0)],
    );
    documents::media_file(&dir, "boat", b"");
    let run = list(&dir, &["--verify"]).ok();

    assert!(row_for(&run, "boat").contains("ok"));
    run.says("0 needing attention");
    std::fs::remove_dir_all(&dir).ok();
}
