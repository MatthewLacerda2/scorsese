//! `docs/references.md` and the reference figures in code, held to each other.
//!
//! The page is where the records were measured and argued over, and the bake
//! report reads the code. A figure changed on one side only would have the
//! report holding a bake to numbers the page no longer says, or the reverse.

use scorsese_render::audio::reference::{REFERENCES, Range};

const PAGE: &str = include_str!("../../../../docs/references.md");

/// Matched literally, so renaming a header fails here rather than quietly
/// finding no rows.
const NUMBERS: &str = "| reference | length | whole-file mean | body mean | body crest | body low / mid / high % | body corr |";
const AIMS: &str = "| reference | crest to aim for | mean that comes with it |";
const OPENINGS: &str = "| reference | first 8 s | under its body by |";

/// The rows of the table under `header`, each split into trimmed cells.
fn table(header: &str) -> Vec<Vec<String>> {
    let after = PAGE
        .split_once(header)
        .unwrap_or_else(|| panic!("no table `{header}` on the page"))
        .1;
    after
        .lines()
        .skip(1)
        .skip_while(|line| line.starts_with("| ---"))
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            let cells = line.trim().trim_matches('|').split('|');
            cells.map(|cell| cell.trim().to_owned()).collect()
        })
        .collect()
}

/// Every number in `cell`, in order. `−` is a minus sign and `–` a range's
/// dash, as the page writes them.
fn numbers(cell: &str) -> Vec<f64> {
    let mut found = Vec::new();
    let mut current = String::new();
    for ch in cell.chars() {
        match ch {
            '0'..='9' | '.' if !(ch == '.' && current.is_empty()) => current.push(ch),
            '−' | '-' | '+' => {
                flush(&mut current, &mut found);
                current.push(if ch == '+' { '+' } else { '-' });
            }
            _ => flush(&mut current, &mut found),
        }
    }
    flush(&mut current, &mut found);
    found
}

fn flush(current: &mut String, found: &mut Vec<f64>) {
    if let Ok(number) = current.parse() {
        found.push(number);
    }
    current.clear();
}

/// The first two numbers of `cell`, lowest first — the range a cell opens
/// with, before any prose after it.
fn range(cell: &str) -> Range {
    let found = numbers(cell);
    assert!(found.len() >= 2, "`{cell}` holds no range");
    Range {
        low: found[0].min(found[1]),
        high: found[0].max(found[1]),
    }
}

#[test]
fn the_numbers_table_is_the_code_s() {
    let rows = table(NUMBERS);
    let names: Vec<&str> = rows.iter().map(|row| row[0].as_str()).collect();
    let coded: Vec<&str> = REFERENCES.iter().map(|it| it.row).collect();
    assert_eq!(names, coded, "the page's records, in order");
    for (row, reference) in rows.iter().zip(REFERENCES) {
        let name = reference.row;
        assert_eq!(range(&row[3]), reference.mean, "{name}: body mean");
        assert_eq!(range(&row[4]), reference.crest, "{name}: body crest");
        let bands: Vec<Range> = row[5].split('/').map(range).collect();
        let coded = vec![reference.low, reference.mid, reference.high];
        assert_eq!(bands, coded, "{name}: body low / mid / high");
        let width = (!row[6].contains("mono")).then(|| range(&row[6]));
        assert_eq!(width, reference.correlation, "{name}: body corr");
    }
}

#[test]
fn the_aims_table_is_the_code_s() {
    let rows = table(AIMS);
    let aimed: Vec<_> = REFERENCES
        .iter()
        .filter(|it| it.aim_crest.is_some())
        .collect();
    let names: Vec<&str> = rows.iter().map(|row| row[0].as_str()).collect();
    let coded: Vec<&str> = aimed.iter().map(|it| it.row).collect();
    assert_eq!(
        names, coded,
        "the records with a crest to aim for, in order"
    );
    for (row, reference) in rows.iter().zip(aimed) {
        let name = reference.row;
        assert_eq!(
            numbers(&row[1]).first().copied(),
            reference.aim_crest,
            "{name}: aim"
        );
        assert_eq!(
            numbers(&row[2]).first().copied(),
            reference.reachable_mean(),
            "{name}: the mean that comes with it"
        );
    }
}

#[test]
fn the_openings_table_is_the_code_s() {
    let rows = table(OPENINGS);
    let names: Vec<&str> = rows.iter().map(|row| row[0].as_str()).collect();
    let coded: Vec<&str> = REFERENCES.iter().map(|it| it.row).collect();
    assert_eq!(names, coded, "the page's records, in order");
    for (row, reference) in rows.iter().zip(REFERENCES) {
        assert_eq!(
            numbers(&row[2]).first().copied(),
            Some(reference.opening_under_db),
            "{}: how far it opens under its body",
            reference.row
        );
    }
}

#[test]
fn a_cell_s_numbers_read_minus_signs_and_range_dashes_apart() {
    assert_eq!(numbers("−12.5 to −16.5"), vec![-12.5, -16.5]);
    assert_eq!(
        numbers("14–18.5 (mostly 15–16)"),
        vec![14.0, 18.5, 15.0, 16.0]
    );
    assert_eq!(numbers("~−1.5"), vec![-1.5]);
    assert_eq!(numbers("+0.6 to +0.97"), vec![0.6, 0.97]);
}
