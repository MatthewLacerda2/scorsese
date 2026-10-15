//! A sound-only render stopped in the mix, and in the encode after it.
//!
//! The mix is asked directly, since a render stopped in it and one stopped at
//! the stage check after it look the same from outside. The encode is a
//! sound-only render, so nothing is captured or drawn: the checks before it
//! are the stage check before the mix, one a clip in the mix, and the stage
//! check before the encode — each counted by [`Cancel::after`].

use std::path::{Path, PathBuf};

use scorsese_core::{
    Asset, AssetId, AssetKind, Clip, ClipId, Fps, Frames, Project, ProjectPath, Track, TrackId,
    TrackKind,
};

use crate::audio::loudness::deliver;
use crate::audio::{LoudnessTarget, mixdown_unless};
use crate::format::{Container, OutputFormat};
use crate::plan::Plan;
use crate::{Cancel, FrameRange, RenderError, RenderSettings, Renderer, Resolution, Tools};

/// Two tones side by side for [`LENGTH`] frames, in a fresh directory.
fn tones(tools: &Tools, label: &str) -> (Project, PathBuf) {
    let root = crate::scratch::fresh(&format!("scorsese-hush-{label}"), "");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("assets")).expect("create the fixture directory");
    // lavfi's own `sine`, at its eighth of full scale: the level the loudness
    // target's passes were measured stopping on.
    crate::testing::generate(
        tools,
        &root.join("assets/tone.wav"),
        &["-f", "lavfi", "-i", "sine=d=3"],
    );
    let tone = Asset::imported(
        AssetId::new("tone"),
        AssetKind::Audio,
        ProjectPath::new("assets/tone.wav"),
    );
    let track = |id: &str| Track {
        clips: vec![Clip::new(
            ClipId::new(id),
            AssetId::new("tone"),
            Frames(0),
            Frames(LENGTH),
        )],
        ..Track::new(TrackId::new(id), TrackKind::Audio)
    };
    let project = Project {
        assets: vec![tone],
        tracks: vec![track("a1"), track("a2")],
        ..Project::new("hushed", Fps::THIRTY)
    };
    (project, root)
}

const LENGTH: u64 = 90;

/// Renders the tones to a wav under `cancel`; hands back the outcome and
/// whatever the render left in the project's folder beside its assets.
fn render(tools: &Tools, cancel: Cancel, label: &str) -> (Result<u64, RenderError>, Vec<String>) {
    let (project, root) = tones(tools, label);
    let raster = Resolution::new(32, 32).expect("a legal raster");
    let settings = RenderSettings::new(raster, Fps::THIRTY)
        .with_format(OutputFormat::defaults_for(Container::Wav));
    let outcome = Renderer::new(tools, settings)
        .with_cancel(cancel)
        .render(&project, &root, FrameRange::ALL, &root.join("out.wav"))
        .map(|report| report.frames);
    let left = left_in(&root);
    std::fs::remove_dir_all(&root).ok();
    (outcome, left)
}

fn left_in(root: &Path) -> Vec<String> {
    let mut left: Vec<String> = std::fs::read_dir(root)
        .expect("the fixture directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into()
        })
        .filter(|name| name != "assets")
        .collect();
    left.sort();
    left
}

#[test]
fn a_cancel_in_the_mix_stops_before_the_next_clip_is_decoded() {
    let tools = crate::testing::tools();
    let (project, root) = tones(&tools, "mix");
    let plan = Plan::build_sound(&project, Fps::THIRTY, FrameRange::ALL).expect("a plan");
    let raster = Resolution::new(32, 32).expect("a legal raster");
    let settings = RenderSettings::new(raster, Fps::THIRTY);
    let at = (root.as_path(), root.join("out.wav"));
    // The first clip is mixed; the second is refused before its decoder starts.
    let cancel = Cancel::after(1);
    let mixed = mixdown_unless(&tools, &settings, &plan, (at.0, &at.1), &cancel);
    let stopped = matches!(mixed, Err(RenderError::Cancelled { .. }));
    drop(mixed);
    let left = left_in(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(stopped, "the mix went on past the cancel");
    assert!(left.is_empty(), "a stopped mix left {left:?}");
}

#[test]
fn a_cancel_during_the_audio_encode_kills_it_and_leaves_nothing() {
    let tools = crate::testing::tools();
    // Before the mix, both clips, before the encode: the encode's first look
    // at the flag is the one that trips it.
    let (outcome, left) = render(&tools, Cancel::after(4), "encode");
    assert!(
        matches!(outcome, Err(RenderError::Cancelled { written: 0, .. })),
        "{outcome:?}"
    );
    assert!(left.is_empty(), "a stopped encode left {left:?}");
}

#[test]
fn an_untripped_flag_mixes_and_encodes_the_whole_soundtrack() {
    let tools = crate::testing::tools();
    let (outcome, left) = render(&tools, Cancel::new(), "whole");
    outcome.expect("the render finishes");
    assert_eq!(left, ["out.wav"]);
}

#[test]
fn a_cancel_stops_the_loudness_target_before_its_next_pass_over_the_mix() {
    let tools = crate::testing::tools();
    let (project, root) = tones(&tools, "loud");
    let plan = Plan::build_sound(&project, Fps::THIRTY, FrameRange::ALL).expect("a plan");
    let raster = Resolution::new(32, 32).expect("a legal raster");
    let target = LoudnessTarget::new(-14.0).expect("a legal target");
    let settings = RenderSettings::new(raster, Fps::THIRTY)
        .with_format(OutputFormat::defaults_for(Container::Wav))
        .with_loudness(Some(target));
    let out = root.join("out.wav");
    let mixed = mixdown_unless(&tools, &settings, &plan, (&root, &out), &Cancel::new())
        .expect("the mix")
        .expect("something to mix");
    let tripped = Cancel::new();
    tripped.cancel();
    let delivered = deliver(&tools, &settings, mixed.0.path(), &out, &tripped);
    let stopped = matches!(delivered, Err(RenderError::Cancelled { .. }));
    drop(mixed);
    let left = left_in(&root);
    std::fs::remove_dir_all(&root).ok();
    assert!(stopped, "the loudness passes went on: {delivered:?}");
    assert!(left.is_empty(), "a stopped lift left {left:?}");
}
