//! `docs/pages.md`'s worked pages, captured: the guide cannot rot.
//!
//! Every ```` ```html page <name> ```` block in the guide is written into a
//! project and drawn at a point where its entrances have finished. Each must be
//! captured, warn about nothing — no refused request, no missing file, no
//! script that threw — and draw something. A page that broke would otherwise
//! go on teaching every agent that reads the guide, with nothing to say so.
//!
//! These need the pinned browser, as `pages.rs` does.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, ProjectPath};
use scorsese_render::{Frame, RenderSettings, Renderer, Resolution};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};

/// The guide, found from this crate rather than from the working directory.
fn guide() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/pages.md");
    std::fs::read_to_string(path).expect("docs/pages.md is in the repo")
}

/// Every worked page in `text`: its name, and its HTML.
fn worked(text: &str) -> Vec<(String, String)> {
    let mut pages = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let Some(name) = line.strip_prefix("```html page ") else {
            continue;
        };
        let html: Vec<&str> = lines.by_ref().take_while(|line| *line != "```").collect();
        pages.push((name.trim().to_owned(), html.join("\n")));
    }
    pages
}

/// Whether any pixel differs from the first: a page that drew nothing over
/// nothing is one flat colour.
fn drew_something(frame: &Frame) -> bool {
    let bytes = frame.bytes();
    bytes.chunks_exact(4).any(|pixel| pixel != &bytes[..4])
}

#[test]
fn the_guide_has_its_six_worked_pages() {
    let names: Vec<String> = worked(&guide()).into_iter().map(|(name, _)| name).collect();
    assert_eq!(
        names,
        [
            "title-card",
            "lower-third",
            "stat-counter",
            "flowchart",
            "icons",
            "in-step"
        ],
        "the guide's worked pages, in order"
    );
}

#[test]
fn every_worked_page_captures_without_a_warning_and_draws() {
    let tools = tools();
    let resolution = Resolution::new(320, 180).expect("a legal raster");
    let renderer = Renderer::new(&tools, RenderSettings::new(resolution, Fps::THIRTY));
    for (name, html) in worked(&guide()) {
        let dir = fixture_dir(&format!("guide-{name}"));
        std::fs::create_dir_all(dir.join("pages")).expect("pages/");
        let path = format!("pages/{name}.html");
        std::fs::write(dir.join(&path), &html).expect("the page");
        let page = Asset::imported(
            AssetId::new(&name),
            AssetKind::Html,
            ProjectPath::new(&path),
        );
        // Four seconds, looked at past three: every entrance in the guide has
        // finished by then, and no exit timed to `duration` has begun.
        let project = project(
            vec![page],
            vec![video_track("v1", vec![clip("c1", &name, 0, 120)])],
        );

        let (frame, notes) = renderer
            .still_noted(&project, &dir, Frames(100))
            .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert!(notes.is_empty(), "{name} was not drawn cleanly: {notes:?}");
        assert!(drew_something(&frame), "{name} drew nothing");
        std::fs::remove_dir_all(&dir).ok();
    }
}
