//! The YAML a skill's frontmatter may be written in: every shape a
//! description can take is measured whole, and the valid ones pass.

use scorsese_lint::rules::agent_docs::skill::{DESCRIPTION_LIMIT, frontmatter_findings};

use super::skill_md;

/// What is wrong with a `SKILL.md` of this frontmatter in folder `ci-merge`.
fn wrong(frontmatter: &str) -> Vec<String> {
    frontmatter_findings(&skill_md(frontmatter), "ci-merge")
}

/// A description longer than the limit, as indented lines of 100 characters.
fn long_lines() -> String {
    let line = format!("  {}\n", "a".repeat(99));
    line.repeat(DESCRIPTION_LIMIT / 100 + 2)
}

#[test]
fn a_plain_description_continued_on_indented_lines_is_measured_whole() {
    let found = wrong(&format!(
        "name: ci-merge\ndescription: Merge.\n{}",
        long_lines()
    ));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("over 1024"));
}

#[test]
fn a_description_starting_on_the_next_line_is_read() {
    assert!(wrong("name: ci-merge\ndescription:\n  Merge a branch.").is_empty());
}

#[test]
fn a_block_scalar_runs_past_a_blank_line_and_takes_any_indicator() {
    let blank = wrong("name: ci-merge\ndescription: >\n  Merge,\n\n  one at a time.\nother: x");
    assert!(blank.is_empty(), "{blank:?}");
    for indicator in [">+", "|-", ">2", "|+2"] {
        let found = wrong(&format!(
            "name: ci-merge\ndescription: {indicator}\n{}",
            long_lines()
        ));
        assert_eq!(found.len(), 1, "{indicator}: {found:?}");
        assert!(found[0].contains("over 1024"), "{indicator}");
    }
}

#[test]
fn a_comment_after_a_plain_value_is_not_part_of_it() {
    assert!(wrong("name: ci-merge # the folder's name\ndescription: Merge.").is_empty());
}

#[test]
fn a_byte_order_mark_does_not_hide_the_frontmatter() {
    let text = format!(
        "\u{feff}{}",
        skill_md("name: ci-merge\ndescription: Merge.")
    );
    assert!(frontmatter_findings(&text, "ci-merge").is_empty());
}
