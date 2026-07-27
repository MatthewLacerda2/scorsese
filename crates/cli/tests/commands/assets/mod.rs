//! `scorsese assets` — what the pool actually contains.
//!
//! Reporting is the whole of what listing does, so these assert the substance
//! of the report — one asset, one state — rather than its shape. The state of
//! each row is a fact about the disk, and every row here is set up so that
//! fact is true before the command runs.

mod gc;
mod verify;

use std::path::{Path, PathBuf};

use crate::common::documents::{self, assets};
use crate::common::{Run, new_project, run_in};

/// A pool holding one asset in each state a listing can report, with the disk
/// arranged to match. `unreadable` is the one absentee: it needs a file that
/// exists and cannot be read, which no portable test can create — the
/// health→severity table covers it in `check`'s own tests instead.
fn every_state(label: &str) -> PathBuf {
    let dir = documents::project(
        label,
        &[
            assets::video("healthy"),
            assets::unprobed_video("unprobed"),
            assets::video("gone"),
            assets::text("title", "THE END"),
            assets::sketch("boat"),
        ],
        &[
            documents::clip("c1", "healthy", 0),
            documents::clip("c2", "unprobed", 30),
            documents::clip("c3", "gone", 60),
            documents::clip("c4", "title", 90),
            documents::clip("c5", "boat", 120),
        ],
    );
    documents::media_file(&dir, "healthy", b"");
    documents::media_file(&dir, "unprobed", b"");
    dir
}

fn list(dir: &Path, arguments: &[&str]) -> Run {
    let mut all = vec!["assets"];
    all.extend_from_slice(arguments);
    run_in(dir, &all)
}

#[test]
fn an_empty_pool_says_it_is_empty_and_says_what_to_do_about_it() {
    let dir = new_project("empty-pool");
    let run = list(&dir, &[]).ok();

    run.says("No assets yet");
    run.says("scorsese import");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn every_state_an_asset_can_be_in_is_reported_as_itself() {
    let dir = every_state("states");
    let run = list(&dir, &[]).ok();

    for (id, state) in [
        ("healthy", "ok"),
        ("unprobed", "not probed"),
        ("gone", "FILE MISSING"),
        ("title", "inline text"),
        ("boat", "awaiting generation"),
    ] {
        let row = row_for(&run, id);
        assert!(
            row.contains(state),
            "asset `{id}` should be `{state}`, and the listing says:\n{row}"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_tally_counts_what_is_unused_and_what_needs_attention() {
    // One missing file among five assets, and one of them referenced by
    // nothing — the two numbers an agent decides what to do next from.
    let dir = documents::project(
        "tally",
        &[assets::video("healthy"), assets::video("gone")],
        &[documents::clip("c1", "healthy", 0)],
    );
    documents::media_file(&dir, "healthy", b"");
    let run = list(&dir, &[]).ok();

    run.says("2 assets, 1 unused, 1 needing attention");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn how_many_clips_lean_on_an_asset_is_part_of_the_answer() {
    // The number `gc` acts on, and the reason deleting one is not obviously
    // safe: an asset two clips share is not the same as one nothing shows.
    let dir = documents::project(
        "uses",
        &[assets::video("shared"), assets::video("orphan")],
        &[
            documents::clip("c1", "shared", 0),
            documents::clip("c2", "shared", 30),
        ],
    );
    documents::media_file(&dir, "shared", b"");
    documents::media_file(&dir, "orphan", b"");
    let run = list(&dir, &[]).ok();

    assert!(row_for(&run, "shared").contains("2 clips"));
    assert!(row_for(&run, "orphan").contains("unused"));
    std::fs::remove_dir_all(&dir).ok();
}

/// The one line of the listing that is about `id`.
#[track_caller]
fn row_for(run: &Run, id: &str) -> String {
    run.output
        .lines()
        .find(|line| line.starts_with(id))
        .unwrap_or_else(|| panic!("no row for `{id}` in:\n{}", run.output))
        .to_owned()
}
