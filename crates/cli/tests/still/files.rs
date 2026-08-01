//! Where the PNGs land, and what happens when an instant is not one the edit
//! has.

use crate::common::{holds, run_without_tools};
use crate::{still, titled};

/// One instant writes exactly the file that was asked for. Someone who names
/// `frame.png` and means one frame should not have to find out what the command
/// renamed it to.
#[test]
fn one_instant_writes_the_file_that_was_named() {
    let dir = titled("still-one");
    still(&dir, "frame.png", &["--at", "1.0s"])
        .ok()
        .says("frame.png");

    assert!(holds(&dir, "frame.png"), "the named file is the file");
    std::fs::remove_dir_all(dir).ok();
}

/// Several cannot all be that one file, so each takes its timeline frame —
/// zero-padded, so a directory listing is in playback order.
#[test]
fn several_instants_write_several_files_named_by_frame() {
    let dir = titled("still-several");
    still(&dir, "frame.png", &["--at", "0s,1.0s,90"]).ok();

    for name in ["frame-00000.png", "frame-00030.png", "frame-00090.png"] {
        assert!(holds(&dir, name), "no {name} was written");
    }
    assert!(
        !holds(&dir, "frame.png"),
        "with several instants none of them is the bare name"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The same instant in both units is one instant. An agent that asks about a
/// boundary and about the time that boundary falls at has asked one question,
/// and should be charged for one composite.
#[test]
fn naming_one_instant_twice_writes_it_once() {
    let dir = titled("still-dedup");
    let run = still(&dir, "frame.png", &["--at", "1.0s,30"]).ok();

    assert!(holds(&dir, "frame.png"), "one instant, so one plain name");
    assert_eq!(
        run.output
            .lines()
            .filter(|line| line.starts_with("Wrote"))
            .count(),
        1,
        "it composited the same frame twice:\n{}",
        run.output
    );
    std::fs::remove_dir_all(dir).ok();
}

/// An instant the timeline does not reach is refused rather than answered with
/// black. There is no such moment in the edit, and inventing a picture of one
/// is the failure this command exists to make impossible.
#[test]
fn an_instant_past_the_end_is_refused_and_writes_nothing() {
    let dir = titled("still-past");
    let run = still(&dir, "frame.png", &["--at", "600"]);

    assert!(run.failed, "frame 600 of a 300-frame edit:\n{}", run.output);
    run.says("600");
    assert!(!holds(&dir, "frame.png"), "a refusal wrote a file anyway");
    std::fs::remove_dir_all(dir).ok();
}

/// A bare decimal is neither unit, and guessing would make `2` and `2.5` mean
/// instants 2.5 seconds apart. The refusal is clap's, before anything is
/// opened.
#[test]
fn an_instant_in_no_unit_at_all_is_refused() {
    let dir = titled("still-unit");
    let run = still(&dir, "frame.png", &["--at", "2.5"]);

    assert!(run.failed, "`2.5` is neither a frame nor a time");
    assert!(!holds(&dir, "frame.png"));
    std::fs::remove_dir_all(dir).ok();
}

/// PNG encoding is ffmpeg's, like every other byte this project writes, so a
/// machine without it is told which binary is missing rather than left with an
/// empty file.
#[test]
fn without_ffmpeg_it_says_so() {
    let dir = titled("still-no-ffmpeg");
    let path = dir.join("frame.png").display().to_string();
    let run = run_without_tools(&dir, &["still", "--out", &path, "--at", "0"]);

    assert!(
        run.failed,
        "it cannot have composited anything:\n{}",
        run.output
    );
    run.says("ffmpeg");
    assert!(!holds(&dir, "frame.png"));
    std::fs::remove_dir_all(dir).ok();
}
