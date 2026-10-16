//! `docs/prices.md`, held to the rate tables in code (#1218).
//!
//! That page is what an agent reads before quoting money, and `guide prices`
//! hands the same text to every MCP client and the web assistant. Nothing else
//! fails when a figure on it and the figure scorsese actually charges by
//! disagree, so every cell of its tables is read here and compared, both ways:
//! a row the code has and the page lacks fails as surely as a wrong number.
//!
//! Parsed rather than generated because the page is prose between its tables,
//! and each table sits where the argument about it is made.

mod assistant;
mod narration;
mod shots;
mod stills;

const PAGE: &str = include_str!("../../../../../docs/prices.md");

/// The page's `## heading` section, up to the next `## `.
///
/// Matched literally: renaming a heading fails here rather than quietly
/// finding nothing and passing.
fn section(heading: &str) -> &'static str {
    let start = PAGE
        .split_once(&format!("\n## {heading}\n"))
        .unwrap_or_else(|| panic!("docs/prices.md has no `## {heading}` section"))
        .1;
    start.split_once("\n## ").map_or(start, |(body, _)| body)
}

/// The rows of the table in `text` whose header line is `header`, each split
/// into its trimmed cells, the header and the `---` line left out.
fn table(text: &str, header: &str) -> Vec<Vec<String>> {
    let after = text
        .split_once(&format!("\n{header}\n"))
        .unwrap_or_else(|| panic!("docs/prices.md has no table headed `{header}`"))
        .1;
    after
        .lines()
        .skip_while(|line| line.starts_with("| ---"))
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            let line = line.trim().trim_start_matches('|').trim_end_matches('|');
            line.split('|').map(|cell| cell.trim().to_owned()).collect()
        })
        .collect()
}

/// The first `**YYYY-MM-DD**` in `text`: the day the section says its figures
/// were read.
fn checked_on(text: &str) -> String {
    text.split("**")
        .find(|piece| is_date(piece))
        .unwrap_or_else(|| panic!("no **YYYY-MM-DD** date in:\n{text}"))
        .to_owned()
}

fn is_date(piece: &str) -> bool {
    let bytes = piece.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(at, byte)| match at {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// `$4`, `$0.20`, `$0.0336` as whole micro-dollars, exactly.
fn micros(cell: &str) -> u64 {
    let figure = cell
        .strip_prefix('$')
        .unwrap_or_else(|| panic!("`{cell}` is not a dollar figure"));
    let (whole, fraction) = figure.split_once('.').unwrap_or((figure, ""));
    assert!(fraction.len() <= 6, "`{cell}` is finer than a micro-dollar");
    let number = |digits: &str| -> u64 {
        digits
            .parse()
            .unwrap_or_else(|_| panic!("`{cell}` is not a dollar figure"))
    };
    let fraction = format!("{fraction:0<6}");
    number(whole) * 1_000_000 + number(&fraction)
}

/// A cell as code, with its backticks taken off.
fn code(cell: &str) -> &str {
    cell.trim_matches('`')
}

/// The page's prose, with every run of whitespace one space, so a sentence a
/// line break falls in still matches.
fn prose(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn a_dollar_figure_reads_as_micro_dollars() {
    assert_eq!(micros("$4"), 4_000_000);
    assert_eq!(micros("$0.20"), 200_000);
    assert_eq!(micros("$0.0336"), 33_600);
    assert_eq!(micros("$2.50"), 2_500_000);
}

#[test]
#[should_panic(expected = "finer than a micro-dollar")]
fn a_figure_finer_than_a_micro_dollar_is_refused() {
    micros("$0.0000001");
}

#[test]
fn only_a_bold_date_is_the_checked_date() {
    assert_eq!(
        checked_on("updated 2026-10-07, read **soon** on **2026-10-08**"),
        "2026-10-08"
    );
}
