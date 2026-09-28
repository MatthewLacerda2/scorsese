//! Making a proxy: smaller, the same frames at the same times, and never
//! half-written.

use scorsese_render::preview::{self, PROXY_SHORT_SIDE};

use super::common::ffmpeg::{fixture_dir, generate, inspect, tools};

#[test]
fn a_proxy_brings_the_short_side_down_and_keeps_every_frame() {
    let tools = tools();
    let dir = fixture_dir("proxy-make");
    for (size, expected) in [("1280x720", (960, 540)), ("720x1280", (540, 960))] {
        let source = dir.join(format!("{size}.mp4"));
        generate(
            &tools,
            &source,
            &[
                "-f",
                "lavfi",
                "-i",
                &format!("testsrc=s={size}:d=0.5:r=30"),
                "-pix_fmt",
                "yuv420p",
            ],
        );
        let out = dir.join("proxies").join(preview::file_name(size));
        preview::make(&tools, &source, &out).expect("the proxy is made");

        let (original, proxy) = (inspect(&tools, &source), inspect(&tools, &out));
        assert_eq!((proxy.width, proxy.height), expected, "{size}");
        assert_eq!(proxy.width.min(proxy.height), u64::from(PROXY_SHORT_SIDE));
        assert_eq!(proxy.frames, original.frames, "{size}: every frame kept");
        assert_eq!(proxy.rate, original.rate, "{size}: the same timing");
    }
    let leftovers: Vec<_> = std::fs::read_dir(dir.join("proxies"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().contains("making"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "nothing half-made left: {leftovers:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_source_ffmpeg_cannot_read_is_refused_and_leaves_nothing() {
    let tools = tools();
    let dir = fixture_dir("proxy-refused");
    let source = dir.join("not-a-video.mp4");
    std::fs::write(&source, b"nothing a decoder recognises").unwrap();
    let out = dir.join(preview::file_name("x"));
    let error = preview::make(&tools, &source, &out).expect_err("nothing to transcode");
    assert!(error.to_string().contains("not-a-video.mp4"), "{error}");
    assert!(
        std::fs::read_dir(&dir).unwrap().count() == 1,
        "only the source is there"
    );
    std::fs::remove_dir_all(&dir).ok();
}
