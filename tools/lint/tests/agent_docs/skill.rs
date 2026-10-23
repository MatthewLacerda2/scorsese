//! A skill's frontmatter: a name Claude accepts and equal to its folder, a
//! description that is there and not too long.

use std::path::Path;

use scorsese_lint::rules::agent_docs::check_doc;
use scorsese_lint::rules::agent_docs::skill::{
    DESCRIPTION_LIMIT, NAME_LIMIT, RULE, block, field, frontmatter_findings,
};

use super::skill_md;

/// What is wrong with a `SKILL.md` of this frontmatter in folder `ci-merge`.
fn wrong(frontmatter: &str) -> Vec<String> {
    frontmatter_findings(&skill_md(frontmatter), "ci-merge")
}

#[test]
fn a_loadable_skill_passes() {
    assert!(wrong("name: ci-merge\ndescription: Merge a branch.").is_empty());
    assert!(wrong("name: \"ci-merge\"\ndescription: 'Merge a branch.'").is_empty());
}

#[test]
fn a_skill_with_no_frontmatter_is_flagged() {
    let found = frontmatter_findings("# A skill\n", "ci-merge");
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("no frontmatter"));
    assert!(
        frontmatter_findings("---\nname: ci-merge\n", "ci-merge")[0].contains("no frontmatter")
    );
}

#[test]
fn a_name_must_be_there_and_equal_its_folder() {
    assert!(wrong("description: Merge.")[0].contains("no `name`"));
    assert!(wrong("name:\ndescription: Merge.")[0].contains("empty `name`"));
    let other = wrong("name: merging\ndescription: Merge.");
    assert_eq!(other.len(), 1);
    assert!(other[0].contains("differs from its folder"));
}

#[test]
fn a_name_holds_only_lower_case_letters_digits_and_hyphens() {
    for bad in ["CI-merge", "ci_merge", "ci merge", "ci-mergé"] {
        let found =
            frontmatter_findings(&skill_md(&format!("name: {bad}\ndescription: Merge.")), bad);
        assert_eq!(found.len(), 1, "{bad}: {found:?}");
        assert!(found[0].contains("only lower-case"), "{bad}");
    }
    let fine = "a-1-b";
    assert!(
        frontmatter_findings(&skill_md(&format!("name: {fine}\ndescription: x")), fine).is_empty()
    );
}

#[test]
fn a_name_over_its_limit_is_flagged() {
    let long = "a".repeat(NAME_LIMIT + 1);
    let found = frontmatter_findings(&skill_md(&format!("name: {long}\ndescription: x")), &long);
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("65 characters"));
    let at = "a".repeat(NAME_LIMIT);
    assert!(
        frontmatter_findings(&skill_md(&format!("name: {at}\ndescription: x")), &at).is_empty()
    );
}

#[test]
fn a_description_must_be_there_not_empty_and_within_its_limit() {
    assert!(wrong("name: ci-merge")[0].contains("no `description`"));
    assert!(wrong("name: ci-merge\ndescription: \"\"")[0].contains("empty `description`"));
    let long = "é".repeat(DESCRIPTION_LIMIT + 1);
    assert!(wrong(&format!("name: ci-merge\ndescription: {long}"))[0].contains("1025 characters"));
    let at = "é".repeat(DESCRIPTION_LIMIT);
    assert!(wrong(&format!("name: ci-merge\ndescription: {at}")).is_empty());
}

#[test]
fn a_block_scalar_description_is_read_whole() {
    let text =
        skill_md("name: ci-merge\ndescription: >\n  Merge a branch,\n  one at a time.\nother: x");
    let block = block(&text).expect("a block");
    assert_eq!(
        field(&block, "description").as_deref(),
        Some("Merge a branch, one at a time.")
    );
    assert!(frontmatter_findings(&text, "ci-merge").is_empty());
}

#[test]
fn only_skill_md_is_held_to_the_frontmatter() {
    let path = Path::new(".claude/skills/ci-merge/SKILL.md");
    let [finding] = &check_doc(path, "# no frontmatter\n")[..] else {
        panic!("expected one finding");
    };
    assert_eq!((finding.rule, finding.line), (RULE, Some(1)));
    let beside = Path::new(".claude/skills/ci-merge/notes.md");
    assert!(check_doc(beside, "# no frontmatter\n").is_empty());
}
