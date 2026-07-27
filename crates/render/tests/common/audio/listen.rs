//! Decoding a rendered file back and measuring how loud it is.

use std::path::Path;

use scorsese_render::Tools;

use super::fixtures::RATE;

/// Whether the file carries an audio stream at all — asked separately, because
/// "no soundtrack" and "a silent soundtrack" are different outcomes and only one
/// of them can be decoded.
pub(crate) fn has_soundtrack(tools: &Tools, file: &Path) -> bool {
    let output = tools
        .ffprobe()
        .args(["-v", "error", "-select_streams", "a"])
        .args(["-show_entries", "stream=index", "-of", "csv=p=0"])
        .arg(file)
        .output()
        .expect("run ffprobe");
    assert!(
        output.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    !output.stdout.is_empty()
}

/// The rendered file's soundtrack, decoded back to mono samples at [`RATE`].
///
/// Mono because loudness is the question and a downmix answers it for both
/// channels at once. Empty when the file has no audio stream at all.
pub(crate) fn soundtrack(tools: &Tools, file: &Path) -> Vec<f32> {
    if !has_soundtrack(tools, file) {
        return Vec::new();
    }
    let output = tools
        .ffmpeg()
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-vn", "-ac", "1", "-ar", &RATE.to_string()])
        .args(["-f", "f32le", "-"])
        .output()
        .expect("run ffmpeg");
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
        .stdout
        .chunks_exact(size_of::<f32>())
        .map(|word| f32::from_le_bytes(word.try_into().expect("four bytes")))
        .collect()
}

/// Root-mean-square level of the window `from..to`, in seconds — how loud that
/// stretch is, which is what every assertion here is really about.
///
/// A window past the end of the samples reads as silence rather than panicking:
/// "there is nothing there" is a legitimate answer, and the tests that ask it
/// mean it.
pub(crate) fn level(samples: &[f32], from: f64, to: f64) -> f32 {
    let at = |seconds: f64| ((seconds * f64::from(RATE)) as usize).min(samples.len());
    let window = &samples[at(from)..at(to)];
    if window.is_empty() {
        return 0.0;
    }
    let total: f64 = window
        .iter()
        .map(|&sample| f64::from(sample) * f64::from(sample))
        .sum();
    (total / window.len() as f64).sqrt() as f32
}

/// Loud enough to be unmistakably sound.
///
/// A sine of amplitude `a` reads `0.707a`, and mixing down to mono for analysis
/// costs another 3 dB when the source was upmixed to stereo on the way in — so
/// the quietest fixture here, at 0.3, still lands near 0.15.
pub(crate) const AUDIBLE: f32 = 0.08;

/// Quiet enough to be unmistakably nothing. Not zero: a lossy encoder rings
/// slightly either side of a hard edge, so demanding digital silence would be
/// asserting something about AAC rather than about scorsese.
pub(crate) const SILENT: f32 = 0.02;

#[track_caller]
pub(crate) fn assert_audible(level: f32, what: &str) {
    assert!(
        level > AUDIBLE,
        "{what}: expected sound, found level {level}"
    );
}

#[track_caller]
pub(crate) fn assert_silent(level: f32, what: &str) {
    assert!(
        level < SILENT,
        "{what}: expected silence, found level {level}"
    );
}
