//! Files a glance still answers for: one of nothing but black, and one whose
//! picture stops before its sound.

use scorsese_render::contact::{self, GLANCE_FEWEST};

use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;

/// All of it black still answers with a sheet: blank frames are what there
/// is, and none at all would read as a broken file.
#[test]
fn a_video_of_nothing_but_black_still_shows_the_floor() {
    let dir = scratch::dir("glance-all-black");
    let tools = tools();
    let file = dir.join("black.mp4");
    generate(
        &tools,
        &file,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=96x54:r=5:d=30",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let glance = contact::glance(&tools, &file, None, false).expect("a glance");
    assert_eq!(glance.sheet.at_seconds.len(), GLANCE_FEWEST);
}

/// A file whose picture stops before its sound — five seconds of video in
/// twenty of audio — is glanced at what decodes, not refused for the
/// moments past the picture's end.
#[test]
fn a_picture_shorter_than_its_sound_is_glanced_at_what_decodes() {
    let dir = scratch::dir("glance-short-picture");
    let tools = tools();
    let file = dir.join("short.mp4");
    let picture =
        "nullsrc=s=96x54:r=5:d=5,geq=lum='128+110*sin(X*(0.05+T/3)+Y*0.07)':cb=128:cr=128";
    generate(
        &tools,
        &file,
        &[
            "-f",
            "lavfi",
            "-i",
            picture,
            "-f",
            "lavfi",
            "-i",
            "sine=duration=20",
            "-pix_fmt",
            "yuv420p",
        ],
    );
    let glance = contact::glance(&tools, &file, None, false).expect("a glance");
    let at = &glance.sheet.at_seconds;
    assert!(
        !at.is_empty() && at.iter().all(|moment| *moment < 5.0),
        "{at:?}"
    );
}
