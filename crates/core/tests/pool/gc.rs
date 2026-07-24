//! Collecting what nothing references.

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::pool::{GcError, remove_assets, unused_assets};
use scorsese_core::{
    AssetId, Clip, ClipId, Project, Seconds, Track, TrackId, TrackKind, import_asset,
};
use std::path::PathBuf;

/// A project with two imported videos, the first of them used by a clip.
fn project_with_one_used_asset(label: &str) -> (PathBuf, Project, AssetId, AssetId) {
    let (dir, mut project) = new_project(label);
    let used = import_asset(
        &mut project,
        &dir,
        &source_file(&dir, "used.mp4", b"used video"),
        None,
        &StubProbe::video(),
    )
    .expect("import used");
    let spare = import_asset(
        &mut project,
        &dir,
        &source_file(&dir, "spare.mp4", b"spare video"),
        None,
        &StubProbe::video(),
    )
    .expect("import spare");

    let mut track = Track::new(TrackId::new("v1"), TrackKind::Video);
    track.clips.push(Clip::new(
        ClipId::new("c1"),
        used.clone(),
        Seconds(0.0),
        Seconds(1.0),
    ));
    project.tracks.push(track);

    (dir, project, used, spare)
}

#[test]
fn only_unreferenced_assets_are_listed() {
    let (_dir, project, _used, spare) = project_with_one_used_asset("gc-list");
    assert_eq!(unused_assets(&project), vec![spare]);
}

#[test]
fn collecting_removes_the_entry_and_the_file() {
    let (dir, mut project, _used, spare) = project_with_one_used_asset("gc-remove");
    let file = dir.join("assets/spare.mp4");
    assert!(file.is_file());

    let report = remove_assets(&mut project, &dir, std::slice::from_ref(&spare)).expect("collect");

    assert_eq!(report.removed, vec![spare]);
    assert_eq!(report.files_deleted, 1);
    assert!(report.bytes_freed > 0, "freed bytes are reported");
    assert!(!file.exists(), "the file is gone");
    assert_eq!(project.assets.len(), 1, "the used asset is untouched");
    assert!(dir.join("assets/used.mp4").is_file());
}

#[test]
fn a_referenced_asset_is_refused() {
    // Removing it would leave a dangling clip reference, which is exactly
    // what validation exists to prevent.
    let (dir, mut project, used, _spare) = project_with_one_used_asset("gc-refuse");
    let error = remove_assets(&mut project, &dir, &[used]).expect_err("must refuse");
    assert!(
        matches!(error, GcError::StillReferenced { .. }),
        "got {error:?}"
    );
    assert_eq!(project.assets.len(), 2, "nothing was removed");
}

#[test]
fn an_unknown_id_is_refused() {
    let (dir, mut project, _used, _spare) = project_with_one_used_asset("gc-unknown");
    let error = remove_assets(&mut project, &dir, &[AssetId::new("ghost")]).expect_err("refuse");
    assert!(
        matches!(error, GcError::NoSuchAsset { .. }),
        "got {error:?}"
    );
}

#[test]
fn an_already_deleted_file_still_collects() {
    let (dir, mut project, _used, spare) = project_with_one_used_asset("gc-already-gone");
    std::fs::remove_file(dir.join("assets/spare.mp4")).expect("delete by hand");

    let report = remove_assets(&mut project, &dir, &[spare]).expect("collect anyway");
    assert_eq!(report.files_missing, 1);
    assert_eq!(report.files_deleted, 0);
    assert_eq!(
        project.assets.len(),
        1,
        "the table entry is still cleaned up"
    );
}

#[test]
fn the_project_still_validates_after_collecting() {
    let (dir, mut project, _used, spare) = project_with_one_used_asset("gc-valid");
    remove_assets(&mut project, &dir, &[spare]).expect("collect");
    assert_eq!(project.validate(), Ok(()));
}
