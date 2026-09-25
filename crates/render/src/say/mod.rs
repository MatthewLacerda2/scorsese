//! Saying a measurement out loud.
//!
//! One phrasing, used by `scorsese render`, `scorsese synth bake`, `scorsese
//! level` and the MCP tools that wrap them — including what a render says
//! about the file it delivered ([`written`], [`delivery`]). Three copies of a
//! decibel format is three chances to disagree about what "mean" means.
//!
//! **Numbers, not pictures.** A spectrogram is the intuitive answer and it is
//! the wrong one for the reader this is written for: an assistant reads text
//! exactly and an image fuzzily, and twenty lines of text cost a fraction of
//! what an image does. A waveform drawn under a clip in the GUI timeline is a
//! genuinely good and completely separate feature — for the human, who can
//! hear, and who is scrubbing.

mod compare;
mod delivery;
mod grid;
mod layers;
mod survey;
mod table;

use scorsese_zimmer::level::Loudness;

pub use compare::comparison;
pub use delivery::{delivery, written};
pub use grid::grid;
pub use layers::layers;
pub use survey::survey;
pub use table::{arrangement, headline, sections, summary};

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

/// A time in the piece, as a caller would place something on it.
///
/// Seconds to the millisecond, because that is the unit every tool that puts
/// something on the timeline takes — `place_clip`'s `start_seconds` among them
/// — and a millisecond is well inside a frame at any rate a video runs at.
/// A clock rounded to the whole second, which this used to be, was readable
/// and useless for the one thing a section boundary is wanted for: putting a
/// caption, a cut or a title on it. Converting `1:06` back to seconds by hand
/// is the arithmetic the report exists to save.
pub(crate) fn moment(seconds: f64) -> String {
    format!("{:.3}", seconds.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::moment;

    #[test]
    fn a_position_is_said_in_seconds_precise_enough_to_place_a_clip_on() {
        assert_eq!(moment(0.0), "0.000");
        assert_eq!(moment(43.479_166), "43.479");
        assert_eq!(moment(90.0), "90.000");
        // Never a negative and never a panic, however the arithmetic that
        // produced it went: a report is not worth aborting a render over.
        assert_eq!(moment(-5.0), "0.000");
    }
}
