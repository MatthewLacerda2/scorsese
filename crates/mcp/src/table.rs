//! The tool table `docs/mcp.md` carries, rendered from the registry.
//!
//! That page is read cold — by a person deciding whether scorsese does what
//! they need, and by an agent that has not connected to the server yet and so
//! has never seen a handshake. Every fact the table states already exists in
//! this crate, next to the code that implements it, which is what makes those
//! facts get updated at all: you cannot change what a tool does without its
//! [`Tool::description`] being three lines from the cursor.
//!
//! So the table is not a second source. It is a rendering of the first one, and
//! `tests/table.rs` holds the checked-in page to it — the same contract
//! `cargo fmt --check` has. A tool added without a row was previously a
//! capability nobody reading the page could learn about, and nothing failed.
//!
//! **Three columns, no third string.** The name, the first sentence of the
//! description, the cost. Deliberately nothing hand-written per row: a
//! description whose opening sentence makes a poor cell is a description that
//! wants fixing, and fixing it improves what every client is shown at the same
//! time.
//!
//! Only the table is derived. The prose under it — how the tools *relate*,
//! which pairs with which, why one validates before writing — is knowledge no
//! per-tool description has and no generator could produce, and it stays
//! hand-written.

use crate::tools::{Tool, registry};

/// Opens the generated region in `docs/mcp.md`.
///
/// It says where the content comes from, because the person most likely to
/// edit the table by hand is the one who has not read this file.
pub const BEGIN: &str = "<!-- BEGIN TOOLS. Generated from the registry by `make mcp-table`; \
     edit the tool's description, not this table. -->";

/// Closes the generated region.
pub const END: &str = "<!-- END TOOLS -->";

/// The table, header rows included, with no marker around it.
pub fn tool_table() -> String {
    let mut out = String::from("| Tool | What it does | Costs |\n| --- | --- | --- |\n");
    for tool in registry() {
        out.push_str(&row(tool.as_ref()));
    }
    out
}

/// The page with its generated region replaced by the current table.
///
/// Returns `None` when the markers are not both there and in that order —
/// which is a page that cannot be generated into rather than one that is out
/// of date, and the two deserve different words.
pub fn regenerated(page: &str) -> Option<String> {
    let opens = page.find(BEGIN)?;
    let closes = page.find(END)?;
    if closes < opens {
        return None;
    }
    Some(format!(
        "{}{BEGIN}\n\n{}\n{}",
        &page[..opens],
        tool_table().trim_end(),
        &page[closes..]
    ))
}

/// One row: what it is called, what it is for, and what calling it spends.
fn row(tool: &dyn Tool) -> String {
    format!(
        "| `{}` | {} | {} |\n",
        tool.name(),
        cell(opening(tool.description())),
        tool.costs().says()
    )
}

/// The opening sentence of a description, the period kept.
///
/// A sentence ends at `". "` and nowhere else, which is why `docs/recipes.md`
/// and `*.scor` inside a description are safe: the dot in a filename is
/// followed by a letter, never by a space.
fn opening(description: &str) -> &str {
    description
        .find(". ")
        .map_or(description, |end| &description[..=end])
}

/// Text made safe to put in a table cell.
///
/// A `|` in a description would silently split the row into an extra column,
/// which renders as a table that is subtly wrong rather than as an error.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_ends_at_a_full_stop_and_a_space() {
        assert_eq!(
            opening("Look at the edit. Composites the timeline."),
            "Look at the edit."
        );
    }

    /// The dot in a filename is followed by a letter, so it is not the end of
    /// anything — a rule worth a test, because half these descriptions name a
    /// document.
    #[test]
    fn a_filename_is_not_the_end_of_a_sentence() {
        let one = "The format is documented in docs/project-format.md.";
        assert_eq!(opening(one), one);
        assert_eq!(
            opening("Reads a *.scor directory. And more."),
            "Reads a *.scor directory."
        );
    }

    #[test]
    fn a_pipe_cannot_break_the_row() {
        assert_eq!(cell("low | mid | high"), "low \\| mid \\| high");
    }

    #[test]
    fn a_page_without_markers_is_not_a_stale_page() {
        assert!(regenerated("# Nothing to generate into").is_none());
        assert!(regenerated(&format!("{END}\n{BEGIN}")).is_none());
    }
}
