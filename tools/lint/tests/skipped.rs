//! A test runs or fails: what counts as skipping one, and which database
//! variable a server test may read.

use std::path::Path;

use scorsese_lint::rules::skips::{DATABASE_ENV, SKIP, check};

/// The rules flagged in `text` at `path`, with their lines.
fn found(path: &str, text: &str) -> Vec<(&'static str, Option<usize>)> {
    check(Path::new(path), text)
        .into_iter()
        .map(|finding| (finding.rule, finding.line))
        .collect()
}

#[test]
fn an_ignore_attribute_is_a_skip() {
    let text = "#[test]\n#[ignore]\nfn a() {}\n";
    assert_eq!(found("crates/core/tests/a.rs", text), [(SKIP, Some(2))]);
    let text = "#[test]\n#[ignore = \"slow\"]\nfn a() {}\n";
    assert_eq!(found("crates/core/tests/a.rs", text), [(SKIP, Some(2))]);
}

#[test]
fn a_cfg_attr_ignore_is_a_skip_even_across_lines() {
    let text = "#[test]\n#[cfg_attr(\n    windows,\n    ignore = \"no\"\n)]\nfn a() {}\n";
    assert_eq!(found("a.rs", text), [(SKIP, Some(2))]);
    let text = "#[cfg_attr(windows, ignore)]\nfn a() {}\n";
    assert_eq!(found("a.rs", text), [(SKIP, Some(1))]);
}

#[test]
fn other_cfg_attrs_and_mentions_in_comments_are_not() {
    let text = "#[cfg_attr(test, derive(Debug))]\nstruct A;\n// an `#[ignore]` here is prose\n";
    assert!(found("a.rs", text).is_empty());
    assert!(found("a.py", "#[ignore]\n").is_empty());
}

#[test]
fn an_allow_above_the_test_attribute_excuses_the_skip() {
    let text =
        format!("// lint: allow({SKIP}) hangs on purpose\n#[test]\n#[ignore]\nfn a() {{}}\n");
    assert!(found("a.rs", &text).is_empty());
}

#[test]
fn a_server_test_may_not_read_the_servers_database_variable() {
    let text = "let url = std::env::var(\"DATABASE_URL\");\n";
    assert_eq!(
        found("crates/server/tests/a.rs", text),
        [(DATABASE_ENV, Some(1))]
    );
    let text = "let url = env::var_os(DATABASE_URL);\n";
    assert_eq!(
        found("crates/server/tests/a.rs", text),
        [(DATABASE_ENV, Some(1))]
    );
}

#[test]
fn its_own_variable_naming_the_constant_and_other_crates_are_fine() {
    let text = "let url = std::env::var(\"SCORSESE_TEST_DATABASE_URL\");\n";
    assert!(found("crates/server/tests/a.rs", text).is_empty());
    let text = "let vars = [(DATABASE_URL, URL)];\n";
    assert!(found("crates/server/tests/a.rs", text).is_empty());
    let text = "let url = std::env::var(\"DATABASE_URL\");\n";
    assert!(found("crates/server/src/config.rs", text).is_empty());
}
