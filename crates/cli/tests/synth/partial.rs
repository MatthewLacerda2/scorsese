//! Baking less than the whole recipe: a window, a solo, and where the file is
//! allowed to land.
//!
//! The rule under all of it is the one #453 called absolute: a partial bake
//! never reaches `generated/` and never touches the asset. So most of what is
//! asserted here is what did *not* happen.

use std::path::{Path, PathBuf};

use crate::common::{new_project, reload, run_in};

/// A project with one song recipe, unbaked.
fn scored(label: &str) -> PathBuf {
    let dir = new_project(label);
    run_in(&dir, &["synth", "new", "theme", "--kind", "song"]).ok();
    dir
}

/// How many files are sitting in `generated/`.
fn generated(dir: &Path) -> usize {
    std::fs::read_dir(dir.join("generated"))
        .map(|entries| entries.count())
        .unwrap_or_default()
}

fn size(path: &Path) -> usize {
    std::fs::read(path).expect("the file was written").len()
}

/// The whole of the rule, in one test: a window renders, reports and writes —
/// and `generated/` and the asset are exactly as they were.
#[test]
fn a_window_lands_in_the_cache_and_leaves_the_bake_alone() {
    let dir = scored("synth-window");
    let run = run_in(&dir, &["synth", "bake", "theme", "--beats", "0:4"]).ok();
    run.says("part of it");
    run.says("cache/synth/theme.wav");
    run.says("not cached");

    assert_eq!(generated(&dir), 0, "a partial bake wrote into generated/");
    let asset = reload(&dir).assets.first().cloned().expect("one asset");
    assert!(asset.path.is_none(), "the asset was pointed at a fragment");
    assert!(asset.sha256.is_none(), "the asset was hashed against one");
    assert!(asset.needs_generation(), "the recipe still needs baking");
}

/// The point of the feature: four beats of a sixteen-beat piece is a
/// quarter of the file, so the loop it serves costs a quarter as much.
#[test]
fn a_window_is_shorter_than_the_whole_piece() {
    let dir = scored("synth-window-size");
    run_in(&dir, &["synth", "bake", "theme", "--beats", "0:4"]).ok();
    let part = size(&dir.join("cache/synth/theme.wav"));

    run_in(&dir, &["synth", "bake", "theme"]).ok();
    let asset = reload(&dir).assets.first().cloned().expect("one asset");
    let whole = size(&asset.path.expect("baked").resolve(&dir));
    assert!(
        part * 2 < whole,
        "the window is {part} bytes against the whole piece's {whole}"
    );
}

/// The same window said the other way, and the file it overwrites.
#[test]
fn seconds_are_the_same_window_and_a_rebake_replaces_it() {
    let dir = scored("synth-seconds");
    run_in(&dir, &["synth", "bake", "theme", "--seconds", "0:2.5"]).ok();
    let first = size(&dir.join("cache/synth/theme.wav"));
    run_in(&dir, &["synth", "bake", "theme", "--seconds", "0:1"]).ok();
    let second = size(&dir.join("cache/synth/theme.wav"));
    assert!(second < first, "the second bake did not replace the first");
    assert_eq!(generated(&dir), 0);
}

/// A caller that wants the file kept says where.
#[test]
fn out_puts_the_file_where_it_is_told() {
    let dir = scored("synth-out");
    let elsewhere = dir.join("cache/eight-bars.wav");
    let named = elsewhere.display().to_string();
    run_in(
        &dir,
        &["synth", "bake", "theme", "--only", "lead", "--out", &named],
    )
    .ok()
    .says(&named);
    assert!(elsewhere.is_file(), "nothing at {named}");
    assert!(
        !dir.join("cache/synth/theme.wav").exists(),
        "and no default"
    );
}

/// A solo names a track the way the song's own notes name it, so a typo is a
/// refusal rather than a file of silence.
#[test]
fn a_solo_of_a_track_that_is_not_there_is_refused() {
    let dir = scored("synth-solo-typo");
    let run = run_in(&dir, &["synth", "bake", "theme", "--only", "strings"]);
    assert!(run.failed, "a name that matches nothing was accepted");
    run.says("strings");
}

/// An excerpt is a question about one piece of music, so it needs to be told
/// which.
#[test]
fn a_window_without_an_asset_is_refused() {
    let dir = scored("synth-window-nameless");
    let run = run_in(&dir, &["synth", "bake", "--beats", "0:4"]);
    assert!(run.failed, "the window was applied to every recipe");
    run.says("name the asset");
}

/// A one-shot has no arrangement to take a stretch of.
#[test]
fn a_window_of_a_one_shot_is_refused() {
    let dir = new_project("synth-window-oneshot");
    run_in(&dir, &["synth", "new", "thud"]).ok();
    let run = run_in(&dir, &["synth", "bake", "thud", "--beats", "0:4"]);
    assert!(run.failed, "a patch was windowed");
    run.says("one-shot");
}

/// Two clocks for one window is a question with two answers, so clap refuses
/// it before anything is rendered.
#[test]
fn a_window_in_both_units_at_once_is_refused() {
    let dir = scored("synth-two-clocks");
    let run = run_in(
        &dir,
        &[
            "synth",
            "bake",
            "theme",
            "--beats",
            "0:4",
            "--seconds",
            "0:2",
        ],
    );
    assert!(run.failed, "both units were accepted");
}
