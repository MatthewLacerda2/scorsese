//! A picture for a model (#1435): a JPEG, its long edge at most
//! [`MODEL_EDGE`], never enlarged, and smaller than the PNG it came from.

use scorsese_core::ProbeMedia;
use scorsese_render::Ffprobe;
use scorsese_render::frames::{MODEL_EDGE, for_a_model};

use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;

/// `png` shrunk for a model: its bytes, and the width and height ffprobe reads.
fn shrunk(png: &std::path::Path) -> (Vec<u8>, (u32, u32)) {
    let tools = tools();
    let bytes = std::fs::read(png).expect("the fixture is written");
    let jpeg = for_a_model(&tools, &bytes).expect("the picture is shrunk");
    let out = png.with_extension("jpg");
    std::fs::write(&out, &jpeg).expect("the jpeg is written");
    let media = Ffprobe::new(tools).probe(&out).expect("the jpeg probes");
    let size = (media.width.unwrap_or(0), media.height.unwrap_or(0));
    (jpeg, size)
}

#[test]
fn a_large_picture_comes_back_a_jpeg_no_longer_than_the_model_reads() {
    let tools = tools();
    let dir = scratch::dir("shrunk-large");
    let png = dir.join("wide.png");
    let noise = [
        "-f",
        "lavfi",
        "-i",
        "testsrc2=s=3200x1800",
        "-frames:v",
        "1",
    ];
    generate(&tools, &png, &noise);
    let (jpeg, size) = shrunk(&png);
    assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF], "a JPEG's magic");
    assert_eq!(
        size,
        (MODEL_EDGE, 882),
        "the long edge capped, the aspect kept"
    );
    let before = std::fs::metadata(&png).unwrap().len();
    assert!(
        (jpeg.len() as u64) * 4 < before,
        "{} vs {before}",
        jpeg.len()
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_small_or_tall_picture_keeps_its_size() {
    let tools = tools();
    let dir = scratch::dir("shrunk-small");
    let cases = [
        ("small", "testsrc=s=320x240", (320, 240)),
        ("tall", "testsrc=s=720x2000", (564, MODEL_EDGE)),
    ];
    for (name, source, expected) in cases {
        let png = dir.join(format!("{name}.png"));
        generate(
            &tools,
            &png,
            &["-f", "lavfi", "-i", source, "-frames:v", "1"],
        );
        assert_eq!(shrunk(&png).1, expected, "{name}");
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn bytes_that_are_not_a_picture_are_refused() {
    assert!(for_a_model(&tools(), b"not a picture").is_err());
}
