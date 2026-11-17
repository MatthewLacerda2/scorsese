//! A sheet with a platform's interface over each frame (#1010).

use scorsese_core::style::Platform;
use scorsese_render::contact::{self, ContactError, Look};
use scorsese_render::{Overlay, OverlayError};

use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;
use crate::contact::banded;

#[test]
fn an_upright_file_is_looked_at_under_the_feed_and_a_wide_one_is_refused() {
    let dir = scratch::dir("contact-overlay");
    let tools = tools();
    let upright = dir.join("upright.mp4");
    let source = [
        "-f",
        "lavfi",
        "-i",
        "color=c=red:s=180x320:d=2:r=10",
        "-pix_fmt",
        "yuv420p",
    ];
    generate(&tools, &upright, &source);
    let tiktok = Overlay::new(Platform::Tiktok, false).expect("a feed");
    let look = |overlay| Look {
        count: 1,
        overlay,
        ..Look::default()
    };

    let plain = contact::sheet(&tools, &upright, &look(None)).expect("a sheet");
    let overlaid = contact::sheet(&tools, &upright, &look(Some(tiktok))).expect("a sheet");
    assert_eq!(plain.image.resolution(), overlaid.image.resolution());
    let red = |image: &scorsese_render::Frame| {
        let bytes = image.bytes();
        bytes
            .chunks_exact(4)
            .filter(|p| p[0] > 200 && p[1] < 60)
            .count()
    };
    assert!(
        red(&overlaid.image) < red(&plain.image) * 9 / 10,
        "the interface covers part of the shot"
    );

    let wide = banded(&tools, &dir);
    let refused = contact::sheet(&tools, &wide, &look(Some(tiktok))).expect_err("landscape");
    assert!(
        matches!(refused, ContactError::Overlay(OverlayError::NotUpright(_))),
        "{refused}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
