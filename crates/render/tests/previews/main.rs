//! Previews (#542): a reduced quality draws fewer pixels of the same picture,
//! reads a proxy where one is made, and a delivery never reads one.
//!
//! The proxies here are stand-ins of a **different colour** from their
//! originals, so which file a frame was decoded from is visible in the frame:
//! red is the original, blue the proxy. A real proxy looks like its original,
//! which is exactly why a test of *which one was read* cannot use one.

#[path = "../common/mod.rs"]
mod common;

mod proxies;

use std::path::Path;

use scorsese_core::{AssetKind, Fit, Fps, Frames, Project};
use scorsese_render::preview::{self, Preview, Proxies, Quality};
use scorsese_render::{Frame, FrameRange, RenderSettings, Renderer, Resolution, Tools};

use common::ffmpeg::{fixture_dir, generate, generate_asset, mean_rgb, tools};
use common::{clip, fitted, project, video_track};

/// The hash the fixture's source is recorded under — anything will do, as
/// long as the proxy is filed under the same one.
const HASH: &str = "0f1e";

/// A red source of `side` pixels square on one clip, fitted as `fit`, with a
/// blue stand-in filed as its proxy in the project's own proxy folder.
fn red_with_blue_proxy(tools: &Tools, root: &Path, side: u32, fit: Fit) -> Project {
    let source = format!("color=c=red:s={side}x{side}:d=1:r=30");
    let mut asset = generate_asset(
        tools,
        root,
        "shot",
        AssetKind::Video,
        &["-f", "lavfi", "-i", &source, "-pix_fmt", "yuv420p"],
    );
    asset.sha256 = Some(HASH.to_owned());
    let stand_in = format!("color=c=blue:s={}x{}:d=1:r=30", side / 2, side / 2);
    generate(
        tools,
        &preview::folder(root).join(preview::file_name(HASH)),
        &["-f", "lavfi", "-i", &stand_in, "-pix_fmt", "yuv420p"],
    );
    project(
        vec![asset],
        vec![video_track(
            "v1",
            vec![fitted(fit, clip("c1", "shot", 0, 30))],
        )],
    )
}

fn raster(side: u32) -> Resolution {
    Resolution::new(side, side).expect("a legal raster")
}

/// The share of a frame's pixels that are clearly red, and clearly blue.
fn shares(frame: &Frame) -> (f64, f64) {
    let pixels: Vec<&[u8]> = frame.bytes().chunks_exact(4).collect();
    let count = |test: fn(&[u8]) -> bool| {
        pixels.iter().filter(|pixel| test(pixel)).count() as f64 / pixels.len() as f64
    };
    (
        count(|p| p[0] > 180 && p[2] < 80),
        count(|p| p[2] > 180 && p[0] < 80),
    )
}

/// One frame of `project`, drawn at `side` square, as a preview when there is
/// one.
fn still(
    tools: &Tools,
    root: &Path,
    project: &Project,
    side: u32,
    as_preview: Option<Preview>,
) -> Frame {
    let renderer = Renderer::new(tools, RenderSettings::new(raster(side), Fps::THIRTY));
    let renderer = match as_preview {
        Some(preview) => renderer.with_preview(preview),
        None => renderer,
    };
    renderer
        .still(project, root, Frames(5))
        .expect("the frame composites")
}

#[test]
fn a_reduced_preview_reads_the_proxy_and_nothing_else_does() {
    let tools = tools();
    let root = fixture_dir("preview-proxy");
    let project = red_with_blue_proxy(&tools, &root, 32, Fit::Fit);
    let proxies = Proxies::made_in(&preview::folder(&root), &project);
    assert_eq!(
        proxies.len(),
        1,
        "the stand-in is found by its source's hash"
    );
    let with = |quality| Some(Preview::new(quality).with_proxies(proxies.clone()));

    let (red, _) = shares(&still(&tools, &root, &project, 32, None));
    assert!(red > 0.9, "a plain still reads the original: {red}");
    let (_, blue) = shares(&still(&tools, &root, &project, 16, with(Quality::Half)));
    assert!(blue > 0.9, "half quality reads the proxy: {blue}");
    let (red, _) = shares(&still(&tools, &root, &project, 32, with(Quality::Full)));
    assert!(red > 0.9, "full quality is the render's picture: {red}");

    // A delivery is a Renderer nobody handed a preview: the proxy sits right
    // there in the project's cache, and the file still carries the original.
    let out = root.join("out.mp4");
    Renderer::new(&tools, RenderSettings::new(raster(32), Fps::THIRTY))
        .render(&project, &root, FrameRange::ALL, &out)
        .expect("the project renders");
    let (r, _, b) = mean_rgb(&tools, &out, 5);
    assert!(r > 180 && b < 80, "a render reads originals: {r} {b}");
    std::fs::remove_dir_all(&root).ok();
}

/// A `native` clip is measured in its source's own pixels, so a preview at
/// half the raster must halve it too — or half quality would show a quarter
/// of the frame covered where the film has a sixteenth. And the proxy, half
/// the size of its original, must arrive at that same rectangle.
#[test]
fn a_native_clip_is_the_same_share_of_the_frame_at_every_quality() {
    let tools = tools();
    let root = fixture_dir("preview-native");
    let project = red_with_blue_proxy(&tools, &root, 16, Fit::Native);
    let full = shares(&still(&tools, &root, &project, 64, None)).0;
    assert!(
        (full - 1.0 / 16.0).abs() < 0.01,
        "16px of a 64px frame: {full}"
    );

    let originals = Some(Preview::new(Quality::Half));
    let half = shares(&still(&tools, &root, &project, 32, originals)).0;
    assert!((half - full).abs() < 0.01, "halved with the raster: {half}");

    let proxies = Proxies::made_in(&preview::folder(&root), &project);
    let proxied = Some(Preview::new(Quality::Quarter).with_proxies(proxies));
    let blue = shares(&still(&tools, &root, &project, 16, proxied)).1;
    assert!(
        (blue - full).abs() < 0.02,
        "the proxy lands on the same rectangle: {blue}"
    );
    std::fs::remove_dir_all(&root).ok();
}
