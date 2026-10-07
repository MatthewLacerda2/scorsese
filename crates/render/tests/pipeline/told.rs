//! A page timed to the edit by `scorsese.clips` (#810): told where a clip is in
//! its own seconds, captured again when that clip moves, and only then.
//!
//! Needs the pinned browser, like `pages.rs` beside it.

use scorsese_core::{Asset, AssetId, AssetKind, ClipId, Fps, Frames, Project, ProjectPath};
use scorsese_render::page::Chrome;
use scorsese_render::{Frame, Renderer};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};
use crate::{BLUE, RED, assert_colour, colour_asset, settings};

/// Blue over the whole frame for exactly as long as the clip `cue` lasts.
const PAGE: &str = "<div id='lit' style='position:absolute;inset:0'></div>\
    <style>@keyframes on { from, to { background: #00f } }</style>\
    <script>const cue = scorsese.clips.cue;\
    lit.style.animation = `on ${cue.end - cue.start}s linear ${cue.start}s`;</script>";

fn top_left(frame: &Frame) -> (u8, u8, u8) {
    let bytes = frame.bytes();
    (bytes[0], bytes[1], bytes[2])
}

fn moved(project: &mut Project, id: &str, start: u64) {
    let clip = project
        .tracks
        .iter_mut()
        .flat_map(|track| &mut track.clips)
        .find(|clip| clip.id == ClipId::new(id))
        .expect("the clip");
    clip.start = Frames(start);
}

#[test]
fn a_page_follows_the_clip_it_reads_and_ignores_the_ones_it_does_not() {
    let tools = tools();
    let dir = fixture_dir("told-clips");
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/stack.html"), PAGE).expect("a page");
    let page = Asset::imported(
        AssetId::new("stack"),
        AssetKind::Html,
        ProjectPath::new("pages/stack.html"),
    );
    // The page's clip opens 3 frames into it at timeline frame 6, so the cue
    // at frame 10 is 7 frames into the page's clock — and lit on screen from
    // timeline frame 10, wherever the page's clip sits.
    let mut shown = clip("page", "stack", 6, 14);
    shown.source_in = Frames(3);
    let mut project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 1), page],
        vec![
            // Under the shot: placed, never seen.
            video_track(
                "under",
                vec![clip("cue", "red", 10, 5), clip("aside", "red", 16, 2)],
            ),
            video_track("v1", vec![clip("shot", "red", 0, 20)]),
            video_track("v2", vec![shown]),
        ],
    );
    let chrome = Chrome::discover().expect("the pinned browser (SCORSESE_CHROME)");
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY)).with_chrome(chrome.clone());
    let preview = Renderer::new(&tools, settings(Fps::THIRTY))
        .with_chrome(chrome)
        .without_capturing();
    let at = |renderer: &Renderer, project: &Project, frame: u64| {
        let still = renderer
            .still(project, &dir, Frames(frame))
            .expect("composes");
        top_left(&still)
    };

    assert_colour(at(&renderer, &project, 12), BLUE, "while the cue plays");
    assert_colour(at(&renderer, &project, 8), RED, "before it");
    assert_colour(at(&renderer, &project, 17), RED, "after it");

    moved(&mut project, "aside", 18);
    assert_colour(
        at(&preview, &project, 12),
        BLUE,
        "a clip the page never read moved: the capture still holds",
    );

    moved(&mut project, "cue", 14);
    assert_colour(
        at(&preview, &project, 12),
        RED,
        "the clip it read moved: not captured yet, so the card over the shot",
    );
    assert_colour(at(&renderer, &project, 12), RED, "captured again: not yet");
    assert_colour(
        at(&renderer, &project, 15),
        BLUE,
        "the cue, where it is now",
    );
    std::fs::remove_dir_all(&dir).ok();
}
