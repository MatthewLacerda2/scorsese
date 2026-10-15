//! The inline allow: what grants one, how far up it reaches, and the whole
//! tree walked end to end.

use std::fs;
use std::path::Path;

use scorsese_lint::check_tree;
use scorsese_lint::rules::allow::{allowed, grants};

#[test]
fn an_allow_needs_its_rule_and_a_reason() {
    assert!(grants("// lint: allow(skip) hangs on purpose", "skip"));
    assert!(grants("# lint: allow(skip) — hangs on purpose", "skip"));
    assert!(!grants("// lint: allow(skip)", "skip"));
    assert!(!grants("// lint: allow(skip) —", "skip"));
    assert!(!grants("// lint: allow(marker) reason", "skip"));
    assert!(!grants("// lint: allow(skipped) reason", "skip"));
}

#[test]
fn it_reaches_up_through_comments_and_attributes_but_not_past_code() {
    let lines = [
        "// lint: allow(skip) why",
        "/// doc",
        "#[test]",
        "#[ignore]",
    ];
    assert!(allowed(&lines, 3, "skip"));
    let lines = ["// lint: allow(skip) why", "fn a() {}", "#[ignore]"];
    assert!(!allowed(&lines, 2, "skip"));
    assert!(allowed(&["#[ignore] // lint: allow(skip) why"], 0, "skip"));
}

#[test]
fn the_tree_is_walked_and_its_findings_ordered() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("conventions");
    let _ = fs::remove_dir_all(&root);
    let write = |path: &str, text: &str| {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, text).expect("write");
    };
    write("crates/a/src/lib.rs", "//! A crate.\npub mod b;\n");
    write("crates/a/tests/t.rs", "#[test]\n#[ignore]\nfn t() {}\n");
    write("target/debug/build/x.rs", "#[ignore]\n");
    write("notes.md", "#[ignore]\n");

    let found: Vec<String> = check_tree(&root)
        .expect("walk")
        .iter()
        .map(ToString::to_string)
        .collect();

    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].starts_with("crates/a/src/lib.rs:2: [crate-doc]"),
        "{found:?}"
    );
    assert!(
        found[1].starts_with("crates/a/tests/t.rs:2: [skip]"),
        "{found:?}"
    );
}
