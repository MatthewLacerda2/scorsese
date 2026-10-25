//! A sound's peaks: the levels the web timeline draws a clip's waveform from.
//!
//! Real files through real ffmpeg, like the picture beside them, and asserted
//! by value: forty levels a second, each the loudest sample of its stretch on
//! a byte's scale.

use scorsese_render::audio::{MOST_PEAKS, PEAKS_PER_SECOND, Peaks, peaks};

use crate::common::audio::tone_file;
use crate::common::ffmpeg::{generate, tools};
use crate::common::scratch;

/// Silent for a second, then a half-scale tone: the first second's levels are
/// zero and the rest sit at half of the byte's range.
#[test]
fn a_sound_reads_as_forty_levels_a_second_at_its_own_level() {
    let tools = tools();
    let dir = scratch::dir("peaks-late");
    let file = tone_file(&tools, &dir, "late", "0.5*gt(t\\,1)", 3.0);

    let read = peaks(&tools, &file).expect("a wav has peaks");

    assert_eq!(read.per_second, 40.0);
    assert_eq!(read.levels.len(), 120, "three seconds at forty a second");
    assert!(
        read.levels[..40].iter().all(|level| *level == 0),
        "the first second is silent: {:?}",
        &read.levels[..40]
    );
    assert!(
        read.levels[41..]
            .iter()
            .all(|level| level.abs_diff(128) <= 1),
        "a half-scale tone peaks at half the range: {:?}",
        &read.levels[41..]
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A stretch left over at the end is a level of its own, not dropped.
#[test]
fn the_last_part_of_a_stretch_is_a_level_of_its_own() {
    let tools = tools();
    let dir = scratch::dir("peaks-tail");
    let file = tone_file(&tools, &dir, "tail", "0.5", 1.01);

    let read = peaks(&tools, &file).expect("a wav has peaks");

    assert_eq!(read.levels.len(), 41, "forty whole stretches and a part");
    assert!(read.levels[40].abs_diff(128) <= 1, "{:?}", read.levels);
    std::fs::remove_dir_all(&dir).ok();
}

/// A stereo file is read on frames, not samples, and a level is the louder
/// channel's: two seconds are eighty levels, and a sound in one channel only
/// still reads at its own level.
#[test]
fn a_stereo_sound_is_read_on_frames_at_its_louder_channel() {
    let tools = tools();
    let dir = scratch::dir("peaks-stereo");
    let file = dir.join("right.wav");
    let graph = "aevalsrc=exprs=0|0.5*sin(2*PI*440*t):duration=2:sample_rate=48000";
    generate(&tools, &file, &["-f", "lavfi", "-i", graph]);

    let read = peaks(&tools, &file).expect("a wav has peaks");

    assert_eq!(read.levels.len(), 80, "two seconds at forty a second");
    assert!(
        read.levels.iter().all(|level| level.abs_diff(128) <= 1),
        "{:?}",
        read.levels
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// What the server stores and sends: the rate as a little-endian `f32`, then
/// one byte a level.
#[test]
fn the_bytes_are_the_rate_then_a_byte_a_level() {
    let read = Peaks {
        per_second: 40.0,
        levels: vec![0, 128, 255],
    };

    let bytes = read.to_bytes();

    assert_eq!(bytes, [0, 0, 32, 66, 0, 128, 255]);
    assert_eq!(Peaks::from_bytes(&bytes), Some(read));
    assert_eq!(Peaks::from_bytes(&[0, 0, 32]), None, "too short for a rate");
}

/// A long file keeps to the cap by merging neighbours, each merged level the
/// loudest of those it stands for, and the rate falls to match.
#[test]
fn a_long_sound_is_merged_down_to_the_cap() {
    let read = Peaks {
        per_second: 40.0,
        levels: vec![1, 9, 3, 4, 7, 2, 5],
    };

    let fitted = read.fit(3);

    assert_eq!(fitted.levels, [9, 7, 5], "groups of three, ceiling of 7/3");
    assert_eq!(fitted.per_second, 40.0 / 3.0);
}

/// A file under the cap is left as it was.
#[test]
fn a_short_sound_is_not_merged() {
    let read = Peaks {
        per_second: 40.0,
        levels: vec![1, 2, 3],
    };

    assert_eq!(read.clone().fit(3), read);
    assert_eq!(PEAKS_PER_SECOND, 40);
    assert_eq!(MOST_PEAKS, 32_768);
}

/// What reaches the browser, as `PEAKS_PER_SECOND` and `MOST_PEAKS` document
/// it: three minutes as they were read, an hour merged in runs of five.
#[test]
fn a_three_minute_and_an_hour_long_sound_on_the_wire() {
    let at = |seconds: usize| {
        Peaks {
            per_second: PEAKS_PER_SECOND as f32,
            levels: vec![0; seconds * PEAKS_PER_SECOND as usize],
        }
        .fit(MOST_PEAKS)
    };

    assert_eq!(at(180).to_bytes().len(), 7_204);
    let hour = at(3600);
    assert_eq!((hour.per_second, hour.to_bytes().len()), (8.0, 28_804));
}
