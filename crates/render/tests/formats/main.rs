//! What kind of file a render actually delivers.
//!
//! These ask ffprobe what came out, because that is the question nobody was
//! asking: the encoder happily muxed H.264 into an ASF called `.wmv` and every
//! test in the repository stayed green, since none of them looked at the file
//! as anything but a bag of frames.
//!
//! What these hold is the muxing decision — container, picture codec, sound
//! codec — and the refusal of the combinations we do not write. Pixel-exact
//! frames stay the golden harness's business; the only thing asked about the
//! picture here is that it survived the encoder at all, since a blank stream
//! probes exactly like a good one.

#[path = "../common/mod.rs"]
mod common;

mod containers;
mod defaults;
mod documented;
mod probing;
mod refusals;

use std::path::{Path, PathBuf};

use scorsese_core::{AssetKind, Fps, Project};
use scorsese_render::{FrameRange, OutputFormat, RenderSettings, Renderer, Resolution, Tools};

use common::ffmpeg::{fixture_dir, generate_asset, mean_rgb, tools};
use common::{audio_track, clip, project, video_track};

/// Small on purpose: what these ask of a file is what kind it is, and every
/// container here costs a real encode.
const RASTER: (u32, u32) = (32, 32);

/// One second of red with a tone under it. Both halves matter — the audio
/// codec is only exercised when there is a mix to encode.
fn demo(tools: &Tools, root: &Path) -> Project {
    let red = generate_asset(
        tools,
        root,
        "red",
        AssetKind::Video,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=32x32:d=1:r=30",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let tone = generate_asset(
        tools,
        root,
        "tone",
        AssetKind::Audio,
        &["-f", "lavfi", "-i", "sine=f=440:d=1", "-ar", "48000"],
    );
    project(
        vec![red, tone],
        vec![
            video_track("v1", vec![clip("c-red", "red", 0, 30)]),
            audio_track("a1", vec![clip("c-tone", "tone", 0, 30)]),
        ],
    )
}

/// Renders the demo in `format` to a file called `name`, and reports what
/// ffprobe makes of the result.
///
/// `name` is separate from the format on purpose: the two disagreeing is the
/// case that proves the setting decides and the file name does not.
fn delivered(label: &str, format: OutputFormat, name: &str) -> probing::Delivered {
    delivered_with(label, settings(format), name)
}

/// [`delivered`], for a test that has something else to say about the render
/// than which format it is in.
fn delivered_with(label: &str, settings: RenderSettings, name: &str) -> probing::Delivered {
    let tools = tools();
    let dir = fixture_dir(label);
    let out = render(&tools, &dir, settings, name);
    let found = probing::probe(&tools, &out);
    // Labelled right is not the same as watchable: a pairing that produced a
    // blank or corrupt stream would ffprobe exactly like a good one, so the
    // picture is checked to have survived every encoder we offer.
    let colour = mean_rgb(&tools, &out, 15);
    assert!(
        colour.0 > 200 && colour.1 < 60 && colour.2 < 60,
        "the picture came out {colour:?} rather than red"
    );
    std::fs::remove_dir_all(&dir).ok();
    found
}

fn settings(format: OutputFormat) -> RenderSettings {
    let resolution =
        Resolution::new(RASTER.0, RASTER.1).expect("the fixture raster is a legal one");
    RenderSettings::new(resolution, Fps::THIRTY).with_format(format)
}

fn render(tools: &Tools, dir: &Path, settings: RenderSettings, name: &str) -> PathBuf {
    let project = demo(tools, dir);
    let out = dir.join(name);
    Renderer::new(tools, settings)
        .render(&project, dir, FrameRange::ALL, &out)
        .expect("the render succeeds");
    out
}
