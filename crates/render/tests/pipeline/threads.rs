//! The same frames, however many threads draw them.
//!
//! Compositing runs on a pool of workers and the frames they finish are written
//! in order — so the thread count must be invisible in the output. The golden
//! renders check that against committed references at whatever the machine's
//! default pool is; this checks the two ends against each other on any machine,
//! which is the half a reference cannot state.

use std::path::{Path, PathBuf};

use scorsese_core::{Easing, Fps, Frames, Keyframe, KeyframeTrack, Project, PropertyPath};
use scorsese_render::{FrameRange, Renderer, Tools, Workers, frames};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, text_asset, video_track};
use crate::{colour_asset, settings};

/// A cut with something for every path through a segment: two decoded layers,
/// the upper one moving and fading so it is transformed rather than copied, and
/// a title over both — which is a layer drawn once and shared by every worker
/// rather than decoded per frame.
fn layered(tools: &Tools, dir: &Path) -> Project {
    let red = colour_asset(tools, dir, "red", "64x64", 2);
    let green = colour_asset(tools, dir, "green", "64x64", 2);
    let mut over = clip("over", "green", 0, 60);
    over.keyframes = vec![
        ramp("opacity", &[(0, 0.0), (59, 1.0)]),
        ramp("transform.position.x", &[(0, -0.4), (59, 0.4)]),
        ramp("transform.scale.x", &[(0, 0.5), (59, 1.5)]),
    ];
    project(
        vec![red, green, text_asset("title")],
        vec![
            video_track("v1", vec![clip("under", "red", 0, 60)]),
            video_track("v2", vec![over]),
            video_track("v3", vec![clip("caption", "title", 0, 60)]),
        ],
    )
}

/// A property travelling linearly through `(frame, value)` points.
fn ramp(property: &str, points: &[(u64, f64)]) -> KeyframeTrack {
    KeyframeTrack::new(
        PropertyPath::new(property),
        points
            .iter()
            .map(|&(t, value)| Keyframe {
                t: Frames(t),
                value,
                easing: Easing::Linear,
            })
            .collect(),
    )
}

/// Renders the whole timeline on `workers` threads.
fn render_on(
    tools: &Tools,
    project: &Project,
    dir: &Path,
    workers: usize,
    name: &str,
) -> (PathBuf, u64) {
    let out = dir.join(name);
    let report = Renderer::new(tools, settings(Fps::THIRTY))
        .with_workers(Workers::new(workers))
        .render(project, dir, FrameRange::ALL, &out)
        .expect("the render succeeds");
    (out, report.frames)
}

#[test]
fn the_thread_count_changes_nothing_about_the_frames() {
    let tools = tools();
    let dir = fixture_dir("threads");
    let project = layered(&tools, &dir);

    let (serial, one_frames) = render_on(&tools, &project, &dir, 1, "one.mp4");
    let (parallel, eight_frames) = render_on(&tools, &project, &dir, 8, "eight.mp4");

    assert_eq!(one_frames, 60);
    assert_eq!(eight_frames, 60, "the pool does not change what is written");
    let resolution = settings(Fps::THIRTY).resolution;
    // Every fourth frame, first and last included: an out-of-order write would
    // put a neighbouring frame's picture here, and every frame of this fixture
    // differs from its neighbours because the upper layer never stops moving.
    for index in (0..60).step_by(4).chain([59]) {
        let alone = frames::extract(&tools, &serial, index, resolution).expect("a serial frame");
        let pooled = frames::extract(&tools, &parallel, index, resolution).expect("a pooled frame");
        assert!(
            alone == pooled,
            "frame {index} differs between one worker and eight"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_pool_of_more_workers_than_frames_still_renders_them_all() {
    let tools = tools();
    let dir = fixture_dir("threads-wide");
    let red = colour_asset(&tools, &dir, "red", "64x64", 1);
    let project = project(
        vec![red],
        vec![video_track("v1", vec![clip("c1", "red", 0, 3)])],
    );

    let (_, written) = render_on(&tools, &project, &dir, 64, "wide.mp4");

    assert_eq!(written, 3, "three frames, sixty-four workers, no deadlock");
    std::fs::remove_dir_all(&dir).ok();
}
