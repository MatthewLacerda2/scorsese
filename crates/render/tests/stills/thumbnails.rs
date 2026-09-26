//! A thumbnail of a source file: small, the right shape, and a refusal rather
//! than an empty file when there is nothing to draw.

use std::path::Path;

use scorsese_core::ProbeMedia;
use scorsese_render::Ffprobe;
use scorsese_render::frames::{Thumbnail, thumbnail};

use crate::common::ffmpeg::{fixture_dir, generate, tools};

/// The width and height of a picture file.
fn size_of(file: &Path) -> (u32, u32) {
    let media = Ffprobe::new(tools())
        .probe(file)
        .expect("the thumbnail probes");
    (media.width.unwrap_or(0), media.height.unwrap_or(0))
}

#[test]
fn each_kind_of_source_draws_a_thumbnail_that_fits_and_is_never_enlarged() {
    let tools = tools();
    let dir = fixture_dir("thumbnails");
    let video = dir.join("wide.mp4");
    let wide = ["-f", "lavfi", "-i", "testsrc=s=640x360:d=2:r=30"];
    generate(
        &tools,
        &video,
        &[&wide[..], &["-pix_fmt", "yuv420p"]].concat(),
    );
    let picture = dir.join("small.png");
    let small = ["-f", "lavfi", "-i", "testsrc=s=100x76", "-frames:v", "1"];
    generate(&tools, &picture, &small);
    let sound = dir.join("tone.wav");
    generate(&tools, &sound, &["-f", "lavfi", "-i", "sine=duration=1"]);

    let cases = [
        (&video, Thumbnail::Frame { at_seconds: 1.0 }, (320, 180)),
        (&picture, Thumbnail::Picture, (100, 76)),
        (&sound, Thumbnail::Waveform, (320, 120)),
    ];
    for (source, what, expected) in cases {
        let out = dir.join(format!("thumb.{}", what.extension()));
        thumbnail(&tools, source, what, &out).expect("the thumbnail is drawn");
        assert_eq!(size_of(&out), expected, "{what:?}");
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_frame_past_the_end_is_refused_rather_than_written_empty() {
    let tools = tools();
    let dir = fixture_dir("thumbnail-past-end");
    let video = dir.join("short.mp4");
    let red = ["-f", "lavfi", "-i", "color=c=red:s=32x32:d=1:r=30"];
    generate(
        &tools,
        &video,
        &[&red[..], &["-pix_fmt", "yuv420p"]].concat(),
    );
    let out = dir.join("thumb.jpg");
    let refused = thumbnail(&tools, &video, Thumbnail::Frame { at_seconds: 9.0 }, &out);
    assert!(refused.is_err(), "{refused:?}");
    std::fs::remove_dir_all(dir).ok();
}
