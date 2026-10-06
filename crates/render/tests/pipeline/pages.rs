//! A page clip, captured by a headless browser (#775) — and the card it shows
//! when it cannot be, which is a stand-in and never a refusal.
//!
//! These capture real pages, so they need the pinned browser
//! (`SCORSESE_CHROME`, which `tools/chromium/fetch` prints) and fail without it,
//! as the golden renders do: a test that passed by drawing the card instead
//! would be passing by doing nothing.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, Project, ProjectPath};
use scorsese_render::page::Chrome;
use scorsese_render::{Frame, FrameRange, Note, Renderer};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};
use crate::{BLUE, RED, assert_colour, colour_asset, settings};

/// The colour of one pixel of a frame still in memory.
fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let at = ((y * frame.resolution().width() + x) * 4) as usize;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2])
}

/// A project of a red shot with `html` as a page over it, in `pages/title.html`.
fn over_red(name: &str, html: &str) -> (std::path::PathBuf, Project) {
    let tools = tools();
    let dir = fixture_dir(name);
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/title.html"), html).expect("a page");
    let page = Asset::imported(
        AssetId::new("title"),
        AssetKind::Html,
        ProjectPath::new("pages/title.html"),
    );
    let project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 1), page],
        vec![
            video_track("v1", vec![clip("c1", "red", 0, 20)]),
            video_track("v2", vec![clip("c2", "title", 0, 20)]),
        ],
    );
    (dir, project)
}

#[test]
fn a_page_draws_over_the_shot_with_the_shot_showing_where_it_draws_nothing() {
    let tools = tools();
    let html = "<div style='position:absolute;inset:0 0 50% 0;background:#00f'></div>\
                <img src='https://example.com/logo.png'>";
    let (dir, project) = over_red("page-drawn", html);
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY));

    let report = renderer
        .render(&project, &dir, FrameRange::ALL, &dir.join("out.mp4"))
        .expect("a page renders");
    let still = renderer
        .still(&project, &dir, Frames(10))
        .expect("a page clip composes");

    assert_colour(pixel(&still, 2, 2), BLUE, "the page's top half");
    let foot = pixel(&still, 2, still.resolution().height() - 2);
    assert_colour(foot, RED, "the shot, through the page's transparent half");
    assert_eq!(
        report.notes,
        [Note::PageWarning {
            asset: "title".into(),
            warning: "the page asked for https://example.com/logo.png from outside the \
                      project; pages render offline, so it rendered without it"
                .into(),
        }],
        "the internet is refused, and said"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[cfg(unix)]
#[test]
fn a_page_that_cannot_be_captured_is_a_band_and_a_note() {
    let tools = tools();
    let (dir, project) = over_red("page-card", "<h1>Hello</h1>");
    // A "browser" that exits the moment it starts: every capture fails.
    let gone = Chrome::at("true").expect("`true` runs");
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY)).with_chrome(gone);

    let report = renderer
        .render(&project, &dir, FrameRange::ALL, &dir.join("out.mp4"))
        .expect("a page that cannot be captured is not a failed render");
    let still = renderer
        .still(&project, &dir, Frames(10))
        .expect("a page clip composes");

    assert_eq!(report.frames, 20, "the render happened");
    assert!(
        matches!(&report.notes[..], [Note::PageNotCaptured { clip, .. }] if clip == "c2"),
        "{:?}",
        report.notes
    );
    assert_colour(pixel(&still, 2, 2), RED, "above the band");
    let foot = pixel(&still, 2, still.resolution().height() - 2);
    assert!(foot.0 < 120, "the band darkens the foot: {foot:?}");
    std::fs::remove_dir_all(&dir).ok();
}
