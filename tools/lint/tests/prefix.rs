//! Group by subfolder, not filename prefix: what makes a group, and what does
//! not.

use std::path::PathBuf;

use scorsese_lint::rules::prefix::{RULE, check};

/// The findings for these paths, each with empty contents.
fn groups(paths: &[&str]) -> Vec<String> {
    with_texts(&paths.iter().map(|path| (*path, "")).collect::<Vec<_>>())
}

/// The findings for these paths and contents, as their messages.
fn with_texts(files: &[(&str, &str)]) -> Vec<String> {
    let paths: Vec<(PathBuf, &str)> = files
        .iter()
        .map(|(path, text)| (PathBuf::from(path), *text))
        .collect();
    check(paths.iter().map(|(path, text)| (path, *text)))
        .into_iter()
        .map(|finding| format!("{}: {}", finding.path.display(), finding.message))
        .collect()
}

#[test]
fn three_siblings_sharing_a_word_are_a_group() {
    let found = groups(&[
        ".github/scripts/mutants-merge.py",
        ".github/scripts/mutants-scope.py",
        ".github/scripts/mutants-summary.py",
    ]);
    let [finding] = &found[..] else {
        panic!("expected one group, got {found:?}");
    };
    assert!(finding.starts_with(".github/scripts: 3 names share the prefix `mutants`"));
    assert!(finding.contains("`mutants/` subfolder"), "{finding}");
}

#[test]
fn two_are_not_a_group_and_other_directories_do_not_count() {
    assert!(
        groups(&[
            "crates/core/src/clip_a.rs",
            "crates/core/src/clip_b.rs",
            "crates/core/src/other/clip_c.rs",
        ])
        .is_empty()
    );
}

#[test]
fn the_test_prefix_says_nothing_and_a_bare_word_counts() {
    let found = groups(&[
        "tests/test_mergeable.py",
        "tests/test_mergeable_picking.py",
        "tests/test_mergeable_skipped.py",
        "tests/test_gh_retry.py",
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("prefix `mergeable`"), "{found:?}");
}

#[test]
fn a_component_and_its_test_are_one_name() {
    assert!(
        groups(&[
            "web/src/card.tsx",
            "web/src/card.test.tsx",
            "web/src/card-list.tsx",
        ])
        .is_empty()
    );
}

#[test]
fn names_without_a_shared_word_are_not_a_group() {
    assert!(
        groups(&[
            "src/lib.rs",
            "src/main.rs",
            "src/scan.rs",
            "src/__init__.py"
        ])
        .is_empty()
    );
}

#[test]
fn a_reasoned_allow_in_any_member_excuses_the_group_and_a_bare_one_does_not() {
    let names = ["src/word_a.rs", "src/word_b.rs", "src/word_c.rs"];
    let excused = format!("// lint: allow({RULE}) the three are generated as a set");
    assert!(with_texts(&[(names[0], ""), (names[1], &excused), (names[2], "")]).is_empty());
    let bare = format!("// lint: allow({RULE})");
    assert_eq!(
        with_texts(&[(names[0], ""), (names[1], &bare), (names[2], "")]).len(),
        1
    );
}
