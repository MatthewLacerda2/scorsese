//! The motion kit (#812), and a file of the project shared between pages.
//!
//! A narrow box slides across the frame at 480 page px a second, placed by
//! `kit.frame`'s seconds, and fades with `kit.exit` on the clip's last 0.3 s.
//! Where the box is says what time the kit handed the page: the clock reads
//! 100 ms at the page's time zero (#917), and a kit that did not take that off
//! would draw the box 48 px further on, which is off the pixel looked at. Its
//! colour comes from the project's `pages/lib.js`, so editing that file must
//! draw the page again.
//!
//! These need the pinned browser, as `pages.rs` does.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, ProjectPath};
use scorsese_render::{Frame, RenderSettings, Renderer, Resolution};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};

/// The page: a 48 px box, its left edge at `t * 480` page px.
const PAGE: &str = r#"<!doctype html>
<html><head>
<script src="https://lib.scorsese/kit.js"></script>
<script src="lib.js"></script>
<style>html, body { margin: 0; height: 100%; }
#box { position: absolute; top: 0; height: 100%; width: 48px; }</style>
</head><body><div id="box"></div>
<script>
  const box = document.getElementById("box");
  box.style.background = colour();
  kit.frame((t) => {
    box.style.left = t * 480 + "px";
    box.style.opacity = kit.exit(t);
  });
</script></body></html>"#;

/// The shared file, saying which colour the box is.
fn lib(colour: &str) -> String {
    format!("function colour() {{ return \"{colour}\"; }}\n")
}

/// The colour of one pixel of a frame.
fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let at = ((y * frame.resolution().width() + x) * 4) as usize;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2])
}

/// Where the box's middle is at frame `f`, in the 320×180 still, which is a
/// sixth of the 1920 px page.
fn middle(f: u32) -> u32 {
    let left = f64::from(f) / 30.0 * 480.0;
    ((left + 24.0) / 6.0).round() as u32
}

#[test]
fn the_kit_hands_a_page_its_own_seconds_and_a_shared_file_redraws_its_pages() {
    let tools = tools();
    let renderer = Renderer::new(
        &tools,
        RenderSettings::new(Resolution::new(320, 180).expect("a raster"), Fps::THIRTY),
    );
    let dir = fixture_dir("kit");
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/slide.html"), PAGE).expect("the page");
    std::fs::write(dir.join("pages/lib.js"), lib("#0000ff")).expect("the shared file");
    let page = Asset::imported(
        AssetId::new("slide"),
        AssetKind::Html,
        ProjectPath::new("pages/slide.html"),
    );
    let project = project(
        vec![page],
        vec![video_track("v1", vec![clip("c1", "slide", 0, 90)])],
    );
    let still = |at: u32| {
        let (frame, notes) = renderer
            .still_noted(&project, &dir, Frames(u64::from(at)))
            .expect("the page is drawn");
        assert!(notes.is_empty(), "frame {at}: {notes:?}");
        frame
    };
    let blue = |(r, g, b): (u8, u8, u8)| r < 60 && g < 60 && b > 190;
    let red = |(r, g, b): (u8, u8, u8)| r > 190 && g < 60 && b < 60;

    for at in [15, 45, 75] {
        let frame = still(at);
        let here = middle(at);
        assert!(
            blue(pixel(&frame, here, 90)),
            "frame {at}: the box is at x = {here}, found {:?}",
            pixel(&frame, here, 90)
        );
        assert!(
            !blue(pixel(&frame, here + 8, 90)),
            "frame {at}: the box is not 100 ms further on"
        );
    }
    let last = still(89);
    assert!(
        !blue(pixel(&last, middle(89), 90)),
        "kit.exit has faded the box out by the clip's last frame"
    );

    std::fs::write(dir.join("pages/lib.js"), lib("#ff0000")).expect("the edit");
    let edited = still(15);
    assert!(
        red(pixel(&edited, middle(15), 90)),
        "editing the shared file draws the page again, found {:?}",
        pixel(&edited, middle(15), 90)
    );
    std::fs::remove_dir_all(&dir).ok();
}
