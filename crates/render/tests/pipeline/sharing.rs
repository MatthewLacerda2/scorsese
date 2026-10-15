//! Page captures shared inside one render and between two at once (#1085):
//! a capture never deletes the file another clip, or another render, was
//! handed, and two captures never write into one file.
//!
//! These capture real pages, so they need the pinned browser
//! (`SCORSESE_CHROME`), as `pages`' do.

use std::path::{Path, PathBuf};

use scorsese_core::{Asset, AssetId, AssetKind, Clip, Fps, Project, ProjectPath};
use scorsese_render::page::Chrome;
use scorsese_render::{FrameRange, Note, RenderReport, Renderer};

use crate::common::ffmpeg::{mean_rgb, tools};
use crate::common::scratch;
use crate::common::{clip_from, project, video_track};
use crate::{BLUE, assert_colour, colour_asset, settings};

/// A red shot under a blue page, shown twice: first entering the page ten
/// frames in, then from its start — two clips with the same clock, so the
/// first is a piece of the slot the second captures whole.
fn twice(name: &str) -> (PathBuf, Project) {
    let tools = tools();
    let dir = scratch::dir(name);
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    let html = "<div style='position:absolute;inset:0;background:#00f'></div>";
    std::fs::write(dir.join("pages/blue.html"), html).expect("a page");
    let page = Asset::imported(
        AssetId::new("blue"),
        AssetKind::Html,
        ProjectPath::new("pages/blue.html"),
    );
    let project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 1), page],
        vec![
            video_track("v1", vec![Clip::at("under", "red", 0, 30)]),
            video_track(
                "v2",
                vec![
                    clip_from("trimmed", "blue", 0, 10, 10),
                    Clip::at("whole", "blue", 10, 20),
                ],
            ),
        ],
    );
    (dir, project)
}

/// Renders all of `project` to `out`, with the pinned browser.
fn render(project: &Project, dir: &Path, out: &str) -> RenderReport {
    let tools = tools();
    let chrome = Chrome::discover().expect("the pinned browser (SCORSESE_CHROME)");
    Renderer::new(&tools, settings(Fps::THIRTY))
        .with_chrome(chrome)
        .render(project, dir, FrameRange::ALL, &dir.join(out))
        .expect("pages shared between clips render")
}

/// Every captured file left in the project's cache, by name.
fn left(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for slot in std::fs::read_dir(dir.join("cache/pages"))
        .expect("captured")
        .flatten()
    {
        for shelf in std::fs::read_dir(slot.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            for file in std::fs::read_dir(shelf.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                names.push(file.file_name().to_string_lossy().into_owned());
            }
        }
    }
    names.retain(|name| name.ends_with(".mkv") || name.ends_with(".json"));
    names.sort();
    names
}

/// The page's clips both captured, with nothing said about them.
fn both_shown(report: &RenderReport) {
    assert_eq!(report.frames, 30);
    assert!(
        !report
            .notes
            .iter()
            .any(|note| matches!(note, Note::PageNotCaptured { .. })),
        "{:?}",
        report.notes
    );
}

#[test]
fn a_clip_s_piece_survives_another_clip_capturing_the_whole_page() {
    let (dir, project) = twice("page-shared");
    let report = render(&project, &dir, "out.mp4");
    both_shown(&report);
    for at in [5, 20] {
        let shown = mean_rgb(&tools(), &dir.join("out.mp4"), at);
        assert_colour(shown, BLUE, &format!("the page at {at}"));
    }
    assert_eq!(
        left(&dir),
        ["capture.json", "frames.mkv"],
        "the piece is tidied once the render is over"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn two_renders_of_one_project_capture_on_two_threads() {
    let (dir, project) = twice("page-threads");
    let reports: Vec<RenderReport> = std::thread::scope(|scope| {
        let renders: Vec<_> = ["a.mp4", "b.mp4"]
            .map(|out| scope.spawn(|| render(&project, &dir, out)))
            .into_iter()
            .collect();
        renders
            .into_iter()
            .map(|r| r.join().expect("renders"))
            .collect()
    });
    for report in &reports {
        both_shown(report);
    }
    for out in ["a.mp4", "b.mp4"] {
        assert_colour(mean_rgb(&tools(), &dir.join(out), 5), BLUE, out);
    }
    let partials: Vec<String> = std::fs::read_dir(dir.join("cache/pages"))
        .expect("captured")
        .flatten()
        .flat_map(|slot| {
            std::fs::read_dir(slot.path())
                .into_iter()
                .flatten()
                .flatten()
        })
        .map(|file| file.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("partial"))
        .collect();
    assert_eq!(
        partials,
        Vec::<String>::new(),
        "nothing half-written is left"
    );
    std::fs::remove_dir_all(&dir).ok();
}
