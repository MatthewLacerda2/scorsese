//! What the report says a render did, held against what it actually did.
//!
//! For a headless render these lines are the only account anyone gets, so the
//! numbers in them are checked against the file rather than taken on trust: a
//! running time is the frame count and the rate, and both are things ffprobe
//! can be asked about independently.

use crate::common::media::probe::probe;
use crate::common::media::tools;
use crate::{one_shot, render};

#[test]
fn the_report_counts_the_frames_and_the_running_time() {
    // 45 frames at 30 fps is a second and a half, and the file has to agree.
    let dir = one_shot("counted", 45);
    let rendered = render(&dir, "cut.mp4", &[]);

    rendered.run.says("45 frames at 30 fps");
    rendered.run.says("(1.50s)");
    rendered.run.says("64x64");
    let file = rendered.file();

    let stream = probe(&tools(), &file, "v:0", "stream=nb_read_frames,r_frame_rate");
    assert_eq!(stream.number("nb_read_frames"), 45);
    assert_eq!(stream.get("r_frame_rate"), "30/1");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_shape_of_the_file_is_said_every_time() {
    // Not behind a flag: the container and both codecs used to be an
    // inference, and an inference nobody prints is one nobody checks.
    let dir = one_shot("shape", 15);
    let rendered = render(&dir, "cut.avi", &[]);

    rendered.run.says("format avi (mpeg4 + pcm_s16le)");
    let _ = rendered.file();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_stills_asked_for_are_written_where_they_were_asked_for() {
    // The description says what the edit claims; these are what the pixels
    // did. So the test is that the files exist and are the render's own size.
    let dir = one_shot("stills", 60);
    let into = dir.join("frames");
    let rendered = render(
        &dir,
        "cut.mp4",
        &["--stills", &into.display().to_string(), "--at", "0,1.0s"],
    );
    let _ = rendered.file();

    let tools = tools();
    for (frame, name) in [(0, "frame-00000.png"), (30, "frame-00030.png")] {
        let still = into.join(name);
        assert!(still.is_file(), "no still for frame {frame} at {name}");
        let picture = probe(&tools, &still, "v:0", "stream=width,height");
        assert_eq!(picture.number("width"), 64);
        assert_eq!(picture.number("height"), 64);
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// Without `--at`, every segment boundary — which is where a cut can be one
/// frame wrong, and the only place a still is worth spending a decode on.
#[test]
fn stills_default_to_the_boundaries_of_the_cut() {
    let dir = one_shot("boundaries", 30);
    let into = dir.join("frames");
    let rendered = render(&dir, "cut.mp4", &["--stills", &into.display().to_string()]);
    rendered.run.says("still(s) in");
    let _ = rendered.file();

    let written: Vec<_> = std::fs::read_dir(&into)
        .expect("the stills directory")
        .filter_map(Result::ok)
        .collect();
    assert!(
        !written.is_empty(),
        "a one-shot cut still has a boundary at its start"
    );
    std::fs::remove_dir_all(&dir).ok();
}
