//! The sectioned, banded table: one line for the whole thing, one row per
//! stretch of it.
//!
//! The columns are chosen so a reader can scan **down** one of them and find
//! the row that is wrong, which is the whole point of splitting the file up in
//! the first place. So every row carries every column, in the same order, at
//! the same width — a table that omits a column when it has nothing to say
//! about it is a table whose rows no longer line up.

use scorsese_zimmer::level::{Bands, Profile, Span};

use super::{clock, loudness};

/// The one-line summary: how long it runs, how loud, and where its energy sits.
pub fn summary(profile: &Profile) -> String {
    let mut said = format!(
        "{:.1} s  {}",
        profile.seconds(),
        loudness(&profile.whole.loudness)
    );
    if let Some(bands) = profile.whole.bands {
        said.push_str(&format!(", {}", balance(&bands)));
    }
    said
}

/// [`summary`] with a name in front of it.
///
/// The subject is passed in rather than taken from the profile because a
/// profile does not know whether it came from a file, a mix or a clip — and
/// naming it is the caller's business, since the caller is the one that will
/// print two of these next to each other.
pub fn headline(subject: &str, profile: &Profile) -> String {
    format!("{subject}  {}", summary(profile))
}

/// One row per section, or nothing at all when there is only one stretch.
///
/// Rows are returned rather than printed, and without leading indentation, so
/// the caller decides how far in they sit — the CLI nests them under a mix, MCP
/// hands them back as lines of a reply, and neither should have to strip the
/// other's formatting.
pub fn sections(profile: &Profile) -> Vec<String> {
    let width = profile
        .sections
        .iter()
        .filter_map(|span| span.label.as_ref())
        .map(String::len)
        .max();
    profile
        .sections
        .iter()
        .map(|span| row(span, width))
        .collect()
}

/// One section's row.
fn row(span: &Span, label_width: Option<usize>) -> String {
    let mut said = format!(
        "{:>5}-{:<5}",
        clock(span.from_seconds),
        clock(span.to_seconds)
    );
    if let Some(width) = label_width {
        said.push_str(&format!(
            "{:<width$}  ",
            span.label.as_deref().unwrap_or(""),
            width = width
        ));
    }
    said.push_str(&columns(span));
    said
}

/// The measured columns of a row, without whatever names it.
///
/// Shared with the per-track table beside this one, because the whole claim
/// both tables make is that their numbers can be read against each other and
/// against the summary above them. Two formatters would eventually disagree
/// about a width and turn that reading into a puzzle.
pub(super) fn columns(span: &Span) -> String {
    let (Some(mean), Some(peak)) = (span.loudness.mean_dbfs, span.loudness.peak_dbfs) else {
        return "silent".to_owned();
    };
    let mut said = format!("mean {mean:>6.1}   peak {peak:>6.1}");
    // Crest is peak minus mean, so it is free once both are in hand and it is
    // the cheapest proxy there is for "does this have dynamics or is it a wall".
    if let Some(crest) = span.crest_db() {
        said.push_str(&format!("   crest {crest:>5.1}"));
    }
    if let Some(bands) = span.bands {
        said.push_str(&format!("   {}", balance(&bands)));
    }
    said
}

/// Three band shares, as whole percentages.
///
/// Percentages rather than decibels per band: the question these answer is
/// "which part of the spectrum is this piece mostly made of", and a share of
/// the whole is the form that question is asked in.
fn balance(bands: &Bands) -> String {
    let (low, mid, high) = bands.percentages();
    format!("low {low:>3}%  mid {mid:>3}%  high {high:>3}%")
}

#[cfg(test)]
mod tests {
    use super::*;
    use scorsese_zimmer::level::{Cut, Profiler};

    /// A signal whose two halves differ by a known amount has to say so on the
    /// right rows — which is the entire claim this table makes.
    #[test]
    fn the_rows_say_which_stretch_was_quiet() {
        let rate = 1_000;
        let mut profiler = Profiler::sectioned(
            1,
            rate,
            vec![
                Cut {
                    label: "verse".to_owned(),
                    end_seconds: 1.0,
                },
                Cut {
                    label: "chorus".to_owned(),
                    end_seconds: 2.0,
                },
            ],
        );
        let square = |level: f32| -> Vec<f32> {
            (0..rate)
                .map(|i| if i % 2 == 0 { level } else { -level })
                .collect()
        };
        profiler.feed(&square(0.5));
        profiler.feed(&square(1.0));

        let rows = sections(&profiler.finish());
        assert_eq!(rows.len(), 2, "one row per pattern in the arrangement");
        assert!(rows[0].contains("verse"), "{}", rows[0]);
        assert!(rows[0].contains("mean   -6.0"), "{}", rows[0]);
        assert!(rows[1].contains("chorus"), "{}", rows[1]);
        assert!(rows[1].contains("mean    0.0"), "{}", rows[1]);
        assert!(rows[0].starts_with(" 0:00-0:01"), "{}", rows[0]);
    }

    /// One stretch is the whole file said twice, so there is no table at all.
    #[test]
    fn a_signal_with_nothing_to_compare_gets_no_table() {
        let mut profiler = Profiler::new(1, 1_000);
        profiler.feed(&[0.5; 500]);
        assert!(sections(&profiler.finish()).is_empty());
    }
}
