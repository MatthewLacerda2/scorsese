//! The size a page is captured at (#881): the raster's own, down to the
//! smallest device scale Chromium will launch at, and that size below it.
//!
//! What `page::request`'s doc and the html arm of `run::segment::layers`
//! claim, held to the browser itself, so a Chromium that lifts or moves the
//! floor fails here rather than leaving those comments quietly wrong. On
//! macOS a frame is a screenshot asked for at that floor (#844), and the page
//! must still be told the raster's own device scale, as it is on Linux.
//!
//! Needs the pinned browser, like `pages.rs` beside it.

use std::collections::BTreeMap;

use scorsese_core::Fps;
use scorsese_render::Resolution;
use scorsese_render::page::{self, Chrome, Request};

use crate::common::ffmpeg::{inspect, mean_rgb, tools};
use crate::common::scratch;

/// The size `raster`'s capture of a one-colour page comes out at. The page is
/// blue while the browser tells it the raster's own device scale, and magenta
/// otherwise — asked by a media query, so the browser answers as it draws.
fn captured_at(chrome: &Chrome, raster: Resolution) -> (u64, u64) {
    let tools = tools();
    let dir = scratch::dir(&format!(
        "capture-size-{}x{}",
        raster.width(),
        raster.height()
    ));
    std::fs::create_dir_all(dir.join("pages")).expect("pages/");
    let dppx = f64::from(raster.width().min(raster.height())) / 1080.0;
    let (low, high) = (dppx - 0.001, dppx + 0.001);
    std::fs::write(
        dir.join("pages/flat.html"),
        format!(
            "<style>div {{ position: absolute; inset: 0; background: #f0f }}\
             @media (min-resolution: {low}dppx) and (max-resolution: {high}dppx) \
             {{ div {{ background: #00f }} }}</style><div></div>"
        ),
    )
    .expect("a page");
    let request = Request {
        page: "pages/flat.html".into(),
        resolution: raster,
        fps: Fps::THIRTY,
        duration: 0.0,
        clips: BTreeMap::new(),
        words: BTreeMap::new(),
    };
    let captured = page::capture(chrome, &tools, &dir, &request).expect("captured");
    let file = inspect(&tools, &captured.file);
    let (red, _, blue) = mean_rgb(&tools, &captured.file, 0);
    assert!(
        red < 40 && blue > 200,
        "the page at {}x{} was told another device scale",
        raster.width(),
        raster.height()
    );
    std::fs::remove_dir_all(&dir).ok();
    (file.width, file.height)
}

#[test]
fn a_page_is_captured_at_the_raster_down_to_a_540_short_side_and_at_that_below() {
    let chrome = scorsese_render::testing::chrome();
    let raster = |width, height| Resolution::new(width, height).expect("a raster");
    assert_eq!(captured_at(&chrome, raster(1280, 720)), (1280, 720));
    assert_eq!(
        captured_at(&chrome, raster(960, 540)),
        (960, 540),
        "the floor"
    );
    assert_eq!(
        captured_at(&chrome, raster(160, 90)),
        (960, 540),
        "a golden's raster, under the floor"
    );
    assert_eq!(
        captured_at(&chrome, raster(270, 480)),
        (540, 960),
        "the short side is the one held, standing up too"
    );
}
