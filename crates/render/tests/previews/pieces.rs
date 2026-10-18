//! A preview played in pieces (#1448): a render of a few seconds remuxed into
//! a fragmented MP4 a browser can append — its first frame at zero, its sound
//! FLAC and exactly as long as the piece.

use std::path::Path;

use scorsese_render::preview::{self, Sound};
use scorsese_render::{Cancel, SampleRate};

use super::common::ffmpeg::{generate, inspect, tools};
use super::common::scratch;

/// The codec of each stream ffprobe finds in `file`, in order.
fn codecs(file: &Path) -> Vec<String> {
    let output = tools()
        .ffprobe()
        .args(["-v", "error", "-show_entries", "stream=codec_name"])
        .args(["-of", "csv=p=0"])
        .arg(file)
        .output()
        .expect("run ffprobe");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The presentation time of every frame of `file`, in display order — `None`
/// for a frame the browser would be handed untimed.
fn frame_times(file: &Path) -> Vec<Option<i64>> {
    let output = tools()
        .ffprobe()
        .args(["-v", "error", "-select_streams", "v:0"])
        .args(["-show_entries", "frame=pts", "-of", "csv=p=0"])
        .arg(file)
        .output()
        .expect("run ffprobe");
    let mut times: Vec<Option<i64>> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim_end_matches(',').parse().ok())
        .collect();
    times.sort_unstable();
    times
}

/// How many samples of sound `file` decodes to, one channel's worth.
fn samples(file: &Path) -> usize {
    let output = tools()
        .ffmpeg()
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-map", "0:a:0", "-ac", "1", "-f", "s16le", "-"])
        .output()
        .expect("run ffmpeg");
    output.stdout.len() / 2
}

/// The version of the first `trun` box: 1 is the one whose composition
/// offsets are signed, which is what puts a piece's first frame at zero.
fn trun_version(file: &Path) -> Option<u8> {
    let bytes = std::fs::read(file).expect("the piece reads");
    let at = bytes.windows(4).position(|window| window == b"trun")?;
    bytes.get(at + 4).copied()
}

/// A second of picture, and of tone when `sound`, encoded as a piece is
/// rendered: H.264 with B-frames, whose reordering would push the first frame
/// past zero, and PCM.
fn rendered(dir: &Path, sound: bool) -> std::path::PathBuf {
    let render = dir.join("render.avi");
    let mut arguments = vec!["-f", "lavfi", "-i", "testsrc2=s=160x90:d=1:r=30"];
    if sound {
        arguments.extend(["-f", "lavfi", "-i", "sine=frequency=440:duration=1"]);
        arguments.extend(["-ar", "48000", "-c:a", "pcm_s16le"]);
    }
    arguments.extend(["-c:v", "libx264", "-pix_fmt", "yuv420p"]);
    generate(&tools(), &render, &arguments);
    render
}

#[test]
fn a_piece_keeps_every_frame_from_zero_and_exactly_its_own_sound() {
    let tools = tools();
    let dir = scratch::dir("preview-piece");
    let render = rendered(&dir, true);
    let piece = dir.join("piece.mp4");
    preview::fragment(&tools, &render, Sound::Mixed, &piece, &Cancel::new())
        .expect("the piece is made");
    assert_eq!(codecs(&piece), ["h264", "flac"]);
    assert_eq!(inspect(&tools, &piece).frames, 30, "every frame kept");
    assert_eq!(trun_version(&piece), Some(1), "offsets written signed");
    // An avi render has no presentation times: each frame is timed again,
    // the first at zero and no two alike.
    let times = frame_times(&piece);
    assert_eq!(times.first(), Some(&Some(0)), "{times:?}");
    assert!(
        times
            .windows(2)
            .all(|pair| pair[0].is_some() && pair[0] < pair[1]),
        "{times:?}"
    );
    assert_eq!(samples(&piece), 48_000, "no priming, no padding");
}

#[test]
fn a_stretch_with_nothing_to_hear_is_silent_or_soundless_as_asked() {
    let tools = tools();
    let dir = scratch::dir("preview-piece-quiet");
    let render = rendered(&dir, false);
    let silent = dir.join("silent.mp4");
    let rate = SampleRate::DEFAULT;
    preview::fragment(
        &tools,
        &render,
        Sound::Silent(rate, 48_000),
        &silent,
        &Cancel::new(),
    )
    .expect("a silent piece is made");
    assert_eq!(codecs(&silent), ["h264", "flac"]);
    assert_eq!(samples(&silent), 48_000, "as long as the picture");

    let soundless = dir.join("soundless.mp4");
    preview::fragment(&tools, &render, Sound::Absent, &soundless, &Cancel::new())
        .expect("a picture-only piece is made");
    assert_eq!(codecs(&soundless), ["h264"]);

    // Asked for a mix the render does not have: an error, and nothing left.
    let mixed = dir.join("mixed.mp4");
    assert!(preview::fragment(&tools, &render, Sound::Mixed, &mixed, &Cancel::new()).is_err());
    assert!(!mixed.exists());
    assert!(!dir.join(".mixed.mp4.partial").exists());
}
