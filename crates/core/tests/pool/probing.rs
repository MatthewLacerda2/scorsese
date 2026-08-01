//! Probing a pool that did not come in through `import`.

use std::path::PathBuf;

use crate::common::stub_probe::StubProbe;
use crate::common::{new_project, source_file};
use scorsese_core::{
    Asset, AssetId, AssetKind, MediaMetadata, ProbeOutcome, Probed, Project, ProjectPath, Reprobe,
    import_asset, probe_assets, unprobed_assets,
};

/// A project with a file on disk and an assets row that says nothing about it
/// — the shape an agent writing `project.json` by hand leaves behind.
fn written_by_hand(label: &str, kind: AssetKind, name: &str) -> (PathBuf, Project) {
    let (dir, mut project) = new_project(label);
    std::fs::write(dir.join("assets").join(name), b"pretend media").expect("write the media");
    project.assets.push(Asset::imported(
        AssetId::new("shot"),
        kind,
        ProjectPath::new(format!("assets/{name}")),
    ));
    (dir, project)
}

/// A prober that would answer differently, to catch metadata being replaced
/// when it should have been left alone.
fn other_answer() -> StubProbe {
    StubProbe::reporting(MediaMetadata {
        duration_seconds: Some(99.0),
        ..MediaMetadata::default()
    })
}

fn only(report: &[Probed]) -> &Probed {
    assert_eq!(report.len(), 1, "expected one asset, got {report:?}");
    &report[0]
}

fn duration(project: &Project) -> Option<f64> {
    project.assets[0].media.and_then(|it| it.duration_seconds)
}

#[test]
fn an_asset_nobody_probed_gets_its_metadata_recorded() {
    let (dir, mut project) = written_by_hand("probe-hand", AssetKind::Video, "shot.mp4");
    assert_eq!(unprobed_assets(&project), vec![AssetId::new("shot")]);

    let report = probe_assets(&mut project, &dir, &StubProbe::video(), Reprobe::Skip);

    assert_eq!(only(&report).outcome, ProbeOutcome::Recorded);
    assert_eq!(duration(&project), Some(2.0));
    assert_eq!(project.assets[0].media.and_then(|it| it.width), Some(320));
    assert!(
        unprobed_assets(&project).is_empty(),
        "nothing left to probe"
    );
}

#[test]
fn an_already_probed_asset_is_left_alone() {
    // The idempotence the window and MCP both lean on: probing twice is one
    // pass over the table, not one process per file.
    let (dir, mut project) = written_by_hand("probe-twice", AssetKind::Video, "shot.mp4");
    probe_assets(&mut project, &dir, &StubProbe::video(), Reprobe::Skip);

    let report = probe_assets(&mut project, &dir, &other_answer(), Reprobe::Skip);

    assert_eq!(only(&report).outcome, ProbeOutcome::AlreadyKnown);
    assert_eq!(duration(&project), Some(2.0), "the second was never asked");
}

#[test]
fn asking_for_all_of_them_replaces_what_is_recorded() {
    let (dir, mut project) = written_by_hand("probe-all", AssetKind::Video, "shot.mp4");
    probe_assets(&mut project, &dir, &StubProbe::video(), Reprobe::Skip);

    let report = probe_assets(&mut project, &dir, &other_answer(), Reprobe::All);

    assert_eq!(only(&report).outcome, ProbeOutcome::Recorded);
    assert_eq!(duration(&project), Some(99.0));
}

/// The one that decides whether a still can be held on screen: ffprobe calls a
/// still a one-frame video and invents a duration for it, and a trim bounded
/// by that invention would be unusable. Import has always dropped it, and
/// probing has to agree — one file, described one way, however it arrived.
#[test]
fn a_still_keeps_no_duration_and_no_frame_rate_whichever_way_it_arrived() {
    let (dir, mut project) = written_by_hand("probe-still", AssetKind::Image, "card.png");
    probe_assets(&mut project, &dir, &StubProbe::image(), Reprobe::Skip);
    let probed = project.assets[0].media.expect("the metadata just recorded");

    let source = source_file(&dir, "other.png", b"a still");
    import_asset(&mut project, &dir, &source, None, &StubProbe::image()).expect("import");
    let imported = project.assets[1].media.expect("import probes");

    assert_eq!(probed.duration_seconds, None);
    assert_eq!(probed.frame_rate, None);
    assert_eq!(probed.width, Some(64), "the size is real and is kept");
    assert_eq!(probed, imported, "two ways in, one description");
}

#[test]
fn a_file_that_is_not_there_is_reported_and_nothing_is_written() {
    let (dir, mut project) = new_project("probe-missing");
    project.assets.push(Asset::imported(
        AssetId::new("ghost"),
        AssetKind::Video,
        ProjectPath::new("assets/ghost.mp4"),
    ));

    let report = probe_assets(&mut project, &dir, &StubProbe::video(), Reprobe::Skip);

    assert_eq!(only(&report).outcome, ProbeOutcome::Missing);
    assert_eq!(project.assets[0].media, None);
}

#[test]
fn a_probe_that_fails_says_why_and_leaves_the_asset_as_it_was() {
    let (dir, mut project) = written_by_hand("probe-refused", AssetKind::Video, "shot.mp4");
    let refuses = StubProbe::failing("moov atom not found");

    let report = probe_assets(&mut project, &dir, &refuses, Reprobe::Skip);

    assert_eq!(
        only(&report).outcome,
        ProbeOutcome::Failed("moov atom not found".to_owned())
    );
    assert_eq!(project.assets[0].media, None, "never half-filled");
}

/// Nothing with no file to read is reported at all: a title lives in the
/// document and a sketch has no media yet by design, so neither is an omission
/// anybody can act on.
#[test]
fn assets_with_no_file_are_not_probed_and_not_reported() {
    let (dir, mut project) = new_project("probe-nothing");
    project
        .assets
        .push(Asset::text(AssetId::new("title"), "HI"));
    project.assets.push(Asset::sketch(
        AssetId::new("shot"),
        AssetKind::GeneratedVideo,
        "a boat",
    ));

    assert!(unprobed_assets(&project).is_empty());
    let report = probe_assets(&mut project, &dir, &StubProbe::video(), Reprobe::Skip);
    assert!(report.is_empty(), "got {report:?}");
}
