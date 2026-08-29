//! Reading back a file that has more than one channel in it.
//!
//! Its own file because the defect was its own kind. Every other test here
//! feeds the readers a mono fixture, where `-ac 1` was a no-op and the decode
//! could not be wrong — so a downmix that summed the channels rather than
//! averaging them read 3 dB hot for as long as nothing stereo was ever measured
//! (#452). What pins it is a signal whose answer is known before ffmpeg is
//! asked: two channels of a known level, and the level that comes back.

use std::path::{Path, PathBuf};

use scorsese_render::Tools;
use scorsese_render::audio::{measure, waveform};

use crate::common::ffmpeg::{fixture_dir, generate, tools};
use crate::loudness::SLACK;

/// Two seconds of a stereo file whose channels are `left` and `right`.
fn pair(tools: &Tools, dir: &Path, name: &str, left: &str, right: &str) -> PathBuf {
    let file = dir.join(format!("{name}.wav"));
    let graph = format!("aevalsrc=exprs={left}|{right}:duration=2:sample_rate=48000");
    generate(tools, &file, &["-f", "lavfi", "-i", &graph]);
    file
}

/// A half-scale 440 Hz sine, the same one both channels of a centred file get.
const HALF: &str = "0.5*sin(2*PI*440*t)";

/// The same claim the mono test makes, on the file shape that broke it: a
/// half-scale sine is 3 dB under its own peak whether it is written once or
/// twice, and a report that summed the two would say 3 dB over.
#[test]
fn a_stereo_file_reads_at_the_level_it_was_written_at() {
    let tools = tools();
    let dir = fixture_dir("stereo-level");
    let file = pair(&tools, &dir, "centred", HALF, HALF);

    let level = measure(&tools, &file).expect("a stereo wav can be measured");
    let mean = level.whole.loudness.mean_dbfs.expect("audible");
    let peak = level.whole.loudness.peak_dbfs.expect("audible");

    let expected = 20.0 * (0.5 / std::f64::consts::SQRT_2).log10();
    assert!(
        (mean - expected).abs() < SLACK,
        "a half-scale stereo sine should read near {expected:.1} dBFS, and read {mean:.1}"
    );
    assert!(
        (peak - 20.0 * 0.5_f64.log10()).abs() < 1.0,
        "its peak is the sample it contains, -6.0 dBFS, and read {peak:.1}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The damaging half of the defect. zimmer's limiter holds a bake a decibel
/// under full scale and the file measures exactly that; the reader used to
/// call it clipping by two, which is a correct mix reported as a fault an
/// author would then go and "fix".
///
/// The fixture sits at 0.8 rather than up against the ceiling on purpose. The
/// meter is fed a run at a time and counts each run boundary as an edge, which
/// over-reports true peak by about a decibel however many channels the file
/// has — #471, found here and separate from this one. A fixture with no margin
/// would be asserting that both defects are fixed.
#[test]
fn a_centred_file_under_full_scale_is_not_reported_clipping() {
    let tools = tools();
    let dir = fixture_dir("stereo-clipping");
    let loud = "0.8*sin(2*PI*440*t)";
    let file = pair(&tools, &dir, "loud", loud, loud);

    let level = measure(&tools, &file).expect("measured");
    let loudness = level.whole.loudness;

    assert!(
        !loudness.is_clipping(),
        "nothing in this file reaches full scale: {loudness:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The case that decides the shape. A file with everything on one side has the
/// peak that channel has — an *averaging* downmix would report it 6 dB under
/// itself and pass a file at the ceiling as having headroom, which is the same
/// defect the other way round.
#[test]
fn a_hard_panned_file_reads_at_the_peak_it_actually_has() {
    let tools = tools();
    let dir = fixture_dir("stereo-panned");
    let file = pair(&tools, &dir, "left", "0.9*sin(2*PI*440*t)", "0");

    let level = measure(&tools, &file).expect("measured");
    let peak = level.whole.loudness.peak_dbfs.expect("audible");

    assert!(
        (peak - 20.0 * 0.9_f64.log10()).abs() < 1.0,
        "the left channel reaches -0.9 dBFS, and the file read {peak:.1}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The picture and the report read the same file through the same decode, so
/// the one thing they must never do is disagree about it — including how long
/// it is, which is a count of sample *frames* and not of samples.
#[test]
fn the_picture_and_the_report_agree_about_a_stereo_file() {
    let tools = tools();
    let dir = fixture_dir("stereo-picture");
    let file = pair(&tools, &dir, "centred", HALF, HALF);

    let reported = measure(&tools, &file).expect("measured");
    let wave = waveform(&tools, &file).expect("heard");
    let level = reported.whole.loudness.peak_dbfs.expect("audible");
    let drawn = wave.findings.peak_dbfs().expect("audible");

    assert!(
        (drawn - level).abs() < 0.5,
        "the picture peaks at {drawn:.1} dBFS where the report says {level:.1}"
    );
    assert_eq!(wave.findings.clipped, 0, "half scale does not clip");
    assert!(
        (wave.seconds - 2.0).abs() < 0.05,
        "a 2s stereo file is 2s long, and read {:.2}",
        wave.seconds
    );
    std::fs::remove_dir_all(&dir).ok();
}
