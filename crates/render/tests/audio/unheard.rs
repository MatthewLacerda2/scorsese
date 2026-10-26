//! A recording's speech not handed over: one past the longest, refused from
//! its probe, and one stopped while ffmpeg decodes it (#1549).

use scorsese_render::Cancel;
use scorsese_render::audio::speech;

use super::speech::LONGEST;
use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;

/// A recording past the longest one heard is refused from its probe, before
/// any of it is decoded.
#[test]
fn a_recording_past_the_longest_is_refused_before_it_is_decoded() {
    let tools = tools();
    let dir = scratch::dir("speech-long");
    let film = dir.join("long.wav");
    generate(
        &tools,
        &film,
        &["-f", "lavfi", "-i", "sine=frequency=440:duration=3"],
    );
    let error = speech(&tools, &film, 2.0, &Cancel::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("longer than"), "{error}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A stop is heard while ffmpeg decodes, and nothing comes back.
#[test]
fn a_stopped_decode_answers_stopped() {
    let tools = tools();
    let dir = scratch::dir("speech-stopped");
    let film = dir.join("take.wav");
    generate(
        &tools,
        &film,
        &["-f", "lavfi", "-i", "sine=frequency=440:duration=3"],
    );
    let cancel = Cancel::new();
    cancel.cancel();
    let error = speech(&tools, &film, LONGEST, &cancel)
        .unwrap_err()
        .to_string();
    assert!(error.contains("stopped"), "{error}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A file ffprobe cannot read is said to be unreadable, not to have no sound.
#[test]
fn a_file_that_will_not_probe_is_not_called_silent() {
    let tools = tools();
    let dir = scratch::dir("speech-unprobed");
    let film = dir.join("broken.mp4");
    std::fs::write(&film, b"not a film at all").expect("as set up");
    let error = speech(&tools, &film, LONGEST, &Cancel::new())
        .unwrap_err()
        .to_string();
    assert!(!error.contains("no sound"), "{error}");
    assert!(error.contains("could not be read"), "{error}");
    std::fs::remove_dir_all(&dir).ok();
}
