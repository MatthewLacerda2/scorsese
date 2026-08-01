//! What `scorsese assets` reports about each asset.

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{
    AssetHealth, AssetKind, AssetStatus, Clip, ClipId, Frames, GenerationState, HashCheck, Track,
    TrackId, TrackKind, asset_status, import_asset,
};

/// A project with one imported video, and the health of that one asset.
fn imported(label: &str) -> (std::path::PathBuf, scorsese_core::Project) {
    let (dir, mut project) = new_project(label);
    let source = source_file(&dir, "clip.mp4", b"a video");
    import_asset(&mut project, &dir, &source, None, &StubProbe::video()).expect("import");
    (dir, project)
}

fn only(rows: &[AssetStatus]) -> &AssetStatus {
    assert_eq!(rows.len(), 1, "expected one asset");
    &rows[0]
}

#[test]
fn a_freshly_imported_asset_is_healthy_and_unused() {
    let (dir, project) = imported("status-ok");
    let rows = asset_status(&project, &dir, HashCheck::Verify);
    assert_eq!(only(&rows).health, AssetHealth::Ok);
    assert_eq!(only(&rows).clip_count, 0);
}

#[test]
fn a_deleted_file_is_reported_missing() {
    let (dir, project) = imported("status-missing");
    std::fs::remove_file(dir.join("assets/clip.mp4")).expect("delete");
    let rows = asset_status(&project, &dir, HashCheck::Skip);
    assert_eq!(only(&rows).health, AssetHealth::Missing);
    assert!(only(&rows).health.needs_attention());
}

#[test]
fn a_changed_file_is_only_noticed_when_hashes_are_checked() {
    let (dir, project) = imported("status-tampered");
    std::fs::write(dir.join("assets/clip.mp4"), b"something else entirely").expect("overwrite");

    let skipped = asset_status(&project, &dir, HashCheck::Skip);
    assert_eq!(
        only(&skipped).health,
        AssetHealth::Ok,
        "not checked, not noticed"
    );

    let verified = asset_status(&project, &dir, HashCheck::Verify);
    assert!(
        matches!(only(&verified).health, AssetHealth::HashMismatch { .. }),
        "got {:?}",
        only(&verified).health
    );
}

/// It used to be a fact; it is now something to fix. Features read this
/// metadata — a source's own length is what bounds a trim — so an asset nobody
/// has looked at is one those features silently skip, and `scorsese probe` is
/// the thing to do about it.
#[test]
fn an_asset_nobody_has_probed_needs_attention() {
    let (dir, mut project) = imported("status-unprobed");
    project.assets[0].media = None;

    let rows = asset_status(&project, &dir, HashCheck::Skip);

    assert_eq!(only(&rows).health, AssetHealth::Unprobed);
    assert!(only(&rows).health.needs_attention());
}

#[test]
fn a_sketch_is_awaiting_generation_not_broken() {
    let (dir, mut project) = new_project("status-sketch");
    project.assets.push(scorsese_core::Asset::sketch(
        scorsese_core::AssetId::new("shot"),
        AssetKind::GeneratedVideo,
        "a city at dawn",
    ));
    let rows = asset_status(&project, &dir, HashCheck::Verify);
    assert_eq!(
        only(&rows).health,
        AssetHealth::Awaiting(GenerationState::Sketch)
    );
    assert!(
        !only(&rows).health.needs_attention(),
        "a sketch is not a fault"
    );
}

#[test]
fn a_text_asset_has_no_file_to_check() {
    let (dir, mut project) = new_project("status-text");
    let mut text = scorsese_core::Asset::imported(
        scorsese_core::AssetId::new("title"),
        AssetKind::Text,
        scorsese_core::ProjectPath::new("unused"),
    );
    text.path = None;
    text.text = Some("SCORSESE".to_owned());
    project.assets.push(text);

    let rows = asset_status(&project, &dir, HashCheck::Verify);
    assert_eq!(only(&rows).health, AssetHealth::Inline);
}

#[test]
fn clip_counts_come_from_the_timeline() {
    let (dir, mut project) = imported("status-uses");
    let asset = project.assets[0].id.clone();
    let mut track = Track::new(TrackId::new("v1"), TrackKind::Video);
    track.clips.push(Clip::new(
        ClipId::new("a"),
        asset.clone(),
        Frames::ZERO,
        Frames(30),
    ));
    track
        .clips
        .push(Clip::new(ClipId::new("b"), asset, Frames(30), Frames(30)));
    project.tracks.push(track);

    let rows = asset_status(&project, &dir, HashCheck::Skip);
    assert_eq!(only(&rows).clip_count, 2, "one asset, two references");
}
