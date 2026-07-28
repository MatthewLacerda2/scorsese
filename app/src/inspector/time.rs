//! A frame count, said the way a person says it.
//!
//! The document is frames and stays frames — this is the edge where one
//! becomes a time, the same edge `scorsese-core` already draws between its
//! exact grid and the seconds it shows anybody.

use scorsese_core::{Fps, Frames};

/// A frame count as a clock reading: `0:04.5`, `2:03.0`, `1:02:03.4`.
///
/// Tenths of a second rather than a timecode's frame field, for two reasons.
/// The exact frame is in the control immediately beside this one, so a second
/// exact answer here would only be a chance to disagree with the first. And a
/// frame field has to invent a subdivision for 29.97 that the grid does not
/// actually have — the well-known place timecode starts lying about how much
/// time has passed.
///
/// Hours appear only when there are hours: `1:02:03.4` is a reading, and
/// `0:00:04.5` for a four-second clip is a form to be filled in.
pub(super) fn readable(frames: Frames, fps: Fps) -> String {
    // Rounded once, into tenths, so a value a hair under a second cannot come
    // out as `0:00.10`.
    let tenths = (fps.seconds(frames).max(0.0) * 10.0).round() as u64;
    let (whole, tenth) = (tenths / 10, tenths % 10);
    let (hours, minutes, seconds) = (whole / 3600, (whole / 60) % 60, whole % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}.{tenth}")
    } else {
        format!("{minutes}:{seconds:02}.{tenth}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_count_reads_as_a_time() {
        assert_eq!(readable(Frames(0), Fps::THIRTY), "0:00.0");
        assert_eq!(readable(Frames(135), Fps::THIRTY), "0:04.5");
        assert_eq!(readable(Frames(1800), Fps::THIRTY), "1:00.0");
        assert_eq!(readable(Frames(3723 * 30), Fps::THIRTY), "1:02:03.0");
    }

    /// Hours are only mentioned when there are any. A four-second clip reading
    /// `0:00:04.5` is a form, not a time.
    #[test]
    fn hours_appear_only_once_there_are_hours() {
        assert_eq!(readable(Frames(107_997), Fps::THIRTY), "59:59.9");
        assert_eq!(readable(Frames(108_000), Fps::THIRTY), "1:00:00.0");
    }

    /// The rate the grid exists for. 30 frames at 29.97 is a hair over a
    /// second, and saying so beats rounding it down to look tidy.
    #[test]
    fn a_broadcast_rate_is_read_off_the_real_grid() {
        assert_eq!(readable(Frames(30), Fps::NTSC), "0:01.0");
        assert_eq!(readable(Frames(1800), Fps::NTSC), "1:00.1");
    }

    /// Tenths are rounded, never truncated: 0.99s is a second to a reader, and
    /// a tenths field that can print `10` is a bug.
    #[test]
    fn a_value_just_under_a_second_rounds_into_it() {
        assert_eq!(readable(Frames(29), Fps::THIRTY), "0:01.0");
    }
}
