//! A page clip before anything can capture one (#775): a card, never a refusal.
//!
//! A page is a layer drawn over the shot, so its card is the translucent band
//! across the foot of the frame rather than a panel over the whole of it — the
//! shot it was written to sit on stays visible above the band.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, ProjectPath};
use scorsese_render::{Frame, FrameRange, Renderer};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};
use crate::{RED, assert_colour, colour_asset, render, settings};

/// The colour of one pixel of a frame still in memory.
fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let at = ((y * frame.resolution().width() + x) * 4) as usize;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2])
}

fn page(id: &str) -> Asset {
    Asset::imported(
        AssetId::new(id),
        AssetKind::Html,
        ProjectPath::new(format!("pages/{id}.html")),
    )
}

#[test]
fn a_page_over_a_shot_is_a_band_and_the_shot_shows_above_it() {
    let tools = tools();
    let dir = fixture_dir("page-card");
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/title.html"), "<h1>Hello</h1>").expect("a page");
    let project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 1), page("title")],
        vec![
            video_track("v1", vec![clip("c1", "red", 0, 20)]),
            video_track("v2", vec![clip("c2", "title", 0, 20)]),
        ],
    );

    let (_, report) = render(&tools, &project, &dir, FrameRange::ALL, Fps::THIRTY);
    let still = Renderer::new(&tools, settings(Fps::THIRTY))
        .still(&project, &dir, Frames(10))
        .expect("a page clip composes");

    assert_eq!(report.frames, 20, "the render happened");
    assert!(
        report.notes.is_empty(),
        "an uncaptured page is not a fault: {:?}",
        report.notes
    );
    assert_colour(pixel(&still, 2, 2), RED, "above the band");
    let foot = pixel(&still, 2, still.resolution().height() - 2);
    assert!(foot.0 < 120, "the band darkens the foot: {foot:?}");
    std::fs::remove_dir_all(&dir).ok();
}
