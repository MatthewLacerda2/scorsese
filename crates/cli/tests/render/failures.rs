//! When a render stops, and what it says about why.
//!
//! "ffmpeg failed" is not actionable; "decoding assets/boat.mp4 failed" is.
//! These are the two ways a project that loads cleanly still cannot be
//! rendered, and for an unattended run the message *is* the diagnosis — there
//! is nobody to ask a second question.

use crate::common::documents::{self, assets};
use crate::render;

#[test]
fn a_clip_whose_media_is_gone_refuses_the_render_and_names_the_asset() {
    // Exactly what `check` exists to catch before anything is spent. When it
    // has not been run, this is the failure: the assets table and the
    // directory disagree, and the refusal says which asset and which path.
    let dir = documents::project(
        "media-gone",
        &[assets::video("boat")],
        &[documents::clip("c1", "boat", 0)],
    );

    let rendered = render(&dir, "cut.mp4", &[]);
    assert!(
        rendered.run.failed,
        "a clip with no file to decode cannot render:\n{}",
        rendered.run.output
    );
    rendered.run.says("asset `boat` points at");
    rendered.run.says("which is not there");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn media_that_will_not_decode_says_which_file_and_which_end_of_the_pipe() {
    // A truncated download, or a file that was never what its name said. The
    // decode is one of three processes deep, so the stage is named too.
    let dir = documents::project(
        "undecodable",
        &[assets::video("boat")],
        &[documents::clip("c1", "boat", 0)],
    );
    documents::media_file(&dir, "boat", b"this is not a video file");

    let rendered = render(&dir, "cut.mp4", &[]);
    assert!(
        rendered.run.failed,
        "ffmpeg cannot decode this, and neither can we:\n{}",
        rendered.run.output
    );
    rendered.run.says("while decoding");
    rendered.run.says("boat.mp4");
    std::fs::remove_dir_all(&dir).ok();
}
