//! `scorsese probe` — filling in what nobody read.
//!
//! Against real media, because the whole point of the command is what ffprobe
//! says about a file: a stub prober would prove the plumbing and nothing about
//! the numbers that end up in the document.

use std::path::{Path, PathBuf};

use crate::common::documents::{self, assets};
use crate::common::media::{colour_video, still, tools};
use crate::common::{Run, reload, run_in, run_without_tools};

/// A project whose assets table names two real files and says nothing else
/// about either — a document written by an agent rather than by `import`.
fn unlooked_at(label: &str) -> PathBuf {
    let dir = documents::project(
        label,
        &[
            assets::raw("shot", "video", "mp4"),
            assets::raw("card", "image", "png"),
        ],
        &[documents::clip("c1", "shot", 0)],
    );
    let tools = tools();
    colour_video(&tools, &dir, "shot", "blue", 2);
    still(&tools, &dir, "card", "red");
    dir
}

fn probe(dir: &Path, arguments: &[&str]) -> Run {
    let mut all = vec!["probe"];
    all.extend_from_slice(arguments);
    run_in(dir, &all)
}

/// The length is the field everything else is waiting on — a ceiling on a
/// right trim reads exactly this — so it is what the assertion is about.
#[test]
fn an_asset_written_by_hand_ends_up_with_its_metadata() {
    let dir = unlooked_at("probe-hand");
    probe(&dir, &[]).ok();

    let project = reload(&dir);
    let media = project
        .asset(&scorsese_core::AssetId::new("shot"))
        .and_then(|asset| asset.media)
        .expect("the metadata just recorded");
    assert_eq!(media.duration_seconds.map(|it| it.round()), Some(2.0));
    assert_eq!(media.width, Some(64));
    std::fs::remove_dir_all(&dir).ok();
}

/// ffprobe calls a still a one-frame video and invents a duration for it. A
/// still has none — how long it is on screen is the clip's business — and a
/// trim bounded by that invention would be unusable. Import has always dropped
/// it; this is the other way in agreeing.
#[test]
fn a_still_is_recorded_with_a_size_and_no_duration() {
    let dir = unlooked_at("probe-still");
    probe(&dir, &[]).ok();

    let media = reload(&dir)
        .asset(&scorsese_core::AssetId::new("card"))
        .and_then(|asset| asset.media)
        .expect("a still is probed too");
    assert_eq!(media.duration_seconds, None);
    assert_eq!(media.frame_rate, None);
    assert_eq!(media.width, Some(64), "its size is real and is kept");
    std::fs::remove_dir_all(&dir).ok();
}

/// Idempotence is what lets this be run after every edit, and what lets the
/// window run it on every open.
#[test]
fn running_it_again_reads_nothing_and_changes_nothing() {
    let dir = unlooked_at("probe-again");
    probe(&dir, &[]).ok();
    let after_first = reload(&dir);

    let run = probe(&dir, &[]).ok();

    run.says("0 probed");
    run.says("2 already known");
    assert_eq!(reload(&dir), after_first, "the document did not move");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_asset_whose_file_is_gone_is_named_and_the_rest_are_still_probed() {
    let dir = unlooked_at("probe-gone");
    std::fs::remove_file(dir.join("assets/card.png")).expect("delete the still");

    let run = probe(&dir, &[]).ok();

    run.says("card");
    run.says("FILE MISSING");
    assert!(
        reload(&dir)
            .asset(&scorsese_core::AssetId::new("shot"))
            .is_some_and(|asset| asset.media.is_some()),
        "one missing file does not stop the others being read"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// It cannot pretend to have probed anything without ffprobe, so it says which
/// binary it wanted rather than reporting a pool of unreadable files.
#[test]
fn without_ffprobe_it_says_so_rather_than_recording_nothing() {
    let dir = unlooked_at("probe-no-tools");

    let run = run_without_tools(&dir, &["probe"]);

    assert!(run.failed, "it cannot do its job:\n{}", run.output);
    run.says("ffprobe");
    std::fs::remove_dir_all(&dir).ok();
}
