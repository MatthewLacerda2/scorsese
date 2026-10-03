//! A render stopped part way through: it says how far it got, and leaves no
//! file behind (#647).
//!
//! Here rather than beside the other pipeline tests because the cancel lands
//! at a known frame only with [`Cancel::after`], which exists for tests in
//! this crate alone — racing a thread against the encoder would make the
//! frame it lands on whatever the machine happened to do.

use std::path::PathBuf;

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, Geometry, Project, ProjectPath, Rgba,
    Shape, Track, TrackId, TrackKind,
};

use crate::{Cancel, FrameRange, RenderError, RenderSettings, Renderer, Resolution, Tools};

/// How long the timeline is, in frames: three seconds, so a cancel at frame
/// ten is plainly part way through.
pub(super) const LENGTH: u64 = 90;

/// A project with something decoded and something drawn on it, so both kinds
/// of layer are mid-stream when the cancel lands. Built in a fresh directory,
/// which is handed back with it.
pub(super) fn fixture(tools: &Tools, label: &str) -> (Project, PathBuf) {
    let root = std::env::temp_dir().join(format!("scorsese-cancel-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("assets")).expect("create the fixture directory");
    let made = tools
        .ffmpeg()
        .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg("color=c=red:s=32x32:d=3:r=30")
        .args(["-pix_fmt", "yuv420p"])
        .arg(root.join("assets/red.mp4"))
        .output()
        .expect("run ffmpeg");
    assert!(made.status.success(), "the source could not be generated");

    let red = Asset::imported(
        AssetId::new("red"),
        AssetKind::Video,
        ProjectPath::new("assets/red.mp4"),
    );
    let card = Geometry::Rectangle {
        width: 0.5,
        height: 0.5,
        radius: 0.0,
    };
    let card = Asset::shape(AssetId::new("card"), Shape::filled(card, Rgba::WHITE));
    let clip = |id: &str, asset: &str| {
        Clip::new(
            ClipId::new(id),
            AssetId::new(asset),
            Frames(0),
            Frames(LENGTH),
        )
    };
    let track = |id: &str, clips| Track {
        clips,
        ..Track::new(TrackId::new(id), TrackKind::Video)
    };
    let project = Project {
        assets: vec![red, card],
        tracks: vec![
            track("v1", vec![clip("c1", "red")]),
            track("v2", vec![clip("c2", "card")]),
        ],
        ..Project::new("cancelled", Fps::THIRTY)
    };
    (project, root)
}

/// Renders the fixture under `cancel`, and fails if a stopped render left its
/// file behind.
fn render(tools: &Tools, cancel: Cancel, label: &str) -> Result<u64, RenderError> {
    let (project, root) = fixture(tools, label);
    let out = root.join("out.mp4");
    let raster = Resolution::new(32, 32).expect("a legal raster");
    let outcome = Renderer::new(tools, RenderSettings::new(raster, Fps::THIRTY))
        .with_cancel(cancel)
        .render(&project, &root, FrameRange::ALL, &out)
        .map(|report| report.frames);
    let left = out.exists();
    std::fs::remove_dir_all(&root).ok();
    assert!(
        !left || outcome.is_ok(),
        "a stopped render left {out:?} behind"
    );
    outcome
}

#[test]
fn a_cancel_mid_picture_stops_at_that_frame_and_removes_the_file() {
    let tools = Tools::discover().expect("ffmpeg and ffprobe must be on PATH");
    // Two checks between stages before the picture starts, then one a frame:
    // so twelve answers of "no" is ten frames encoded and the eleventh refused.
    let outcome = render(&tools, Cancel::after(12), "mid");
    match outcome {
        Err(RenderError::Cancelled { written, of }) => {
            assert_eq!((written, of), (10, LENGTH));
        }
        other => panic!("expected a cancel at frame ten, got {other:?}"),
    }
}

#[test]
fn a_cancel_before_anything_is_drawn_writes_nothing() {
    let tools = Tools::discover().expect("ffmpeg and ffprobe must be on PATH");
    let cancel = Cancel::new();
    cancel.cancel();
    let outcome = render(&tools, cancel, "early");
    assert!(
        matches!(
            outcome,
            Err(RenderError::Cancelled {
                written: 0,
                of: LENGTH
            })
        ),
        "{outcome:?}"
    );
}

#[test]
fn an_untripped_flag_changes_nothing() {
    let tools = Tools::discover().expect("ffmpeg and ffprobe must be on PATH");
    let outcome = render(&tools, Cancel::new(), "whole");
    assert_eq!(outcome.expect("the render finishes"), LENGTH);
}
