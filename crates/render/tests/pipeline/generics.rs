//! A page's CSS generic families draw in the faces they mean (#1030):
//! `sans-serif` and `system-ui` in Inter, `serif` in Source Serif 4 and
//! `monospace` in JetBrains Mono — never fontconfig's first shipped face or
//! Chrome's metric twins for Arial and Times New Roman.
//!
//! Needs the pinned browser, like `pages.rs` beside it.

use scorsese_core::{Asset, AssetId, AssetKind, Clip, Fps, Frames, ProjectPath};
use scorsese_render::page::Chrome;
use scorsese_render::{Frame, Renderer};

use crate::common::ffmpeg::tools;
use crate::common::scratch;
use crate::common::{project, video_track};
use crate::{BLUE, assert_colour, colour_asset, settings};

/// Four strips, each lit blue when its generic measures as the face it means.
/// Inter and Source Serif 4 are measured by the names their files give
/// themselves, told apart from the browser's default first so a name it could
/// not find never passes for one it did. `monospace` is held to fixed pitch:
/// `JetBrains Mono` is also a web font the page is given, still loading while
/// this script runs, so it cannot be measured by name here.
const PAGE: &str = "<style>i { position:absolute; top:0; bottom:0; width:25% }</style>\
    <i id=sans style='left:0'></i><i id=serif style='left:25%'></i>\
    <i id=mono style='left:50%'></i><i id=ui style='left:75%'></i>\
    <script>\
    const c = document.createElement('canvas').getContext('2d');\
    const w = (family, text = 'Hamburgefonstiv 0123') =>\
      (c.font = `40px ${family}`, c.measureText(text).width);\
    const [inter, source, none] = ['\"Inter V\"', '\"Source Serif 4 Variable\"', 'nonesuch'].map(f => w(f));\
    const apart = new Set([inter, source, none]).size === 3;\
    const lit = (id, ok) => { if (apart && ok) document.getElementById(id).style.background = '#00f'; };\
    lit('sans', w('sans-serif') === inter);\
    lit('serif', w('serif') === source);\
    lit('mono', w('monospace') !== none && w('monospace', 'iiii') === w('monospace', 'WWWW'));\
    lit('ui', w('system-ui') === inter);\
    </script>";

fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let at = ((y * frame.resolution().width() + x) * 4) as usize;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2])
}

#[test]
fn a_page_s_generic_families_draw_in_the_faces_they_mean() {
    let tools = tools();
    let dir = scratch::dir("page-generics");
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/generics.html"), PAGE).expect("a page");
    let page = Asset::imported(
        AssetId::new("generics"),
        AssetKind::Html,
        ProjectPath::new("pages/generics.html"),
    );
    let project = project(
        vec![colour_asset(&tools, &dir, "red", "64x64", 1), page],
        vec![
            video_track("v1", vec![Clip::at("c1", "red", 0, 10)]),
            video_track("v2", vec![Clip::at("c2", "generics", 0, 10)]),
        ],
    );
    let chrome = Chrome::discover().expect("the pinned browser (SCORSESE_CHROME)");
    let renderer = Renderer::new(&tools, settings(Fps::THIRTY)).with_chrome(chrome);
    let still = renderer.still(&project, &dir, Frames(5)).expect("composes");

    let quarter = still.resolution().width() / 4;
    for (strip, generic) in ["sans-serif", "serif", "monospace", "system-ui"]
        .into_iter()
        .enumerate()
    {
        let x = quarter * strip as u32 + quarter / 2;
        assert_colour(pixel(&still, x, 2), BLUE, generic);
    }
    std::fs::remove_dir_all(&dir).ok();
}
