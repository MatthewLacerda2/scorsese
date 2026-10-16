//! What was paid for stays unless the user asked for it to go.

use crate::common::new_project;
use scorsese_core::pool::{GcError, Paid, remove_assets, unused_assets};
use scorsese_core::{Asset, AssetId, AssetKind, GenerationState, Project, ProjectPath};
use std::path::{Path, PathBuf};

/// A Veo shot nothing shows, its file under `generated/` with an older
/// generation of the same brief beside it.
fn unused_shot(label: &str) -> (PathBuf, Project, AssetId) {
    let (dir, mut project) = new_project(label);
    std::fs::create_dir_all(dir.join("generated")).expect("make generated/");
    std::fs::write(shot(&dir), b"bought").expect("write the shot");
    std::fs::write(older(&dir), b"bought before").expect("write the older one");
    let id = AssetId::new("shot");
    let mut shot = Asset::sketch(id.clone(), AssetKind::GeneratedVideo, "a boat");
    shot.state = Some(GenerationState::Generated);
    shot.path = Some(ProjectPath::new("generated/shot-new.mp4"));
    project.assets.push(shot);
    (dir, project, id)
}

fn shot(dir: &Path) -> PathBuf {
    dir.join("generated/shot-new.mp4")
}

fn older(dir: &Path) -> PathBuf {
    dir.join("generated/shot-old.mp4")
}

#[test]
fn a_paid_asset_is_listed_but_kept() {
    let (dir, mut project, id) = unused_shot("gc-paid-kept");
    assert_eq!(unused_assets(&project), vec![id.clone()], "it is unused");

    let error = remove_assets(&mut project, &dir, &[id], Paid::Keep).expect_err("kept");

    assert!(matches!(error, GcError::Paid { .. }), "{error:?}");
    assert_eq!(project.assets.len(), 1, "the row stays");
    assert!(shot(&dir).is_file(), "and so does the file");
}

#[test]
fn a_refused_paid_asset_stops_the_whole_collection() {
    // Checked before any file is touched, like every other refusal.
    let (dir, mut project, id) = unused_shot("gc-paid-first");
    let free = AssetId::new("title");
    project.assets.push(Asset::text(free.clone(), "Hello"));

    remove_assets(&mut project, &dir, &[free, id], Paid::Keep).expect_err("kept");

    assert_eq!(project.assets.len(), 2, "nothing was dropped");
}

#[test]
fn asked_to_delete_it_goes_and_older_generations_stay() {
    let (dir, mut project, id) = unused_shot("gc-paid-delete");

    let report = remove_assets(&mut project, &dir, &[id], Paid::Delete).expect("asked");

    assert!(project.assets.is_empty());
    assert_eq!(report.files_deleted, 1);
    assert!(!shot(&dir).exists(), "the asked-for file is gone");
    assert!(
        older(&dir).is_file(),
        "a file no asset names is never collection's to delete"
    );
}

#[test]
fn a_sketch_cost_nothing_and_collects_as_before() {
    let (dir, mut project) = new_project("gc-paid-sketch");
    let id = AssetId::new("idea");
    let sketch = Asset::sketch(id.clone(), AssetKind::GeneratedAudio, "a line");
    assert!(!sketch.is_paid(), "no file, nothing bought");
    project.assets.push(sketch);

    remove_assets(&mut project, &dir, &[id], Paid::Keep).expect("free to collect");
    assert!(project.assets.is_empty());
}
