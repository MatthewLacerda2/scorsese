//! `scorsese assets gc` — collecting what nothing uses.
//!
//! Deleting media is not undoable and the media may be the only copy, so the
//! report is the default and the deletion is asked for twice. Both halves are
//! asserted against the disk: a report that quietly deleted something, and a
//! `--delete` that quietly kept it, are the same bug from opposite ends.

use std::path::{Path, PathBuf};

use crate::common::documents::{self, assets};
use crate::common::{Run, reload, run_in};

/// A pool of one asset a clip shows and one nothing does, both on disk.
fn used_and_unused(label: &str) -> PathBuf {
    let dir = documents::project(
        label,
        &[assets::video("shown"), assets::video("forgotten")],
        &[documents::clip("c1", "shown", 0)],
    );
    documents::media_file(&dir, "shown", b"");
    // Not empty, so the bytes it frees are a number worth reporting.
    documents::media_file(&dir, "forgotten", &[0; 2048]);
    dir
}

fn gc(dir: &Path, arguments: &[&str]) -> Run {
    let mut all = vec!["assets", "gc"];
    all.extend_from_slice(arguments);
    run_in(dir, &all)
}

fn media(dir: &Path, id: &str) -> PathBuf {
    dir.join("assets").join(format!("{id}.mp4"))
}

#[test]
fn gc_reports_what_nothing_uses_and_deletes_none_of_it() {
    let dir = used_and_unused("gc-report");
    let run = gc(&dir, &[]).ok();

    run.says("unused: forgotten");
    run.says("Nothing deleted");
    run.silent_about("unused: shown");

    assert_eq!(reload(&dir).assets.len(), 2, "the table was edited");
    assert!(media(&dir, "forgotten").is_file(), "the file was deleted");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn gc_delete_drops_the_entry_and_unlinks_the_file() {
    let dir = used_and_unused("gc-delete");
    let run = gc(&dir, &["--delete"]).ok();

    run.says("Removed 1 assets, deleted 1 files");
    run.says("2.0 KB");

    let project = reload(&dir);
    assert_eq!(project.assets.len(), 1);
    assert_eq!(project.assets[0].id.as_str(), "shown");
    assert!(!media(&dir, "forgotten").exists(), "the file survived");
    assert!(media(&dir, "shown").is_file(), "a used asset was collected");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn gc_on_a_pool_where_every_asset_is_used_changes_nothing() {
    let dir = documents::project(
        "gc-nothing",
        &[assets::video("shown")],
        &[documents::clip("c1", "shown", 0)],
    );
    documents::media_file(&dir, "shown", b"");
    let run = gc(&dir, &["--delete"]).ok();

    run.says("Every asset is used by at least one clip");
    assert_eq!(reload(&dir).assets.len(), 1);
    assert!(media(&dir, "shown").is_file());
    std::fs::remove_dir_all(&dir).ok();
}

/// A collected asset leaves a project that still validates, which is the point
/// of refusing to collect one a clip still shows.
#[test]
fn what_gc_leaves_behind_is_still_a_loadable_project() {
    let dir = used_and_unused("gc-coherent");
    gc(&dir, &["--delete"]).ok();

    let project = reload(&dir);
    assert_eq!(project.clips().count(), 1);
    std::fs::remove_dir_all(&dir).ok();
}
