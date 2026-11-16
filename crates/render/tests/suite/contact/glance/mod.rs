//! A glance at a whole video (#1563): frames chosen with judgement, and the
//! transcript beside them when there is one.
//!
//! The fixtures are built so each drop is visible in the moments chosen: an
//! opening of black that no moment may land in, footage that never changes
//! and so earns only the floor, footage that changes all the way through and
//! so earns the count its length is worth.

use std::path::{Path, PathBuf};

use scorsese_render::Tools;
use scorsese_render::contact::{self, ContactError, GLANCE_FEWEST, glance_count};

use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;

mod decoding;
mod words;

/// `seconds` of a picture that changes all the time and never repeats —
/// stripes whose frequency grows with the clock, so no two moments seconds
/// apart match — after `black` seconds of black.
pub(super) fn changing(tools: &Tools, dir: &Path, black: f64, seconds: f64) -> PathBuf {
    let file = dir.join(format!("changing-{black}-{seconds}.mp4"));
    let moving = format!(
        "nullsrc=s=96x54:r=5:d={seconds},geq=lum='128+110*sin(X*(0.05+T/150)+Y*0.07)'\
         :cb=128:cr=128,format=yuv420p"
    );
    let graph = if black > 0.0 {
        format!(
            "color=c=black:s=96x54:r=5:d={black},format=yuv420p[black];{moving}[moving];\
             [black][moving]concat=n=2:v=1:a=0"
        )
    } else {
        moving
    };
    generate(
        tools,
        &file,
        &["-filter_complex", &graph, "-pix_fmt", "yuv420p"],
    );
    file
}

/// `seconds` of one picture that never moves, busy enough not to be blank.
fn still_footage(tools: &Tools, dir: &Path, seconds: f64) -> PathBuf {
    let file = dir.join("still.mp4");
    let source = format!(
        "nullsrc=s=96x54:r=5:d={seconds},geq=lum='128+110*sin(X*0.1+Y*0.07)':cb=128:cr=128"
    );
    generate(
        tools,
        &file,
        &["-f", "lavfi", "-i", &source, "-pix_fmt", "yuv420p"],
    );
    file
}

#[test]
fn the_count_grows_with_the_length_between_the_floor_and_the_cap() {
    assert_eq!(glance_count(0.0), GLANCE_FEWEST);
    assert_eq!(glance_count(4.0), 3);
    assert_eq!(
        glance_count(180.0),
        12,
        "a three-minute video gets the most"
    );
    assert_eq!(glance_count(3600.0), 12);
    let counts: Vec<usize> = (0..40).map(|n| glance_count(f64::from(n) * 5.0)).collect();
    assert!(
        counts.windows(2).all(|pair| pair[0] <= pair[1]),
        "{counts:?}"
    );
    assert!(glance_count(60.0) > glance_count(10.0));
}

/// Footage that changes throughout earns the count its length is worth.
#[test]
fn a_changing_video_gets_the_frames_its_length_is_worth() {
    let dir = scratch::dir("glance-changing");
    let tools = tools();
    let file = changing(&tools, &dir, 0.0, 60.0);
    let glance = contact::glance(&tools, &file, None, false).expect("a glance");
    assert_eq!(
        glance.sheet.at_seconds.len(),
        glance_count(60.0),
        "{:?}",
        glance.sheet.at_seconds
    );
    let at = &glance.sheet.at_seconds;
    assert!(
        at.windows(2).all(|pair| pair[0] < pair[1]),
        "in order: {at:?}"
    );
    assert!(
        at[0] < 10.0 && *at.last().unwrap() > 50.0,
        "across the whole file: {at:?}"
    );
}

/// An opening second of black is not a frame worth sending.
#[test]
fn an_opening_of_black_is_never_a_frame() {
    let dir = scratch::dir("glance-black");
    let tools = tools();
    let file = changing(&tools, &dir, 4.0, 20.0);
    let glance = contact::glance(&tools, &file, None, false).expect("a glance");
    let at = &glance.sheet.at_seconds;
    assert!(at.len() >= GLANCE_FEWEST, "{at:?}");
    assert!(
        at.iter().all(|moment| *moment >= 4.0),
        "none in the black: {at:?}"
    );
}

/// A video that barely changes needs few frames: the floor, not the count
/// its length would be worth.
#[test]
fn a_video_that_never_changes_gets_the_floor() {
    let dir = scratch::dir("glance-still");
    let tools = tools();
    let file = still_footage(&tools, &dir, 90.0);
    let glance = contact::glance(&tools, &file, None, false).expect("a glance");
    assert_eq!(
        glance.sheet.at_seconds.len(),
        GLANCE_FEWEST,
        "{:?}",
        glance.sheet.at_seconds
    );
}

#[test]
fn a_sound_has_nothing_to_glance_at() {
    let dir = scratch::dir("glance-sound");
    let tools = tools();
    let file = dir.join("tone.wav");
    generate(&tools, &file, &["-f", "lavfi", "-i", "sine=duration=2"]);
    assert!(matches!(
        contact::glance(&tools, &file, None, false),
        Err(ContactError::NoPicture { .. })
    ));
}
