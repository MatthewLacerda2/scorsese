//! A sheet of several pictures, one from each file (#900).

use scorsese_render::contact;

use crate::common::ffmpeg::{fixture_dir, generate, tools};

/// A `size` JPEG of one colour.
fn picture(
    tools: &scorsese_render::Tools,
    dir: &std::path::Path,
    name: &str,
    colour: &str,
    size: &str,
) -> std::path::PathBuf {
    let file = dir.join(format!("{name}.jpg"));
    let source = format!("color=c={colour}:s={size}:d=1");
    generate(
        tools,
        &file,
        &["-f", "lavfi", "-i", &source, "-frames:v", "1"],
    );
    file
}

/// Pictures of different shapes share one cell size, each letterboxed rather
/// than cropped, in the order given.
#[test]
fn pictures_of_mixed_shapes_tile_letterboxed_in_order() {
    let dir = fixture_dir("contact-pictures");
    let tools = tools();
    let wide = picture(&tools, &dir, "wide", "red", "160x90");
    let tall = picture(&tools, &dir, "tall", "blue", "90x160");
    let sheet = contact::pictures(
        &tools,
        &[
            (wide, String::from("1. wide")),
            (tall, String::from("2. tall")),
        ],
        false,
    )
    .expect("a sheet of two pictures");
    let (width, height) = (sheet.resolution().width(), sheet.resolution().height());
    // Two 480x270 cells, each with 34 rows of label under it (#919).
    assert_eq!((width, height), (960, 270 + 34));

    let at = |x: u32, y: u32| {
        let index = ((y * width + x) * 4) as usize;
        let bytes = sheet.bytes();
        (bytes[index], bytes[index + 1], bytes[index + 2])
    };
    let (r, _, b) = at(240, 100);
    assert!(r > 150 && b < 100, "the first cell is the red one");
    let (r, _, b) = at(720, 100);
    assert!(b > 150 && r < 100, "the second cell is the blue one");
    let (r, g, b) = at(490, 100);
    assert!(
        r < 30 && g < 30 && b < 30,
        "the tall picture is letterboxed in black"
    );
}
