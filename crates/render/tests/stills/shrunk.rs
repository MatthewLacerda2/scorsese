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

#[test]
fn transparency_is_laid_over_white_and_what_is_opaque_keeps_its_colour() {
    let tools = tools();
    let dir = scratch::dir("shrunk-alpha");
    let png = dir.join("lottie.png");
    let clear_with_a_blue_square = "color=c=red@0.0:s=64x64,format=rgba[clear];\
         color=c=blue:s=32x32,format=rgba[square];[clear][square]overlay=16:16";
    let source = [
        "-f",
        "lavfi",
        "-i",
        clear_with_a_blue_square,
        "-frames:v",
        "1",
    ];
    generate(&tools, &png, &source);
    let (jpeg, _) = shrunk(&png);
    let out = dir.join("lottie.jpg");
    std::fs::write(&out, jpeg).expect("the jpeg is written");
    let pixel = |x: u32, y: u32| {
        let decoded = tools
            .ffmpeg()
            .args(["-nostdin", "-v", "error", "-i"])
            .arg(&out)
            .args(["-vf", &format!("crop=2:2:{x}:{y}")])
            .args(["-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
            .output()
            .expect("ffmpeg runs");
        decoded.stdout[..3].to_vec()
    };
    let corner = pixel(0, 0);
    assert!(
        corner.iter().all(|&c| c > 240),
        "white, not black: {corner:?}"
    );
    let middle = pixel(32, 32);
    assert!(middle[2] > 200 && middle[0] < 60, "still blue: {middle:?}");
    std::fs::remove_dir_all(dir).ok();
}
