//! Prompt clips through the whole pipeline: a card, no decoder, no provider.
//!
//! The money feature of the sketch lifecycle is that this render costs
//! nothing, so what these assert is that it *happens* — a project of prompts
//! produces a watchable file — and that media a prompt is not entitled to
//! never reaches the picture.

use std::path::PathBuf;

use scorsese_core::{AssetKind, Fps, GenerationState, Project};
use scorsese_render::{
    FrameRange, Note, RenderReport, RenderSettings, Renderer, Resolution, StandIn, Tools,
};

use crate::common::ffmpeg::{fixture_dir, generate, inspect, mean_rgb, tools};
use crate::common::prompts::in_state;
use crate::common::{clip, project, sketch_asset, stale_asset, video_track};

/// Big enough for a card's text to be more than one pixel of ink.
const RASTER: (u32, u32) = (256, 144);

fn render_at(tools: &Tools, project: &Project, root: &std::path::Path) -> (PathBuf, RenderReport) {
    let out = root.join("out.mp4");
    let resolution = Resolution::new(RASTER.0, RASTER.1).expect("a legal raster");
    let report = Renderer::new(tools, RenderSettings::new(resolution, Fps::THIRTY))
        .render(project, root, FrameRange::ALL, &out)
        .expect("a prompt clip renders");
    (out, report)
}

/// A card is gray: no channel leans, and it is neither black nor white.
#[track_caller]
fn assert_gray_card(found: (u8, u8, u8), what: &str) {
    let (red, green, blue) = found;
    let lean = red.abs_diff(green).max(green.abs_diff(blue));
    assert!(
        lean <= 6 && (20..=140).contains(&red),
        "{what}: expected a gray card, found {found:?}"
    );
}

#[test]
fn a_sketch_renders_as_a_card_with_no_decoder_and_no_provider() {
    let tools = tools();
    let dir = fixture_dir("sketch");
    let project = project(
        vec![sketch_asset("veo1")],
        vec![video_track("v1", vec![clip("c1", "veo1", 0, 20)])],
    );

    let (out, report) = render_at(&tools, &project, &dir);

    assert_eq!(report.frames, 20);
    assert_eq!(inspect(&tools, &out).frames, 20);
    assert!(report.notes.is_empty(), "a sketch is not a fault");
    assert_gray_card(mean_rgb(&tools, &out, 10), "a sketch clip");
    std::fs::remove_dir_all(&dir).ok();
}

/// `stale` means the prompt was edited after the media was generated, so the
/// file is still there and is the wrong shot. Showing it would be showing
/// footage the project no longer asks for — silently, which is the worst way.
#[test]
fn a_stale_prompt_renders_its_card_and_not_the_media_it_still_has() {
    let tools = tools();
    let dir = fixture_dir("stale");
    generate(
        &tools,
        &dir.join("generated/veo1.mp4"),
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=64x64:d=2:r=30",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let project = project(
        vec![stale_asset("veo1")],
        vec![video_track("v1", vec![clip("c1", "veo1", 0, 20)])],
    );

    let (out, report) = render_at(&tools, &project, &dir);

    assert!(report.notes.is_empty(), "stale media is not missing media");
    assert_gray_card(mean_rgb(&tools, &out, 10), "a stale clip");
    std::fs::remove_dir_all(&dir).ok();
}

/// The fallback. A generated file that has gone — deleted, or never copied
/// along with the project — is a card and a note, not a dead render: the
/// preview is still worth watching and the report says exactly what to
/// regenerate.
#[test]
fn a_generated_file_that_is_gone_falls_back_to_a_card_and_says_so() {
    let tools = tools();
    let dir = fixture_dir("gone");
    let project = project(
        vec![in_state(
            "veo1",
            AssetKind::GeneratedVideo,
            GenerationState::Generated,
        )],
        vec![video_track("v1", vec![clip("c1", "veo1", 0, 20)])],
    );

    let (out, report) = render_at(&tools, &project, &dir);

    assert_eq!(
        report.notes,
        [Note::GeneratedMissing {
            clip: "c1".to_owned(),
            asset: "veo1".to_owned(),
            stood_in: StandIn::SlugCard,
        }]
    );
    assert_eq!(
        inspect(&tools, &out).frames,
        20,
        "the render still happened"
    );
    assert_gray_card(mean_rgb(&tools, &out, 10), "a generated clip with no file");
    std::fs::remove_dir_all(&dir).ok();
}

/// The card is the prompt and not just a gray rectangle: more words is more
/// white ink on the panel. What the words *look like* is the `slugs` golden
/// fixture's business, which is the only honest reference for a glyph.
#[test]
fn the_prompt_is_what_is_on_the_card() {
    let tools = tools();
    let (dir, wordy_dir) = (fixture_dir("prompt-ink"), fixture_dir("prompt-ink-long"));
    let blank = project(
        vec![sketch_asset("veo1")],
        vec![video_track("v1", vec![clip("c1", "veo1", 0, 6)])],
    );
    let mut wordy = blank.clone();
    wordy.assets[0].prompt = Some("WWWW ".repeat(80));

    let bare = mean_rgb(&tools, &render_at(&tools, &blank, &dir).0, 3);
    let inked = mean_rgb(&tools, &render_at(&tools, &wordy, &wordy_dir).0, 3);

    assert!(
        inked.0 > bare.0 + 10,
        "more prompt is more white ink: {bare:?} then {inked:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&wordy_dir).ok();
}
