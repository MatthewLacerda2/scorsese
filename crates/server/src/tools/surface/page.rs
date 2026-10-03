//! `docs/web.md`'s lists of the tools served as they are, held to [`STORED`]
//! and [`PROJECT_FILES`].
//!
//! The page used to carry them as one table cell each, every name on one line,
//! so any two pull requests adding a tool conflicted there as well as in the
//! code (#691). Now each list is generated, one bullet a line, between markers
//! — the way `docs/mcp.md`'s table is — and this test is what holds the page to
//! it: a difference is a failure, and `make mcp-table` is the repair, running
//! this same test with `UPDATE_WEB_TOOLS` set.

use std::path::PathBuf;

use super::{PROJECT_FILES, STORED};

/// Each generated region: the name its markers carry, and what fills it.
const REGIONS: [(&str, &[&str]); 2] = [("STORED", STORED), ("PROJECT_FILES", PROJECT_FILES)];

/// The page, found from this crate rather than from the working directory.
fn page() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/web.md")
}

/// The opening marker of the region called `region`.
fn begin(region: &str) -> String {
    format!(
        "<!-- BEGIN {region}. Generated from `tools/surface` by `make mcp-table`; \
         edit the list there, not here. -->"
    )
}

/// `page` with the region called `region` holding `names`, one bullet each,
/// or `None` when its markers are not both there and in that order.
fn regenerated(page: &str, region: &str, names: &[&str]) -> Option<String> {
    let begin = begin(region);
    let end = format!("<!-- END {region} -->");
    let opens = page.find(&begin)?;
    let closes = page.find(&end)?;
    if closes < opens {
        return None;
    }
    let list: String = names.iter().map(|name| format!("- `{name}`\n")).collect();
    Some(format!(
        "{}{begin}\n\n{list}\n{}",
        &page[..opens],
        &page[closes..]
    ))
}

#[test]
fn the_page_lists_exactly_the_tools_served_as_they_are() {
    let path = page();
    let on_disk = std::fs::read_to_string(&path).expect("docs/web.md is in the repo");
    let mut current = on_disk.clone();
    for (region, names) in REGIONS {
        current = regenerated(&current, region, names)
            .unwrap_or_else(|| panic!("docs/web.md carries both {region} markers, in order"));
    }
    if on_disk == current {
        return;
    }
    if std::env::var_os("UPDATE_WEB_TOOLS").is_some() {
        std::fs::write(&path, current).expect("rewriting docs/web.md");
        return;
    }
    panic!(
        "docs/web.md's served-tool lists are not what tools/surface says.\n\
         Run `make mcp-table` and commit what it writes."
    );
}

#[test]
fn a_region_is_one_bullet_a_line_and_nothing_outside_it_moves() {
    let page = format!("before\n{}\nstale\n<!-- END X -->\nafter\n", begin("X"));
    let written = regenerated(&page, "X", &["a", "b"]).expect("both markers");
    assert_eq!(
        written,
        format!(
            "before\n{}\n\n- `a`\n- `b`\n\n<!-- END X -->\nafter\n",
            begin("X")
        )
    );
    assert_eq!(regenerated(&written, "X", &["a", "b"]), Some(written));
    assert!(regenerated("<!-- END X -->\n", "X", &["a"]).is_none());
}
