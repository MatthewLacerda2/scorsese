//! Rendering a slice of the timeline, and rendering more of a clip than it has.

use scorsese_core::{AssetKind, Fps, Project};
use scorsese_render::{FrameRange, Note, RenderError, RenderSettings, Renderer, Resolution};

use crate::common::ffmpeg::{fixture_dir, inspect, mean_rgb, tools};
use crate::common::{clip, clip_from, file_asset, project, video_track};
use crate::{BLACK, GREEN, assert_colour, banded_source, colour_asset, render};

#[test]
fn a_clips_source_in_decides_which_source_frame_it_opens_on() {
    let tools = tools();
    let dir = fixture_dir("seek");
    let bands = banded_source(&tools, &dir);
    let project = project(
        vec![bands],
        // One second in: the green band, not the red one it starts with.
        vec![video_track("v1", vec![clip_from("c1", "bands", 0, 15, 30)])],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 15);
    assert_colour(mean_rgb(&tools, &out, 0), GREEN, "the frame it opens on");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_range_renders_only_its_own_frames_from_the_right_place_in_the_source() {
    let tools = tools();
    let dir = fixture_dir("range");
    let bands = banded_source(&tools, &dir);
    let project = project(
        vec![bands],
        vec![video_track("v1", vec![clip("c1", "bands", 0, 90)])],
    );
    let range: FrameRange = "30:45".parse().expect("a range");

    let (out, report) = render(&tools, &project, &dir, range, Fps::THIRTY);

    assert_eq!(report.frames, 15, "fifteen frames, not ninety");
    assert_eq!(inspect(&tools, &out).frames, 15);
    assert_colour(
        mean_rgb(&tools, &out, 0),
        GREEN,
        "frame 30 of the timeline is one second into the source",
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_clip_longer_than_its_source_finishes_in_black_and_says_so() {
    let tools = tools();
    let dir = fixture_dir("short");
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    // The source holds 30 frames; the clip asks for 60.
    let project = project(
        vec![red],
        vec![video_track("v1", vec![clip("c1", "red", 0, 60)])],
    );

    let (out, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 60, "the clip's length is what was asked for");
    assert_eq!(
        report.notes,
        [Note::ClipRanShort {
            clip: "c1".to_owned(),
            missing: 30
        }],
        "running out of source is reported, never silent"
    );
    assert_colour(
        mean_rgb(&tools, &out, 50),
        BLACK,
        "past the end of the source",
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_media_file_fails_before_anything_is_encoded() {
    let dir = fixture_dir("missing");
    let project: Project = project(
        vec![file_asset("gone", AssetKind::Video)],
        vec![video_track("v1", vec![clip("c1", "gone", 0, 30)])],
    );
    let settings = RenderSettings::new(Resolution::new(64, 64).expect("raster"), Fps::THIRTY);
    let out = dir.join("out.mp4");

    let error = Renderer::new(&tools(), settings)
        .render(&project, &dir, FrameRange::ALL, &out)
        .expect_err("must fail");

    assert!(
        matches!(error, RenderError::MissingMedia { .. }),
        "got {error:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
