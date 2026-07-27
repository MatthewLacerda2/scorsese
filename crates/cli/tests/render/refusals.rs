//! What `scorsese render` will not do, and how early it says so.
//!
//! A refusal that arrives after the encode has already run is not a fix for
//! the problem it is meant to fix: the cost of a wrong-shaped file is the
//! minutes spent producing it, and for an unattended agent, the plausible
//! result it then acts on.
//!
//! So these runs point ffmpeg and ffprobe at a binary that does not exist. A
//! refusal that reaches the tools says so — `Tools::discover` complains about
//! ffmpeg by name — and a refusal that beats them cannot. The control test
//! below is what gives the silence its meaning.

use std::path::Path;

use crate::Run;
use crate::common::{new_project, run_without_tools};

/// What `Tools::discover` says when it cannot find them, in the words the
/// error itself uses.
const LOOKED_FOR_FFMPEG: &str = "install ffmpeg";

/// A render that cannot reach ffmpeg however far it gets.
fn render(dir: &Path, arguments: &[&str]) -> Run {
    let mut all = vec!["render"];
    all.extend_from_slice(arguments);
    run_without_tools(dir, &all)
}

/// The control. Without this, the tests below would pass just as well if the
/// render had refused for some entirely different reason before ffmpeg came
/// up — including a `--out` nobody could write to.
#[test]
fn an_accepted_format_gets_all_the_way_to_looking_for_ffmpeg() {
    let dir = new_project("accepted");
    let run = render(&dir, &["--out", "cut.wmv"]);

    assert!(run.failed, "there is no ffmpeg to render with");
    run.says(LOOKED_FOR_FFMPEG);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_refused_pairing_never_gets_as_far_as_ffmpeg() {
    let dir = new_project("pairing");
    let run = render(&dir, &["--out", "cut.wmv", "--video-codec", "h264"]);

    assert!(run.failed, "an ASF carrying H.264 is not a wmv");
    run.says("does not write wmv with h264");
    run.says("wmv2");
    run.silent_about(LOOKED_FOR_FFMPEG);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_extension_we_do_not_write_never_gets_as_far_as_ffmpeg() {
    let dir = new_project("extension");
    let run = render(&dir, &["--out", "cut.mov"]);

    assert!(run.failed, "scorsese does not write QuickTime");
    run.says("does not write `.mov` files");
    run.silent_about(LOOKED_FOR_FFMPEG);
    std::fs::remove_dir_all(&dir).ok();
}

/// The flag decides which combination gets checked, not the file name. Both
/// halves are needed: the first run would pass if the extension were still
/// what mattered, and the second would fail.
#[test]
fn the_container_flag_is_what_gets_checked() {
    let dir = new_project("flag");
    let refused = render(
        &dir,
        &[
            "--out",
            "cut.mp4",
            "--container",
            "wmv",
            "--video-codec",
            "h264",
        ],
    );
    assert!(refused.failed, "wmv is not written with h264");
    refused.says("does not write wmv with h264");
    refused.silent_about(LOOKED_FOR_FFMPEG);

    let accepted = render(
        &dir,
        &[
            "--out",
            "cut.wmv",
            "--container",
            "mp4",
            "--video-codec",
            "h264",
        ],
    );
    accepted.says(LOOKED_FOR_FFMPEG);
    std::fs::remove_dir_all(&dir).ok();
}
