//! The order a folder of frames plays in, and where its numbering skips.
//!
//! **Numbers in a name are compared as numbers.** `frame_9.png` comes before
//! `frame_10.png`, which a plain sort of names gets backwards — and a camera
//! that rolls over from `IMG_9999` to `IMG_10000` is not rare. Everything else
//! is compared as text, case aside, so `shot_a_2` stays beside `shot_a_1`.
//! Leading zeros do not change a number, and decide between two names that are
//! otherwise equal, so the order is total and the same on every machine.
//!
//! **A gap is reported, never refused.** A frame missing from the middle of a
//! numbered run is nearly always one somebody deleted on purpose — the blurred
//! photo, the take with a hand in it — and the stills play in the order they
//! are listed whatever their numbers say. So the gap is named for whoever
//! imported the folder to judge, and nothing about the sequence depends on it.

use std::cmp::Ordering;

/// A name split into runs of digits and runs of everything else.
#[derive(Debug, PartialEq, Eq)]
enum Run<'a> {
    /// Digits, compared by value and then by how many there were.
    Number(&'a str),
    /// Anything else, compared as lowercase text.
    Text(&'a str),
}

fn runs(name: &str) -> Vec<Run<'_>> {
    let mut runs = Vec::new();
    let mut start = 0;
    let bytes = name.as_bytes();
    for at in 1..=bytes.len() {
        let boundary =
            at == bytes.len() || bytes[at].is_ascii_digit() != bytes[start].is_ascii_digit();
        if boundary {
            let run = &name[start..at];
            runs.push(if bytes[start].is_ascii_digit() {
                Run::Number(run)
            } else {
                Run::Text(run)
            });
            start = at;
        }
    }
    runs
}

fn compare_runs(a: &Run<'_>, b: &Run<'_>) -> Ordering {
    match (a, b) {
        (Run::Number(a), Run::Number(b)) => {
            let (a_trimmed, b_trimmed) = (a.trim_start_matches('0'), b.trim_start_matches('0'));
            // Longer without its zeros is larger, whatever the digits are —
            // so a number past `u64` still orders, without parsing it.
            a_trimmed
                .len()
                .cmp(&b_trimmed.len())
                .then_with(|| a_trimmed.cmp(b_trimmed))
                .then_with(|| a.len().cmp(&b.len()))
        }
        (Run::Number(_), Run::Text(_)) => Ordering::Less,
        (Run::Text(_), Run::Number(_)) => Ordering::Greater,
        (Run::Text(a), Run::Text(b)) => a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()),
    }
}

/// How two file names order as frames.
pub(super) fn natural(a: &str, b: &str) -> Ordering {
    let (a_runs, b_runs) = (runs(a), runs(b));
    a_runs
        .iter()
        .zip(&b_runs)
        .map(|(a, b)| compare_runs(a, b))
        .find(|ordering| ordering.is_ne())
        .unwrap_or_else(|| a_runs.len().cmp(&b_runs.len()))
        // Names that differ only in case still have one order.
        .then_with(|| a.cmp(b))
}

/// The frame number a name carries: its last run of digits, before the
/// extension. `None` when it has none, or one too long to be a count.
pub(super) fn number(name: &str) -> Option<u64> {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    runs(stem).into_iter().rev().find_map(|run| match run {
        Run::Number(digits) => digits.parse().ok(),
        Run::Text(_) => None,
    })
}

/// Where a numbered run of frames skips numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    /// The last frame before the gap, by file name.
    pub after: String,
    /// The first frame after it.
    pub before: String,
    /// How many numbers are missing between the two.
    pub missing: u64,
}

/// Every gap in `names`, which are already in the order they play. Nothing is
/// reported unless every name carries a number: a folder of `dawn.png`,
/// `noon.png` and `dusk.png` has no numbering to have a gap in.
pub(super) fn gaps(names: &[String]) -> Vec<Gap> {
    let Some(numbers) = names
        .iter()
        .map(|name| number(name))
        .collect::<Option<Vec<_>>>()
    else {
        return Vec::new();
    };
    numbers
        .windows(2)
        .zip(names.windows(2))
        .filter_map(|(pair, named)| {
            let missing = pair[1].checked_sub(pair[0])?.checked_sub(1)?;
            (missing > 0).then(|| Gap {
                after: named[0].clone(),
                before: named[1].clone(),
                missing,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(names: &[&str]) -> Vec<String> {
        let mut names: Vec<String> = names.iter().map(|&name| name.to_owned()).collect();
        names.sort_by(|a, b| natural(a, b));
        names
    }

    #[test]
    fn numbers_in_names_order_as_numbers() {
        assert_eq!(
            sorted(&[
                "frame_10.png",
                "frame_9.png",
                "frame_100.png",
                "frame_1.png"
            ]),
            [
                "frame_1.png",
                "frame_9.png",
                "frame_10.png",
                "frame_100.png"
            ]
        );
        assert_eq!(
            sorted(&["IMG_10000.JPG", "IMG_9999.JPG"]),
            ["IMG_9999.JPG", "IMG_10000.JPG"]
        );
    }

    #[test]
    fn padded_and_unpadded_numbers_agree_and_the_order_is_total() {
        assert_eq!(
            sorted(&["0002.png", "1.png", "03.png"]),
            ["1.png", "0002.png", "03.png"]
        );
        assert_eq!(sorted(&["01.png", "1.png"]), ["1.png", "01.png"]);
        assert_eq!(
            sorted(&["B.png", "a.png", "A.png"]),
            ["A.png", "a.png", "B.png"]
        );
    }

    #[test]
    fn the_frame_number_is_the_last_one_in_the_stem() {
        assert_eq!(number("take2_0041.png"), Some(41));
        assert_eq!(number("0007.png"), Some(7));
        assert_eq!(number("dusk.png"), None);
    }

    #[test]
    fn a_skipped_number_is_a_gap_and_unnumbered_frames_have_none() {
        let names = sorted(&["f_1.png", "f_2.png", "f_5.png", "f_6.png"]);
        assert_eq!(
            gaps(&names),
            [Gap {
                after: "f_2.png".to_owned(),
                before: "f_5.png".to_owned(),
                missing: 2
            }]
        );
        assert_eq!(gaps(&sorted(&["dawn.png", "f_1.png", "f_3.png"])), []);
    }
}
