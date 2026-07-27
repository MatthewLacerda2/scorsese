//! The render settings, asserted against the delivered file.
//!
//! Every one of these could be made to pass by a command that printed the flag
//! back and encoded whatever it liked, which is why nothing here reads the
//! report: the question is what is in the file.

use crate::common::media::probe::probe;
use crate::common::media::tools;
use crate::{one_shot, render};

/// Everything about the picture stream, in one ffprobe.
const PICTURE: &str = "stream=width,height,r_frame_rate,nb_read_frames,codec_name";

#[test]
fn the_raster_and_the_rate_asked_for_are_what_the_file_has() {
    // A 30-frame timeline on a 30 fps grid, delivered at 15: half as many
    // frames, and a shape the source is not.
    let dir = one_shot("raster", 30);
    let file = render(&dir, "cut.mp4", &["--resolution", "96x48", "--fps", "15"]).file();

    let stream = probe(&tools(), &file, "v:0", PICTURE);
    assert_eq!(stream.number("width"), 96);
    assert_eq!(stream.number("height"), 48);
    assert_eq!(stream.get("r_frame_rate"), "15/1");
    assert_eq!(stream.number("nb_read_frames"), 15);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_projects_own_grid_is_what_a_render_defaults_to() {
    // The one output rate that conforms nothing, and the reason `--fps` is an
    // `Option` rather than a flag with a number baked into it.
    let dir = one_shot("default-rate", 30);
    let file = render(&dir, "cut.mp4", &[]).file();

    let stream = probe(&tools(), &file, "v:0", PICTURE);
    assert_eq!(stream.get("r_frame_rate"), "30/1");
    assert_eq!(stream.number("nb_read_frames"), 30);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_frame_range_delivers_only_that_stretch_of_the_timeline() {
    let dir = one_shot("range", 60);
    let file = render(&dir, "cut.mp4", &["--range", "10:25"]).file();

    let stream = probe(&tools(), &file, "v:0", PICTURE);
    assert_eq!(stream.number("nb_read_frames"), 15);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_container_decides_the_codec_it_is_written_with() {
    // AVI is written with MPEG-4 Part 2, mp4 with H.264. Naming the file is
    // the whole of that decision, and it is the file that has to prove it.
    let dir = one_shot("containers", 15);
    let avi = render(&dir, "cut.avi", &[]).file();
    let mp4 = render(&dir, "cut.mp4", &[]).file();

    let tools = tools();
    assert_eq!(
        probe(&tools, &avi, "v:0", PICTURE).get("codec_name"),
        "mpeg4"
    );
    assert_eq!(
        probe(&tools, &mp4, "v:0", PICTURE).get("codec_name"),
        "h264"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_named_codec_overrides_what_the_container_defaults_to() {
    // AVI carries H.264 too; it is only never the default. The refusal side of
    // this pairing table is `refusals`.
    let dir = one_shot("codec", 15);
    let file = render(&dir, "cut.avi", &["--video-codec", "h264"]).file();

    assert_eq!(
        probe(&tools(), &file, "v:0", PICTURE).get("codec_name"),
        "h264"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_bitrate_budget_reaches_the_encoder() {
    // Asserting the number the file comes out at would be asserting something
    // about x264. That the budget is what decides the size is the claim
    // `--bitrate` makes, and two renders of one shot are what test it.
    let dir = crate::pattern("bitrate", 30);
    let big = ["--resolution", crate::common::media::BIG];
    let lean = render(
        &dir,
        "lean.mp4",
        &[&big[..], &["--bitrate", "60k"]].concat(),
    )
    .file();
    let rich = render(&dir, "rich.mp4", &[&big[..], &["--bitrate", "6M"]].concat()).file();

    let size = |file: &std::path::Path| std::fs::metadata(file).expect("the render").len();
    assert!(
        size(&rich) > size(&lean) * 2,
        "6M ({} bytes) should dwarf 60k ({} bytes)",
        size(&rich),
        size(&lean)
    );
    std::fs::remove_dir_all(&dir).ok();
}
