//! A thumbnail of a source file: small, the right shape, and a refusal rather
//! than an empty file when there is nothing to draw.

use std::path::Path;

use scorsese_core::ProbeMedia;
use scorsese_render::Ffprobe;
use scorsese_render::frames::{Thumbnail, thumbnail};

use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;

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
    let dir = scratch::dir("thumbnails");
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
    let dir = scratch::dir("thumbnail-past-end");
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

/// A `.scor` received from someone else may carry a link where a thumbnail is
/// kept (#1737): the thumbnail replaces the link, and what it named is left as
/// it was.
#[cfg(unix)]
#[test]
fn a_link_at_a_thumbnails_name_is_replaced_never_written_through() {
    let tools = tools();
    let dir = scratch::dir("thumbnail-planted");
    let outside = scratch::dir("thumbnail-planted-outside");
    let victim = outside.join("victim.txt");
    std::fs::write(&victim, "the victim's own bytes").expect("a victim");
    let picture = dir.join("small.png");
    let small = ["-f", "lavfi", "-i", "testsrc=s=100x76", "-frames:v", "1"];
    generate(&tools, &picture, &small);
    let out = dir.join("thumb.png");
    std::os::unix::fs::symlink(&victim, &out).expect("a link");

    thumbnail(&tools, &picture, Thumbnail::Picture, &out).expect("the thumbnail is drawn");

    assert_eq!(
        std::fs::read(&victim).expect("the victim"),
        b"the victim's own bytes",
        "the link's target was written through"
    );
    let landed = std::fs::symlink_metadata(&out).expect("the thumbnail");
    assert!(landed.is_file(), "the link was not replaced");
    assert_eq!(size_of(&out), (100, 76));
    assert_eq!(
        std::fs::read_dir(&dir).expect("the folder").count(),
        2,
        "no scratch is left beside the thumbnail"
    );
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&outside).ok();
}
