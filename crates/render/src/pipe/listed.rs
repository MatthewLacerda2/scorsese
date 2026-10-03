//! Decoding an image sequence: one file per output frame, read in order.
//!
//! **Which still is on screen when is ours to decide**, so it is decided before
//! ffmpeg is started: the shot works out which still every output frame shows
//! ([`frames`]) and hands ffmpeg the list, one entry per frame, through its
//! `concat` demuxer. ffmpeg only decodes — it never retimes a sequence. That is
//! why the filter chain has no `fps` in it for one: frames leave in exactly the
//! order and number they are listed, with `-fps_mode passthrough`.
//!
//! Asking ffmpeg to time it instead — each still given a duration in seconds
//! and the `fps` filter left to conform them — was tried and drops or doubles
//! a frame at hold boundaries: a duration of a third of a frame's worth of
//! microseconds rounds, and the filter's nearest-frame rule then lands on the
//! wrong side of it. A list per frame cannot.
//!
//! The list goes in on **stdin**, so nothing is written beside the project for
//! a render to clean up.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{ChildStdin, Command};

use scorsese_core::Fps;

use crate::plan::Shot;

/// Which file each of `count` output frames shows, starting where `shot`
/// does: `files` are the shot's stills in the sequence's order.
///
/// An output frame is a point on the **render's** grid; the sequence's holds
/// are on the **timeline's**, and the clip's speed sits between the two. So
/// each frame is carried back to the timeline as an exact ratio of the two
/// rates — a division of whole numbers, which lands exactly on a hold boundary
/// whenever it is one — then through the speed into the sequence.
pub(crate) fn frames(
    shot: &Shot<'_>,
    files: &[PathBuf],
    timeline: Fps,
    output: Fps,
    count: u64,
) -> Vec<PathBuf> {
    let Some(sequence) = &shot.asset.sequence else {
        return Vec::new();
    };
    positions(
        shot.source_in,
        shot.clip.speed.get(),
        timeline,
        output,
        count,
    )
    .filter_map(|position| files.get(sequence.still_at(position)?).cloned())
    .collect()
}

/// Where in the sequence, in timeline frames, each of `count` output frames
/// falls — starting `from` frames in and moving at `speed`.
fn positions(
    from: f64,
    speed: f64,
    timeline: Fps,
    output: Fps,
    count: u64,
) -> impl Iterator<Item = f64> {
    let numerator = u128::from(timeline.num()) * u128::from(output.den());
    let denominator = (u128::from(timeline.den()) * u128::from(output.num())).max(1);
    (0..count).map(move |frame| {
        let elapsed = (u128::from(frame) * numerator) as f64 / denominator as f64;
        from + elapsed * speed
    })
}

/// Points `command` at a list arriving on stdin, ahead of `-i`.
///
/// `file` and `pipe` are the two protocols the list needs: the stills are
/// files, named absolutely, and the list itself is the pipe. `-safe 0` because
/// an absolute path is what "unsafe" means to the demuxer.
pub(super) fn input(command: &mut Command) {
    command.args([
        "-protocol_whitelist",
        "file,pipe",
        "-f",
        "concat",
        "-safe",
        "0",
        "-i",
        "pipe:0",
        "-fps_mode",
        "passthrough",
    ]);
}

/// The list as ffmpeg's `concat` demuxer reads it: every file once per frame
/// it is shown, each lasting a second so the timestamps only ever climb.
///
/// The second is arbitrary and never seen — nothing downstream retimes a
/// listed source — but a duration of zero would hand the muxer the same
/// timestamp twice, which it reports as an error on every frame.
pub(super) fn script(files: &[PathBuf]) -> String {
    let mut script = String::from("ffconcat version 1.0\n");
    for file in files {
        let file = std::path::absolute(file).unwrap_or_else(|_| file.clone());
        // Single-quoted, with a quote inside closed, escaped and reopened —
        // the demuxer's own rule for a path with an apostrophe in it.
        let quoted = file.display().to_string().replace('\'', r"'\''");
        let _ = writeln!(script, "file 'file:{quoted}'\nduration 1");
    }
    script
}

/// Writes the list into ffmpeg and closes its stdin, on a thread of its own:
/// a long list is more than a pipe holds, and ffmpeg reads it while it opens
/// its input, so nothing here may wait on ffmpeg to finish reading.
pub(super) fn feed(mut stdin: ChildStdin, script: String) {
    std::thread::spawn(move || {
        // A failed write is ffmpeg having exited, which it reports itself.
        let _ = stdin.write_all(script.as_bytes());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(from: f64, speed: f64, timeline: Fps, output: Fps, count: u64) -> Vec<f64> {
        positions(from, speed, timeline, output, count).collect()
    }

    #[test]
    fn a_render_at_the_timeline_rate_steps_one_frame_at_a_time() {
        assert_eq!(at(3.0, 1.0, Fps::THIRTY, Fps::THIRTY, 3), [3.0, 4.0, 5.0]);
    }

    /// 24 timeline frames are 30 output frames, so every fifth output frame
    /// lands on a whole timeline frame — exactly, which is what a hold
    /// boundary needs.
    #[test]
    fn another_output_rate_lands_exactly_on_whole_timeline_frames() {
        let film = Fps::new(24, 1).expect("24 is a rate");
        let positions = at(0.0, 1.0, film, Fps::THIRTY, 11);
        assert_eq!(positions[5], 4.0);
        assert_eq!(positions[10], 8.0);
        assert!(positions[4] < 4.0);
    }

    #[test]
    fn a_clip_s_speed_moves_through_the_sequence_faster() {
        assert_eq!(at(0.0, 2.0, Fps::THIRTY, Fps::THIRTY, 3), [0.0, 2.0, 4.0]);
    }

    #[test]
    fn a_path_with_an_apostrophe_survives_the_quoting() {
        let script = script(&[PathBuf::from("/tmp/it's/0001.png")]);
        assert_eq!(
            script,
            "ffconcat version 1.0\nfile 'file:/tmp/it'\\''s/0001.png'\nduration 1\n"
        );
    }
}
