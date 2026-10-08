//! A Lottie played by a page (#903): lottie-web, shipped, driven from the clip's
//! clock rather than its own.
//!
//! `wave.json` is a hand-made Lottie — one box crossing its 512-unit square in
//! 60 frames at 30 fps — standing in for a file `stock_import` writes, since a
//! LottieFiles animation may not be redistributed on its own. Where the box
//! is in a frame says which of the animation's frames was drawn, so these hold
//! the page to the clock: frame N of the clip is the animation at N/30 s, with
//! nothing run in between that a still could skip.
//!
//! These need the pinned browser, as `pages.rs` does.

use scorsese_core::{Asset, AssetId, AssetKind, Fps, Frames, ProjectPath};
use scorsese_render::{Frame, RenderSettings, Renderer, Resolution};

use crate::common::ffmpeg::{fixture_dir, tools};
use crate::common::{clip, project, video_track};

/// The stand-in Lottie.
const WAVE: &str = include_str!("../fixtures/lottie/wave.json");

/// A page playing `wave.json` in a 720 px square in the middle of the frame,
/// one pass of it and then held — the shape `docs/pages.md` teaches, without
/// the loop and the recolouring.
const PAGE: &str = r#"<!doctype html>
<html><head>
<script src="https://lib.scorsese/lottie.min.js"></script>
<style>html, body { margin: 0; height: 100%; } body { display: grid; place-content: center; }
#wave { width: 720px; height: 720px; }</style>
</head><body><div id="wave"></div>
<script>
  const start = performance.now();
  fetch("wave.json").then((r) => r.json()).then((data) => {
    const anim = lottie.loadAnimation({ container: document.getElementById("wave"),
      renderer: "svg", loop: false, autoplay: false, animationData: data });
    const draw = (now) => {
      const at = (now - start) / 1000 * anim.frameRate;
      anim.goToAndStop(Math.min(at, anim.totalFrames - 1), true);
      requestAnimationFrame(draw);
    };
    requestAnimationFrame(draw);
  });
</script></body></html>"#;

/// The colour of one pixel of a frame.
fn pixel(frame: &Frame, x: u32, y: u32) -> (u8, u8, u8) {
    let at = ((y * frame.resolution().width() + x) * 4) as usize;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2])
}

/// Whether `colour` is the box's blue (0.2, 0.5, 0.9).
fn boxed(colour: (u8, u8, u8)) -> bool {
    let (r, g, b) = colour;
    r.abs_diff(51) < 24 && g.abs_diff(128) < 24 && b.abs_diff(230) < 24
}

/// Where the box's middle is at animation frame `f`, in the 320×180 still:
/// the 512-unit square is drawn 720 px wide from x = 600 of a 1920 px page,
/// and the box goes from 96 to 416 units across the 60 frames.
fn middle(f: f64) -> u32 {
    let units = 96.0 + 320.0 * f / 60.0;
    ((600.0 + units * 720.0 / 512.0) / 6.0).round() as u32
}

#[test]
fn a_lottie_is_drawn_at_the_frame_the_clock_says_and_its_library_is_recorded() {
    let tools = tools();
    let renderer = Renderer::new(
        &tools,
        RenderSettings::new(Resolution::new(320, 180).expect("a raster"), Fps::THIRTY),
    );
    let dir = fixture_dir("lottie");
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    std::fs::write(dir.join("pages/wave.json"), WAVE).expect("the Lottie");
    std::fs::write(dir.join("pages/wave.html"), PAGE).expect("the page");
    let page = Asset::imported(
        AssetId::new("wave"),
        AssetKind::Html,
        ProjectPath::new("pages/wave.html"),
    );
    let project = project(
        vec![page],
        vec![video_track("v1", vec![clip("c1", "wave", 0, 90)])],
    );

    for at in [15, 45] {
        let (frame, notes) = renderer
            .still_noted(&project, &dir, Frames(at))
            .expect("the page is drawn");
        assert!(notes.is_empty(), "frame {at}: {notes:?}");
        let here = middle(at as f64);
        assert!(
            boxed(pixel(&frame, here, 90)),
            "frame {at}: the box is at x = {here}, found {:?}",
            pixel(&frame, here, 90)
        );
        let elsewhere = middle(60.0 - at as f64);
        assert!(
            !boxed(pixel(&frame, elsewhere, 90)),
            "frame {at}: the box is not at x = {elsewhere} yet"
        );
    }

    // lottie-web is part of what the capture depends on, by its URL.
    let records: Vec<String> = walk(&dir.join("cache/pages"))
        .into_iter()
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect();
    assert!(
        records
            .iter()
            .any(|record| record.contains("\"https://lib.scorsese/lottie.min.js\"")),
        "a capture that loaded lottie-web records it"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Every file under `dir`.
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .flat_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                walk(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}
