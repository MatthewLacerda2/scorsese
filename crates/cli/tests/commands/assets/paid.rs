//! `scorsese assets gc` keeps what was paid for (the maintainer, 2026-10-15):
//! an unused Veo shot is listed as kept, and goes only when `--include-paid`
//! says the user asked for it.

use std::path::{Path, PathBuf};

use crate::common::documents::{self, assets};
use crate::common::{reload, run_in};

/// A paid shot and a free video, neither shown, both on disk.
fn paid_and_free(label: &str) -> PathBuf {
    let dir = documents::project(
        label,
        &[
            assets::generated("shot", "video", "mp4"),
            assets::video("spare"),
        ],
        &[],
    );
    documents::file_at(&dir, "generated/shot.mp4", b"bought");
    documents::media_file(&dir, "spare", b"");
    dir
}

fn shot(dir: &Path) -> PathBuf {
    dir.join("generated/shot.mp4")
}

#[test]
fn gc_delete_keeps_a_paid_file_and_says_why() {
    let dir = paid_and_free("gc-paid-kept");
    let run = run_in(&dir, &["assets", "gc", "--delete"]).ok();

    run.says("kept: shot — paid for; pass --include-paid to delete it");
    run.says("unused: spare");
    run.says("Removed 1 assets");
    let project = reload(&dir);
    assert_eq!(project.assets.len(), 1, "the paid row was dropped");
    assert_eq!(project.assets[0].id.as_str(), "shot");
    assert!(shot(&dir).is_file(), "the paid file was deleted");
    assert!(
        !dir.join("assets/spare.mp4").exists(),
        "the free one stayed"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn gc_include_paid_deletes_it_when_asked() {
    let dir = paid_and_free("gc-paid-asked");
    let run = run_in(&dir, &["assets", "gc", "--delete", "--include-paid"]).ok();

    run.says("unused: shot (paid for)");
    run.says("Removed 2 assets, deleted 2 files");
    assert!(reload(&dir).assets.is_empty());
    assert!(!shot(&dir).exists(), "the asked-for file survived");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn gc_with_only_paid_media_unused_collects_nothing() {
    let dir = documents::project(
        "gc-paid-only",
        &[assets::generated("shot", "video", "mp4")],
        &[],
    );
    documents::file_at(&dir, "generated/shot.mp4", b"bought");
    let run = run_in(&dir, &["assets", "gc", "--delete"]).ok();

    run.says("Nothing to collect");
    assert_eq!(reload(&dir).assets.len(), 1);
    assert!(shot(&dir).is_file());
    std::fs::remove_dir_all(&dir).ok();
}
