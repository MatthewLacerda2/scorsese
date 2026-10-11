//! The motion kit (#812): its clock, its easings and its count-up, each
//! checked in the pinned browser that draws pages, as `pages.rs` is.
//!
//! The easing and count checks run in the page itself. A page throws, naming
//! what it found, when the kit answers something else, and a script that threw
//! comes back as a note; it paints itself green once every check has run, so
//! a page that never got that far cannot pass by saying nothing.

mod clock;
mod count;
mod easings;
mod font;

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, ProjectPath};
use scorsese_render::{RenderSettings, Renderer, Resolution};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};

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
    let tools = tools();
    let renderer = Renderer::new(
        &tools,
        RenderSettings::new(Resolution::new(64, 36).expect("a raster"), Fps::THIRTY),
    );
    let dir = fixture_dir(label);
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/checks.html"), page).expect("the page");
    let asset = Asset::imported(
        AssetId::new("checks"),
        AssetKind::Html,
        ProjectPath::new("pages/checks.html"),
    );
    let project = project(
        vec![asset],
        vec![video_track("v1", vec![clip("c1", "checks", 0, 30)])],
    );
    let (frame, notes) = renderer
        .still_noted(&project, &dir, Frames(15))
        .expect("the page is drawn");
    assert!(notes.is_empty(), "{notes:?}");
    let [r, g, b] = [0, 1, 2].map(|i| frame.bytes()[(18 * 64 + 32) * 4 + i]);
    assert!(
        r < 60 && g > 190 && b < 60,
        "the checks ran to their end, found {:?}",
        (r, g, b)
    );
    std::fs::remove_dir_all(&dir).ok();
}
