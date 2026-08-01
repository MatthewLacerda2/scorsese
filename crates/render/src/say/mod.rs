//! Saying a measurement out loud.
//!
//! One phrasing, used by `scorsese render`, `scorsese synth bake`, `scorsese
//! level` and the MCP tools that wrap them. Three copies of a decibel format is
//! three chances to disagree about what "mean" means.
//!
//! **Numbers, not pictures.** A spectrogram is the intuitive answer and it is
//! the wrong one for the reader this is written for: an assistant reads text
//! exactly and an image fuzzily, and twenty lines of text cost a fraction of
//! what an image does. A waveform drawn under a clip in the GUI timeline is a
//! genuinely good and completely separate feature — for the human, who can
//! hear, and who is scrubbing.

mod compare;
mod layers;
mod survey;
mod table;

use scorsese_soundgen::level::Loudness;

pub use compare::comparison;
pub use layers::layers;
pub use survey::survey;
pub use table::{headline, sections, summary};

/// How loud something came out, as a line to print after the thing it measures.
///
/// Silence is said in words rather than as a number, because a silence is not
/// "minus infinity decibels" to anyone reading a report — it is a clip that
/// makes no sound, which is a different sentence and usually a more urgent one.
///
/// True peak is named only when it differs from the sample peak by enough to
/// matter. It is the number that says whether a lossy encoder will clip, and
/// printing it beside an identical sample peak on every line would train
/// everyone to skip the pair.
pub fn loudness(level: &Loudness) -> String {
    let (Some(mean), Some(peak), Some(true_peak)) =
        (level.mean_dbfs, level.peak_dbfs, level.true_peak_dbfs)
    else {
        return "silent".to_owned();
    };
    let mut said = format!("mean {mean:.1} dBFS, peak {peak:.1} dBFS");
    if true_peak - peak >= TRUE_PEAK_WORTH_SAYING {
        said.push_str(&format!(", true peak {true_peak:.1} dBFS"));
    }
    if level.is_clipping() {
        said.push_str(" — clipping");
    }
    said
}

/// How far above the sample peak a true peak has to be before it is worth a
/// reader's attention, in decibels. A tenth of a decibel is inaudible and
/// within the noise of the interpolation.
const TRUE_PEAK_WORTH_SAYING: f64 = 0.1;

/// A time in the piece, as a reader would say it.
///
/// `m:ss` rather than a decimal count of seconds, because these are positions
/// in a piece of music and that is how anyone looking for one scrubs to it.
/// Hours appear when there are any and not otherwise — a forty-second bake
/// reading `0:00:08` puts two units of noise in front of the one that matters.
pub(crate) fn clock(seconds: f64) -> String {
    let whole = seconds.max(0.0).round() as u64;
    let (hours, minutes, seconds) = (whole / 3_600, (whole % 3_600) / 60, whole % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::clock;

    #[test]
    fn a_position_is_said_the_way_someone_scrubbing_would_say_it() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(8.4), "0:08");
        assert_eq!(clock(90.0), "1:30");
        assert_eq!(clock(3_754.0), "1:02:34");
        // Never a negative and never a panic, however the arithmetic that
        // produced it went: a report is not worth aborting a render over.
        assert_eq!(clock(-5.0), "0:00");
    }
}
