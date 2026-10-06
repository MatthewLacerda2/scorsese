//! A page's layout mistakes, heard as notes (#813): text off the frame, inside
//! the safe margin, out of its box, or over other text — and nothing for an
//! entrance passing through, or text nobody can see.
//!
//! These need the pinned browser, as `pages.rs` does.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, ProjectPath};
use scorsese_render::{Note, Renderer};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};
use crate::settings;

/// Every mistake the measuring knows, held still, beside the things that look
/// like mistakes and are not. At 64 × 64 the page lays out in 1080 × 1080.
const PAGE: &str = r##"<style>
  body { margin: 0; font: 40px Inter; color: #fff; }
  p { position: absolute; margin: 0; white-space: nowrap; }
  /* Off the left at 0 s, moving every sample until it lands at 1 s. */
  @keyframes in { from { transform: translateX(-300px); } to { transform: none; } }
</style>
<div style="position:absolute;inset:0;background:#123"></div>
<div style="position:absolute;left:300px;top:200px;width:200px;height:60px;background:#345;display:flex;justify-content:center">
  <span style="white-space:nowrap">a caption far too long for its card</span>
</div>
<p style="left:20px;top:400px">hugging the edge</p>
<p style="left:900px;top:500px">running off</p>
<p style="left:300px;top:700px">first</p><p style="left:330px;top:705px">second</p>
<p style="left:300px;top:900px;opacity:0">invisible at -500</p>
<p style="left:200px;top:800px;animation:in 1s linear both">sliding in</p>
<svg width="1080" height="1080" style="position:absolute;inset:0">
  <text x="1060" y="300" font-size="40" fill="#fff">svg label</text>
</svg>"##;

#[test]
fn held_layout_mistakes_are_notes_and_passing_motion_is_not() {
    let tools = tools();
    let dir = fixture_dir("page-layout");
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/layout.html"), PAGE).expect("a page");
    let page = Asset::imported(
        AssetId::new("layout"),
        AssetKind::Html,
        ProjectPath::new("pages/layout.html"),
    );
    let project = project(
        vec![page],
        vec![video_track("v1", vec![clip("c1", "layout", 0, 30)])],
    );
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY));

    let (_, notes) = renderer
        .still_noted(&project, &dir, Frames(5))
        .expect("a page clip composes");
    let said: Vec<&str> = notes
        .iter()
        .map(|note| match note {
            Note::PageWarning { warning, .. } => warning.as_str(),
            other => panic!("only page warnings: {other:?}"),
        })
        .collect();

    let expected = [
        r#""a caption far too long for its card" overflows its box <div> on the left"#,
        r#""a caption far too long for its card" overflows its box <div> on the right"#,
        r#""hugging the edge" is inside the safe margin at the left: 20 px"#,
        r#""running off" runs off the right of the frame"#,
        r#""first" and "second" overlap"#,
        r#""svg label" runs off the right of the frame"#,
    ];
    for want in expected {
        assert!(
            said.iter().any(|note| note.contains(want)),
            "{want}: {said:#?}"
        );
    }
    assert_eq!(said.len(), expected.len(), "nothing else: {said:#?}");
    assert!(
        said.iter()
            .all(|note| note.ends_with("(from 0s into the page)")),
        "{said:#?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
