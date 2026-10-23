//! No history in an agent doc: issue and pull-request numbers, links to them
//! and dates are found; colours, entities, placeholders and versions are not.

use std::path::Path;

use scorsese_lint::rules::agent_docs::check_doc;
use scorsese_lint::rules::agent_docs::history::{RULE, cites};

#[test]
fn issue_numbers_links_and_dates_are_found() {
    for line in [
        "Fixed in #123.",
        "(#9)",
        "pinned to a commit (rusty#413)",
        "see https://github.com/o/r/issues/12",
        "or https://github.com/o/r/pull/7#discussion",
        "(the maintainer, 2026-10-23)",
        "since 2026-10-23T05:00:00Z",
    ] {
        assert!(cites(line).is_some(), "{line}");
    }
}

#[test]
fn what_only_looks_like_history_is_not() {
    for line in [
        "# A heading",
        "a colour, `#33ccffaa`, reads through",
        "an entity, &#123;, is markup",
        "one leaving it open writes `refs #N`",
        "`make mergeable PR=171`",
        "version 1.2.3 or 20261023",
        "the issues/ folder and /pull/ requests",
        "a range 2026-10 or 12-31",
    ] {
        assert_eq!(cites(line), None, "{line}");
    }
}

#[test]
fn each_line_that_cites_history_is_its_own_finding() {
    let text = "# Rules\n\nA rule (#12).\nAnother, 2026-01-02.\n";
    let found = check_doc(Path::new("crates/core/CLAUDE.md"), text);
    let lines: Vec<_> = found
        .iter()
        .map(|finding| (finding.rule, finding.line))
        .collect();
    assert_eq!(lines, [(RULE, Some(3)), (RULE, Some(4))]);
}

#[test]
fn an_html_comment_allow_above_with_a_reason_excuses_the_line() {
    let text = format!(
        "<!-- lint: allow({RULE}) the vendor's own changelog date -->\nAs of 2026-01-02.\nA rule (#12).\n"
    );
    let found = check_doc(Path::new(".claude/skills/a/SKILL.md"), &text);
    let lines: Vec<_> = found
        .iter()
        .filter(|finding| finding.rule == RULE)
        .map(|finding| finding.line)
        .collect();
    assert_eq!(lines, [Some(3)]);
}

#[test]
fn a_colour_of_digits_only_is_found_and_takes_an_allow() {
    assert!(cites("color: #000000").is_some());
}

#[test]
fn an_allow_inside_a_list_item_excuses_no_other_item() {
    let text = format!("* <!-- lint: allow({RULE}) a quoted changelog -->\n* fixed in #12\n");
    let found = check_doc(Path::new("CLAUDE.md"), &text);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].line, Some(2));
}

#[test]
fn other_markdown_may_cite_history() {
    assert!(check_doc(Path::new("docs/web.md"), "Since #12, on 2026-01-02.\n").is_empty());
}
