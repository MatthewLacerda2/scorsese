//! Two measurements, side by side.
//!
//! Each row is the value on the left and the move on the right, because that is
//! the order the two are read in: what is it now, and how far did it come. The
//! left column is the *first* subject throughout — a comparison whose sides
//! swapped between rows would be worse than no comparison.

use scorsese_zimmer::level::{BandsDifference, Difference, Profile};

/// A field-by-field comparison, as lines to print under the two names.
///
/// A difference of nothing gets one line saying so rather than five lines of
/// `+0.0`. That case is not a curiosity: it is what a re-bake of an unchanged
/// recipe produces, and "nothing changed" is the answer to the question that
/// was asked.
pub fn comparison(this: &Profile, that: &Profile) -> Vec<String> {
    let difference = Difference::between(&this.whole, &that.whole);
    if difference.is_nothing() {
        return vec!["identical — nothing measurably changed".to_owned()];
    }

    let mut lines = Vec::new();
    lines.push(decibels(
        "mean",
        this.whole.loudness.mean_dbfs,
        difference.mean_db,
        Sense::Loudness,
    ));
    lines.push(decibels(
        "peak",
        this.whole.loudness.peak_dbfs,
        difference.peak_db,
        Sense::Loudness,
    ));
    lines.push(decibels(
        "crest",
        this.whole.crest_db(),
        difference.crest_db,
        Sense::Dynamics,
    ));
    if let (Some(bands), Some(moved)) = (this.whole.bands, difference.bands) {
        let (low, mid, high) = bands.percentages();
        lines.push(balance(
            &[("low", low), ("mid", mid), ("high", high)],
            &moved,
        ));
    }
    if difference.seconds.abs() >= WORTH_SAYING_SECONDS {
        lines.push(format!(
            "length  {:>6.1} s     ( {:+.1} s )",
            this.seconds(),
            difference.seconds
        ));
    }
    lines
}

/// How far two lengths have to differ before it is worth a row, in seconds.
/// Below a frame at any rate anyone renders at, two files are the same length.
const WORTH_SAYING_SECONDS: f64 = 0.05;

/// Which way "more" reads for a field, so the row can say it in a word.
///
/// A word, because the sign alone is ambiguous in exactly the place it matters:
/// −4.6 dB of *mean* is quieter and −4.6 dB of *crest* is flatter, and those are
/// different findings that a reader skimming a column of signed numbers will
/// conflate.
#[derive(Debug, Clone, Copy)]
enum Sense {
    /// Up is louder.
    Loudness,
    /// Up is more dynamic range, down is closer to a wall.
    Dynamics,
}

impl Sense {
    fn word(self, by: f64) -> &'static str {
        match (self, by > 0.0) {
            (Self::Loudness, true) => " louder",
            (Self::Loudness, false) => " quieter",
            (Self::Dynamics, true) => " more dynamic",
            (Self::Dynamics, false) => " flatter",
        }
    }
}

/// One decibel row: where it stands, and how far it moved.
fn decibels(field: &str, value: Option<f64>, by: Option<f64>, sense: Sense) -> String {
    let Some(value) = value else {
        return format!("{field:<7}  silent");
    };
    match by {
        Some(by) if by.abs() >= WORTH_SAYING_DB => format!(
            "{field:<7} {value:>6.1} dB     ( {by:+.1} dB{} )",
            sense.word(by)
        ),
        // Present but unmoved, and said rather than dropped: a row that
        // disappears when it matches makes the reader work out which of five
        // fields is missing.
        Some(_) => format!("{field:<7} {value:>6.1} dB     ( unchanged )"),
        None => format!("{field:<7} {value:>6.1} dB     ( the other is silent )"),
    }
}

/// How far a level has to move before it is worth calling a move, in decibels.
/// A tenth of a decibel is inaudible, and printing one trains a reader to
/// ignore the column it is in.
const WORTH_SAYING_DB: f64 = 0.1;

/// The three band shares and how far each moved, on one line.
///
/// One line rather than three, because the finding is always a *shift between*
/// bands — energy that left the top arrived somewhere — and three separate rows
/// hide the trade that is the whole point.
fn balance(shares: &[(&str, u32); 3], moved: &BandsDifference) -> String {
    let mut said = String::new();
    for ((name, share), by) in shares.iter().zip([moved.low, moved.mid, moved.high]) {
        if !said.is_empty() {
            said.push_str("   ");
        }
        said.push_str(&format!("{name} {share:>3}% ( {by:+.0} pts )"));
    }
    format!("bands   {said}")
}
