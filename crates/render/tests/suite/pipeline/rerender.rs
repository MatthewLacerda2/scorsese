//! Rendering again to the same name: a failure costs the new render, never
//! the last good one (#1086).

use std::path::Path;

use scorsese_core::{Clip, Fit, Fps};
use scorsese_render::{FrameRange, RenderError};

use crate::common::ffmpeg::tools;
use crate::common::scratch;
use crate::common::{fitted, project, video_track};
use crate::pipeline::{colour_asset, render, settings};

/// Every name in `dir` that is not one of the project's own folders.
fn beside(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("listed")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name != "assets")
        .collect();
    names.sort();
    names
}

#[test]
fn a_render_that_fails_part_way_leaves_the_previous_file_as_it_was() {
    let tools = tools();
    let dir = scratch::dir("rerender-fails");
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    let blue = colour_asset(&tools, &dir, "blue", "64x64", 1);
    let timeline = |clips| {
        project(
            vec![red.clone(), blue.clone()],
            vec![video_track("v1", clips)],
        )
    };
    let (out, _) = render(
        &tools,
        &timeline(vec![Clip::at("c1", "red", 0, 30)]),
        &dir,
        FrameRange::ALL,
        Fps::THIRTY,
    );
    let before = std::fs::read(&out).expect("the first cut");
    // A second clip whose source is not a video at all. Filled, so nothing
    // asks its size while planning: the encoder has started and taken the
    // first clip's frames by the time its decoder fails.
    std::fs::write(dir.join("assets/blue.mp4"), b"not a video").expect("a broken source");

    let failed = scorsese_render::Renderer::new(&tools, settings(Fps::THIRTY)).render(
        &timeline(vec![
            Clip::at("c1", "red", 0, 30),
            fitted(Fit::Fill, Clip::at("c2", "blue", 30, 30)),
        ]),
        &dir,
        FrameRange::ALL,
        &out,
    );

    assert!(
        matches!(failed, Err(RenderError::Ffmpeg { .. })),
        "the broken source fails the decode: {failed:?}"
    );
    assert_eq!(
        std::fs::read(&out).expect("still a cut"),
        before,
        "untouched"
    );
    assert_eq!(beside(&dir), ["out.mp4"], "nothing half-written is left");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_render_that_succeeds_replaces_the_previous_file() {
    let tools = tools();
    let dir = scratch::dir("rerender-succeeds");
    let red = colour_asset(&tools, &dir, "red", "64x64", 2);
    let timeline = |frames| {
        project(
            vec![red.clone()],
            vec![video_track("v1", vec![Clip::at("c1", "red", 0, frames)])],
        )
    };
    let (out, _) = render(&tools, &timeline(15), &dir, FrameRange::ALL, Fps::THIRTY);
    let before = std::fs::read(&out).expect("the first cut");

    let (_, report) = render(&tools, &timeline(45), &dir, FrameRange::ALL, Fps::THIRTY);

    assert_eq!(report.frames, 45);
    assert_ne!(std::fs::read(&out).expect("a cut"), before, "replaced");
    assert_eq!(beside(&dir), ["out.mp4"], "nothing is left beside it");
    std::fs::remove_dir_all(&dir).ok();
}
