//! The motion kit (#812): its clock, its easings, its count-up, its text
//! reveal and its path helpers, each checked in the pinned browser that draws
//! pages, as `pages.rs` is.
//!
//! Most checks run in the page itself. A page throws, naming
//! what it found, when the kit answers something else, and a script that threw
//! comes back as a note; it paints itself green once every check has run, so
//! a page that never got that far cannot pass by saying nothing.

mod clock;
mod count;
mod dashes;
mod easings;
mod font;
mod paths;
mod reveal;

use scorsese_core::{Asset, AssetId, AssetKind, Clip, Fps, Frames, ProjectPath};
use scorsese_render::{Frame, RenderSettings, Renderer, Resolution};

use crate::common::ffmpeg::tools;
use crate::common::scratch;
use crate::common::{project, video_track};

/// Draws a page whose `<script>` is `checks` (with the kit loaded) and asserts
/// that nothing it ran threw and that it reached its end.
fn passes(label: &str, checks: &str) {
    let page = format!(
        r##"<!doctype html>
<html><head>
<script src="https://lib.scorsese/kit.js"></script>
<style>html, body {{ margin: 0; height: 100%; background: #f00; }}</style>
</head><body><script>
kit.frame(() => {{
  if (document.body.dataset.checked) return;
  {checks}
  document.body.dataset.checked = "yes";
  document.body.style.background = "#0f0";
}});
</script></body></html>"##
    );
    let frame = drawn(label, &page, (64, 36), 15);
    let [r, g, b] = [0, 1, 2].map(|i| frame.bytes()[(18 * 64 + 32) * 4 + i]);
    assert!(
        r < 60 && g > 190 && b < 60,
        "the checks ran to their end, found {:?}",
        (r, g, b)
    );
}

/// `page` drawn at `size` on its `frame`th frame, at 30 fps, with nothing
/// noted about it.
fn drawn(label: &str, page: &str, (width, height): (u32, u32), frame: u64) -> Frame {
    let tools = tools();
    let renderer = Renderer::new(
        &tools,
        RenderSettings::new(
            Resolution::new(width, height).expect("a raster"),
            Fps::THIRTY,
        ),
    );
    let dir = scratch::dir(label);
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/checks.html"), page).expect("the page");
    let asset = Asset::imported(
        AssetId::new("checks"),
        AssetKind::Html,
        ProjectPath::new("pages/checks.html"),
    );
    let project = project(
        vec![asset],
        vec![video_track("v1", vec![Clip::at("c1", "checks", 0, 30)])],
    );
    let (drawn, notes) = renderer
        .still_noted(&project, &dir, Frames(frame))
        .expect("the page is drawn");
    assert!(notes.is_empty(), "{notes:?}");
    std::fs::remove_dir_all(&dir).ok();
    drawn
}
