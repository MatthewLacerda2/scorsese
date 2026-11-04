//! Clicks, dropouts and stepped gain in a real file, read through ffmpeg.
//!
//! The measurement itself is `zimmer`'s and is tested there against signals
//! built sample by sample. What is asserted here is that a file on disk
//! reaches it whole: the picture's defects and the level report's come from
//! the same decode the numbers beside them do.

use scorsese_render::audio::{examine, waveform};
use scorsese_render::say;

use crate::common::audio::tone_file;
use crate::common::ffmpeg::tools;
use crate::common::scratch;

/// A tone with twenty milliseconds missing a second in — the "choppy" a
/// listener hears and a level report cannot.
#[test]
fn a_gap_in_a_file_is_a_dropout_at_its_moment() {
    let tools = tools();
    let dir = scratch::dir("defects-gap");
    let file = tone_file(&tools, &dir, "gap", "0.5*(1-between(t\\,1\\,1.02))", 2.0);

    let wave = waveform(&tools, &file).expect("a wav can be heard");
    assert_eq!(wave.defects.dropouts.count, 1, "{:?}", wave.defects);
    let gap = &wave.defects.dropouts.worst[0];
    assert!((gap.at_seconds - 1.0).abs() < 0.002, "{gap:?}");

    let (_, defects) = examine(&tools, &file).expect("a wav can be measured");
    assert_eq!(defects, wave.defects, "the picture and the report agree");
    std::fs::remove_dir_all(&dir).ok();
}

/// "Did the fix land?" as a number: a tone faded in half-decibel steps at
/// 60 Hz, against the same tone faded smoothly.
#[test]
fn a_stepped_fade_is_said_and_compared_with_a_smooth_one() {
    let tools = tools();
    let dir = scratch::dir("defects-steps");
    let stepped = "0.5*pow(10\\,-min(floor(t*60)\\,40)/40)";
    let smooth = "0.5*pow(10\\,-min(t*60\\,40)/40)";
    let stepped = tone_file(&tools, &dir, "stepped", stepped, 2.0);
    let smooth = tone_file(&tools, &dir, "smooth", smooth, 2.0);

    let (_, broken) = examine(&tools, &stepped).expect("measured");
    let (_, fixed) = examine(&tools, &smooth).expect("measured");
    assert_eq!(broken.zipper.count, 1, "{broken:?}");
    assert!(fixed.is_clean(), "{fixed:?}");

    let said = say::defects(&broken).join("\n");
    assert!(
        said.contains("steps 0.5 dB every 16.") && said.contains("(60 Hz)"),
        "{said}"
    );
    assert_eq!(
        say::defects_compared(&fixed, &broken),
        "defects  clicks 0 (was 0), dropouts 0 (was 0), stepped gain 0 (was 1)"
    );
    std::fs::remove_dir_all(&dir).ok();
}
