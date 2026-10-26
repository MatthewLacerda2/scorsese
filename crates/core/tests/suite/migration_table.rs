//! `docs/project-format.md`'s migration step table, held to the steps.
//!
//! The table is where a reader learns what `scorsese migrate` will do to their
//! document, one row per `vN → vN+1`. `migrate`'s own test holds its steps to
//! one per version from `OLDEST_MIGRATABLE` up to `SCHEMA_VERSION`; this holds
//! the table to the same range, so a bump that adds a step and forgets its row
//! (or a row left behind when the oldest moves forward) fails here (#1226).

use scorsese_core::SCHEMA_VERSION;
use scorsese_core::migrate::OLDEST_MIGRATABLE;

const DOC: &str = include_str!("../../../../docs/project-format.md");

/// Every `(from, to)` the table names, in the order it names them.
fn rows(doc: &str) -> Vec<(u32, u32)> {
    doc.lines()
        .filter_map(|line| line.strip_prefix("| v"))
        .filter_map(|rest| {
            let (step, _) = rest.split_once(" |")?;
            let (from, to) = step.split_once(" → v")?;
            Some((from.parse().ok()?, to.parse().ok()?))
        })
        .collect()
}

/// One row per version, oldest first, ending at `newest`.
fn expected(oldest: u32, newest: u32) -> Vec<(u32, u32)> {
    (oldest..newest).map(|from| (from, from + 1)).collect()
}

#[test]
fn the_table_has_one_row_per_step() {
    assert_eq!(
        rows(DOC),
        expected(OLDEST_MIGRATABLE, SCHEMA_VERSION),
        "docs/project-format.md's migration table must have one `| vN → vN+1 |` row \
         per version from OLDEST_MIGRATABLE to SCHEMA_VERSION, oldest first"
    );
}

#[test]
fn a_missing_or_stray_row_is_seen() {
    let doc = "| step | what | does |\n| --- | --- | --- |\n\
               | v1 → v2 | a | b |\n| v3 → v4 | c | d |\n| not a row |\n";
    assert_eq!(rows(doc), vec![(1, 2), (3, 4)]);
    assert_ne!(rows(doc), expected(1, 4));
    assert_eq!(rows(doc), [expected(1, 2), expected(3, 4)].concat());
}
