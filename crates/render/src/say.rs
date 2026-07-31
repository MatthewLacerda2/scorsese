//! Saying a loudness measurement out loud.
//!
//! One phrasing, used by `scorsese render`, `scorsese synth bake` and the MCP
//! tools that wrap them. Three copies of a decibel format is three chances to
//! disagree about what "mean" means.

use scorsese_soundgen::level::Loudness;

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
