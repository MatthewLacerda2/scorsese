//! Text clips through the whole pipeline: no decoder, and a font off disk.

use std::path::{Path, PathBuf};

use scorsese_core::{Asset, AssetId, Fps, Project, Rgba, TextStyle};
use scorsese_render::{
    FrameRange, RenderError, RenderReport, RenderSettings, Renderer, Resolution, Tools,
};

use crate::common::ffmpeg::{fixture_dir, inspect, mean_rgb, tools};
use crate::common::{clip, project, text_asset_in, video_track};

/// Bigger than the 64×64 the other fixtures use: letters need somewhere to be.
const RASTER: (u32, u32) = (256, 144);

fn titled(id: &str, content: &str, color: Rgba) -> Asset {
    Asset {
        style: Some(TextStyle {
            color,
            size: 0.35,
            ..TextStyle::default()
        }),
        ..Asset::text(AssetId::new(id), content)
    }
}

fn render_at(
    tools: &Tools,
    project: &Project,
    root: &Path,
) -> Result<(PathBuf, RenderReport), RenderError> {
    let out = root.join("out.mp4");
    let resolution = Resolution::new(RASTER.0, RASTER.1).expect("a legal raster");
    let report = Renderer::new(tools, RenderSettings::new(resolution, Fps::THIRTY)).render(
        project,
        root,
        FrameRange::ALL,
        &out,
    )?;
    Ok((out, report))
}

/// The one thing a golden fixture cannot say on its own: a text clip has no
/// file, so nothing is decoded for it, and the render still writes every frame.
#[test]
fn a_text_clip_renders_without_a_file_to_decode() {
    let tools = tools();
    let dir = fixture_dir("text");
    let project = project(
        vec![titled("title", "THE END", Rgba::WHITE)],
        vec![video_track("v1", vec![clip("c1", "title", 0, 20)])],
    );

    let (out, report) = render_at(&tools, &project, &dir).expect("a text clip renders");

    assert_eq!(report.frames, 20);
    assert_eq!(inspect(&tools, &out).frames, 20);
    assert!(report.notes.is_empty(), "text never runs short of source");
    let (red, green, blue) = mean_rgb(&tools, &out, 10);
    assert!(
        red > 4 && green > 4 && blue > 4,
        "white letters lift a black frame off zero; found {red},{green},{blue}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Colour is the document's to choose, all the way through the pipeline —
/// core says text *has* a colour and never which one.
#[test]
fn the_colour_the_document_asks_for_reaches_the_file() {
    let tools = tools();
    let dir = fixture_dir("text-colour");
    let project = project(
        vec![titled("title", "THE END", Rgba::opaque(255, 0, 0))],
        vec![video_track("v1", vec![clip("c1", "title", 0, 10)])],
    );

    let (out, _) = render_at(&tools, &project, &dir).expect("a text clip renders");

    let (red, green, blue) = mean_rgb(&tools, &out, 5);
    assert!(
        red > green + 4 && red > blue + 4,
        "red letters make a red-leaning frame; found {red},{green},{blue}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A font the project names has to be there. Refused rather than quietly
/// falling back to the shipped sans: a title in the wrong face is a re-render
/// nobody would know to ask for.
#[test]
fn a_font_the_project_does_not_have_stops_the_render() {
    let tools = tools();
    let dir = fixture_dir("text-font");
    let project = project(
        vec![text_asset_in("title", "assets/Nowhere.ttf")],
        vec![video_track("v1", vec![clip("c1", "title", 0, 10)])],
    );

    let error = render_at(&tools, &project, &dir).expect_err("a missing font is a refusal");
    assert!(
        matches!(error, RenderError::UnusableFont { .. }),
        "got {error:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A file that is there but is not a font is the same refusal, from the parser
/// rather than the filesystem.
#[test]
fn a_font_file_that_is_not_a_font_stops_the_render() {
    let tools = tools();
    let dir = fixture_dir("text-not-a-font");
    std::fs::create_dir_all(dir.join("assets")).expect("create assets dir");
    std::fs::write(dir.join("assets/Fake.ttf"), b"not a font at all").expect("write the fake");
    let project = project(
        vec![text_asset_in("title", "assets/Fake.ttf")],
        vec![video_track("v1", vec![clip("c1", "title", 0, 10)])],
    );

    let error = render_at(&tools, &project, &dir).expect_err("not a font is a refusal");
    assert!(
        matches!(error, RenderError::UnusableFont { .. }),
        "got {error:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// An emoji survives the trip through the renderer's own text path, and the
/// channel that would say it did not stays empty.
///
/// The golden fixture pins what the fire looks like; only a pipeline test says
/// that the caption's colour does not reach it — white letters were asked for
/// and an orange frame comes back — and that a face which paints cleanly leaves
/// no note, so a note in a report means something when one appears.
#[test]
fn an_emoji_reaches_the_file_in_its_own_colours_and_notes_nothing() {
    let tools = tools();
    let dir = fixture_dir("text-emoji");
    let project = project(
        vec![titled("title", "🔥", Rgba::WHITE)],
        vec![video_track("v1", vec![clip("c1", "title", 0, 10)])],
    );

    let (out, report) = render_at(&tools, &project, &dir).expect("a text clip renders");

    assert!(
        report.notes.is_empty(),
        "everything Noto asks for can be painted: {:?}",
        report.notes
    );
    let (red, green, blue) = mean_rgb(&tools, &out, 5);
    assert!(
        red > blue + 4 && green > blue,
        "the fire keeps its own orange whatever colour the caption is; \
         found {red},{green},{blue}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
