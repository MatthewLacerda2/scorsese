//! What actually came out — read back off the PNG rather than off the report.

use std::path::Path;

use scorsese_render::{Frame, Resolution, frames};

use crate::common::media;
use crate::{RASTER, shot, still, titled};

/// A written still, read back as a frame at the raster it was taken at.
fn written(file: &Path, raster: &str) -> Frame {
    let raster: Resolution = raster.parse().expect("the fixture raster parses");
    frames::read_png(&media::tools(), file, raster).expect("the still reads back")
}

/// The pixel at the middle of the picture, as `(r, g, b, a)`. The middle
/// because that is where a fitted source lands whatever shape it is — a corner
/// is letterbox as often as it is content.
fn centre(frame: &Frame) -> (u8, u8, u8, u8) {
    let raster = frame.resolution();
    let at = ((raster.height() / 2) * raster.width() + raster.width() / 2) as usize * 4;
    let bytes = frame.bytes();
    (bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3])
}

/// The whole claim of the command in one assertion: the PNG holds the picture
/// the timeline puts on screen at that instant. A path that composited nothing
/// would report success just as cheerfully.
#[test]
fn a_still_of_a_red_shot_is_red() {
    let dir = shot("still-red", "red");
    still(&dir, "frame.png", &["--at", "0s"]).ok();

    let (r, g, b, a) = centre(&written(&dir.join("frame.png"), RASTER));
    assert!(
        r > 200 && g < 60 && b < 60 && a == 255,
        "the still of a red shot came out ({r}, {g}, {b}, {a})"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// A title is composited by the same pass, so it reaches the PNG the same way —
/// and this fixture has no media at all, which is what makes it worth having:
/// nothing was decoded and there is still a picture.
///
/// Asserted as "something bright is on a black frame" rather than as a named
/// pixel, because which pixel a glyph covers is the text pipeline's business
/// and the golden renders are where that is held to anything.
#[test]
fn a_title_reaches_the_png_with_nothing_to_decode() {
    let dir = titled("still-title");
    still(&dir, "frame.png", &["--at", "0"]).ok();

    let frame = written(&dir.join("frame.png"), RASTER);
    let brightest = frame.bytes().iter().copied().max().unwrap_or_default();
    assert!(
        brightest > 200,
        "a still of a white title came out at most {brightest} bright — it is black"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// `--grid` reaches the PNG, and the two heavy rules cross where they claim to:
/// the middle of the frame is `0.5` on both axes, so the pixel dead centre of a
/// ruled still is the rule rather than the shot.
#[test]
fn a_ruled_still_carries_its_rules_into_the_png() {
    let dir = shot("still-grid", "red");
    still(&dir, "ruled.png", &["--at", "0s", "--grid"]).ok();

    let (r, g, b, _) = centre(&written(&dir.join("ruled.png"), RASTER));
    assert!(
        r > 150 && g > 150 && b > 150,
        "the centre of a ruled still came out ({r}, {g}, {b}) — the 0.5 rules \
         are not crossing there"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The picture is the same at every raster, which is the promise `--resolution`
/// rests on: layout is a fraction of the frame, so a smaller still is a cheaper
/// look at the same edit rather than a different one.
#[test]
fn the_raster_changes_the_size_and_not_the_picture() {
    let dir = shot("still-raster", "red");
    still(&dir, "big.png", &["--at", "0s", "--resolution", "160x90"]).ok();

    let frame = written(&dir.join("big.png"), "160x90");
    assert_eq!(
        frame.resolution().to_string(),
        "160x90",
        "the PNG is the raster asked for"
    );
    let (r, g, b, _) = centre(&frame);
    assert!(
        r > 200 && g < 60 && b < 60,
        "still red at a bigger raster: ({r}, {g}, {b})"
    );
    std::fs::remove_dir_all(dir).ok();
}
