//! `docs/mcp.md`'s tool table is generated, and this is what holds the page to
//! it.
//!
//! The contract is `cargo fmt --check`'s: the file on disk must be what the
//! generator would write, and a difference is a failure rather than something
//! this quietly repairs. `make mcp-table` is the repair, and it runs this same
//! test with `UPDATE_MCP_TABLE` set.
//!
//! What this replaces is nothing — which is the point. A tool added without a
//! row was invisible to every test in the repo, so the page simply drifted, and
//! an agent reading it cold learned about twenty-two capabilities out of
//! twenty-three without anything going wrong.

use std::path::PathBuf;

use scorsese_mcp::{regenerated, registry, tool_table};

/// A cell is the first sentence of a description. Past about this many
/// characters it stops being a cell and becomes a paragraph sitting in a
/// column, which is the point at which the *description* is what wants fixing.
const LONGEST_CELL: usize = 180;

/// The page, found from this crate rather than from the working directory:
/// `cargo test` may be run from anywhere in the workspace.
fn page() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/mcp.md")
}

/// The generated table, split into cells, header rows dropped.
fn rows() -> Vec<Vec<String>> {
    tool_table()
        .lines()
        .skip(2)
        .map(|line| {
            line.trim()
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn the_page_carries_exactly_what_the_registry_says() {
    let path = page();
    let on_disk = std::fs::read_to_string(&path).expect("docs/mcp.md is in the repo");
    let current =
        regenerated(&on_disk).expect("docs/mcp.md carries both TOOLS markers, in that order");
    if on_disk == current {
        return;
    }
    if std::env::var_os("UPDATE_MCP_TABLE").is_some() {
        std::fs::write(&path, current).expect("rewriting docs/mcp.md");
        return;
    }
    panic!(
        "docs/mcp.md's tool table is not what the registry says.\n\
         Run `make mcp-table` and commit what it writes."
    );
}

/// The failure this whole issue is about: a tool that exists and has no row.
#[test]
fn every_tool_in_the_registry_has_a_row() {
    let rows = rows();
    assert_eq!(
        rows.len(),
        registry().len(),
        "the table has {} rows for {} tools",
        rows.len(),
        registry().len()
    );
    for (row, tool) in rows.iter().zip(registry()) {
        assert_eq!(row.len(), 3, "`{}`'s row is not three cells", tool.name());
        assert_eq!(row[0], format!("`{}`", tool.name()));
        assert_eq!(row[2], tool.costs().says());
    }
}

/// The table is generated from the first sentence of each description, so a
/// description that opens with a paragraph puts a paragraph on the page. That
/// is a fixable thing about the description rather than about the table, and
/// this is where it gets said.
#[test]
fn every_cell_is_one_sentence_that_stands_alone() {
    for row in rows() {
        let what = &row[1];
        assert!(
            what.ends_with('.'),
            "{}'s description does not open with a whole sentence: {what}",
            row[0]
        );
        let length = what.chars().count();
        assert!(
            length <= LONGEST_CELL,
            "{}'s opening sentence is {length} characters, which is a paragraph \
             in a table cell. Shorten the description's first sentence.",
            row[0]
        );
    }
}
