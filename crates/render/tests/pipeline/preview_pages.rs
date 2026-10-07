//! A window's preview and a page (#776): drawn from the cache only, as its
//! card until a capture made elsewhere lands, and never waiting on a browser.
//!
//! Needs the pinned browser, like `pages.rs` beside it.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, ProjectPath};
use scorsese_render::page::{self, Chrome};
use scorsese_render::{Frame, Renderer};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};
use crate::{BLUE, RED, assert_colour, colour_asset, settings};

fn top_left(frame: &Frame) -> (u8, u8, u8) {
    let bytes = frame.bytes();
    (bytes[0], bytes[1], bytes[2])
}

/// How many whole captures are under `dir`.
fn captures(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .map(|path| {
            if path.is_dir() {
                captures(&path)
            } else {
                usize::from(path.ends_with("frames.mkv"))
            }
        })
        .sum()
}

#[test]
fn a_preview_shows_the_card_until_the_capture_lands_and_then_the_page() {
    let tools = tools();
    let dir = fixture_dir("preview-pages");
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(
        dir.join("pages/title.html"),
        "<div style='position:absolute;inset:0;background:#00f'></div>",
    )
    .expect("a page");
    let page = Asset::imported(
        AssetId::new("title"),
        AssetKind::Html,
        ProjectPath::new("pages/title.html"),
    );
    let project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 1), page],
        vec![
            video_track("v1", vec![clip("c1", "red", 0, 20)]),
            // The same page twice: told the clips from two places, and one
            // capture serves both, since it reads none of them.
            video_track(
                "v2",
                vec![clip("c2", "title", 0, 10), clip("c3", "title", 10, 10)],
            ),
        ],
    );
    let chrome = Chrome::discover().expect("the pinned browser (SCORSESE_CHROME)");
    let preview = Renderer::new(&tools, settings(Fps::THIRTY)).without_capturing();

    let blind = preview.still(&project, &dir, Frames(5)).expect("composes");
    assert_colour(
        top_left(&blind),
        RED,
        "no browser given: the card, over the shot",
    );
    let preview = preview.with_chrome(chrome.clone());
    let waiting = preview.still(&project, &dir, Frames(5)).expect("composes");
    assert_colour(
        top_left(&waiting),
        RED,
        "nothing captured yet: still the card",
    );
    assert!(
        !dir.join("cache/pages").exists(),
        "and nothing was captured for it"
    );

    let requests = preview.page_requests(&project, &dir).expect("plans");
    assert_eq!(requests.len(), 2, "one clock, told from two places");
    for request in &requests {
        page::capture(&chrome, &tools, &dir, request).expect("captured");
    }
    assert_eq!(
        captures(&dir.join("cache/pages")),
        1,
        "one capture for both"
    );
    let landed = preview.still(&project, &dir, Frames(5)).expect("composes");
    assert_colour(top_left(&landed), BLUE, "the capture, read from the cache");
    std::fs::remove_dir_all(&dir).ok();
}
