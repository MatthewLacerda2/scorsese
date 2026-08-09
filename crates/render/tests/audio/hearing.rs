//! Reading a sound file as a picture, and as the findings beside it.
//!
//! Real files through real ffmpeg, because the claim is about files somebody
//! actually generated. What is asserted is the **findings** rather than the
//! pixels: a picture is for a person or a model to look at, and a test that
//! held one to a reference image would be asserting a layout where the thing at
//! stake is whether the numbers under it are true.

use std::path::{Path, PathBuf};

use scorsese_render::audio::waveform;
use scorsese_render::{Tools, audio};

use crate::common::ffmpeg::{fixture_dir, generate, tools};

/// A file of `seconds` at `amplitude`, 440 Hz.
fn tone(tools: &Tools, dir: &Path, name: &str, amplitude: &str, seconds: f64) -> PathBuf {
    let file = dir.join(format!("{name}.wav"));
    let graph = format!(
        "aevalsrc=exprs=({amplitude})*sin(2*PI*440*t):duration={seconds}:sample_rate=48000"
    );
    generate(tools, &file, &["-f", "lavfi", "-i", &graph]);
    file
}

/// The finding the whole tool exists for. A file of silence has a duration, a
/// size and a hash exactly like a file of speech, which is how three narration
/// lines got verified without anybody knowing whether they said anything.
#[test]
fn a_silent_file_is_reported_silent() {
    let tools = tools();
    let dir = fixture_dir("hear-silent");
    let file = tone(&tools, &dir, "quiet", "0", 2.0);

    let wave = waveform(&tools, &file).expect("a wav can be heard");

    assert!(wave.findings.is_silent(), "nothing is in this file");
    assert_eq!(wave.findings.peak_dbfs(), None, "silence has no level");
    assert!(
        (wave.seconds - 2.0).abs() < 0.05,
        "a 2s file should read 2s, and read {:.2}",
        wave.seconds
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// The other half of the same question: a file with sound in it reads at the
/// level it was written at, so "silent" is a finding and not the default.
#[test]
fn a_file_with_sound_in_it_reads_at_its_own_level() {
    let tools = tools();
    let dir = fixture_dir("hear-level");
    let file = tone(&tools, &dir, "half", "0.5", 2.0);

    let wave = waveform(&tools, &file).expect("a wav can be heard");
    let peak = wave.findings.peak_dbfs().expect("audible");

    assert!(!wave.findings.is_silent());
    assert!(
        (peak - 20.0 * 0.5_f64.log10()).abs() < 1.0,
        "a half-scale sine peaks near -6 dBFS, and read {peak:.1}"
    );
    assert_eq!(wave.findings.clipped, 0, "half scale does not clip");
    std::fs::remove_dir_all(&dir).ok();
}

/// A line that starts late is the failure a duration cannot show: the file is
/// exactly as long as it should be and the first word is not where it should
/// be. The picture shows the gap; this is the number under it.
#[test]
fn silence_at_the_head_is_measured_rather_than_ignored() {
    let tools = tools();
    let dir = fixture_dir("hear-late");
    let file = dir.join("late.wav");
    // Silent for the first second, then a tone: `gt(t,1)` is zero until it is
    // not, which is the shape of a narration line that starts late.
    let graph = "aevalsrc=exprs=0.5*gt(t\\,1)*sin(2*PI*440*t):duration=3:sample_rate=48000";
    generate(&tools, &file, &["-f", "lavfi", "-i", graph]);

    let wave = waveform(&tools, &file).expect("a wav can be heard");

    assert!(!wave.findings.is_silent(), "there is a tone in here");
    assert!(
        (wave.findings.lead_in - 1.0).abs() < 0.05,
        "it starts a second in, and was measured at {:.2}s",
        wave.findings.lead_in
    );
    assert!(
        wave.findings.lead_out < 0.05,
        "it runs to the end, and was measured as ending {:.2}s early",
        wave.findings.lead_out
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Clipping is the one thing on the picture that is always a fault, so it is
/// counted rather than left to be seen.
#[test]
fn a_file_driven_past_full_scale_is_reported_clipped() {
    let tools = tools();
    let dir = fixture_dir("hear-clipped");
    let file = tone(&tools, &dir, "loud", "1.4", 2.0);

    let wave: audio::Waveform = waveform(&tools, &file).expect("a wav can be heard");

    assert!(
        wave.findings.clipped > 0,
        "a sine driven to 1.4 flattens against full scale"
    );
    std::fs::remove_dir_all(&dir).ok();
}
