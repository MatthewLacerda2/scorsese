//! A preview played in pieces (#1448): each stream copied out of a render
//! into a fragmented MP4 a browser can append, its first frame at zero.

use std::path::Path;

use scorsese_render::Cancel;
use scorsese_render::preview::{self, Stream};

use super::common::ffmpeg::{generate, inspect, tools};
use super::common::scratch;

/// The kinds of stream ffprobe finds in `file`, in order.
fn kinds(file: &Path) -> Vec<String> {
    let output = tools()
        .ffprobe()
        .args(["-v", "error", "-show_entries", "stream=codec_type"])
        .args(["-of", "csv=p=0"])
        .arg(file)
        .output()
        .expect("run ffprobe");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The version of the first `trun` box: 1 is the one whose composition
/// offsets are signed, which is what puts a piece's first frame at zero.
fn trun_version(file: &Path) -> Option<u8> {
    let bytes = std::fs::read(file).expect("the piece reads");
    let at = bytes.windows(4).position(|window| window == b"trun")?;
    bytes.get(at + 4).copied()
}

#[test]
fn each_stream_comes_out_alone_fragmented_and_starting_at_zero() {
    let tools = tools();
    let dir = scratch::dir("preview-pieces");
    let render = dir.join("render.mp4");
    // Encoded as a render is, B-frames and all: the reordering they bring is
    // what would push a piece's first frame past zero.
    generate(
        &tools,
        &render,
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=160x90:d=1:r=30",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
        ],
    );

    let picture = dir.join("picture.mp4");
    preview::fragment(&tools, &render, Stream::Picture, &picture, &Cancel::new())
        .expect("the picture is copied out");
    assert_eq!(kinds(&picture), ["video"]);
    assert_eq!(inspect(&tools, &picture).frames, 30, "every frame kept");
    assert_eq!(trun_version(&picture), Some(1), "offsets written signed");

    let sound = dir.join("sound.mp4");
    preview::fragment(&tools, &render, Stream::Sound, &sound, &Cancel::new())
        .expect("the sound is copied out");
    assert_eq!(kinds(&sound), ["audio"]);
    assert!(trun_version(&sound).is_some(), "fragmented");
}

#[test]
fn a_stream_the_render_does_not_have_is_an_error_and_leaves_nothing() {
    let tools = tools();
    let dir = scratch::dir("preview-pieces-silent");
    let render = dir.join("render.mp4");
    generate(
        &tools,
        &render,
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=160x90:d=0.2:r=30",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let sound = dir.join("sound.mp4");
    assert!(preview::fragment(&tools, &render, Stream::Sound, &sound, &Cancel::new()).is_err());
    let left: Vec<_> = std::fs::read_dir(&dir)
        .expect("the scratch folder reads")
        .map(|entry| entry.expect("an entry").file_name())
        .collect();
    assert_eq!(left, ["render.mp4"], "nothing half-written is left");
}
