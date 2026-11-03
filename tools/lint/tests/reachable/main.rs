//! Every `.rs` under a crate's `tests/suite/` is a module the suite's
//! `main.rs` reaches: an undeclared file is flagged and named with the module
//! that should declare it, `#[path]` and `x/mod.rs` are followed, data trees
//! stay out, every app test binary is walked, and an allow excuses an
//! `include!`d file.

mod parsing;

use std::path::{Path, PathBuf};

use scorsese_lint::rules::Finding;
use scorsese_lint::rules::reachable::{RULE, check};

pub(crate) const TEST: &str = "//! A test.\n\n#[test]\nfn a() {}\n";

/// What `check` finds in a tree of `(path, text)`.
fn findings(tree: &[(&str, &str)]) -> Vec<Finding> {
    let files: Vec<(&Path, &str)> = tree.iter().map(|(p, t)| (Path::new(*p), *t)).collect();
    check(&files)
}

/// The paths `check` flags in a tree of `(path, text)`.
pub(crate) fn flagged(tree: &[(&str, &str)]) -> Vec<PathBuf> {
    findings(tree).into_iter().map(|f| f.path).collect()
}

#[test]
fn a_file_no_module_declares_is_flagged_and_names_its_declarer() {
    let tree = [
        ("crates/mcp/tests/suite/main.rs", "mod protocol;\n"),
        ("crates/mcp/tests/suite/protocol/mod.rs", "mod tools;\n"),
        ("crates/mcp/tests/suite/protocol/tools.rs", TEST),
        ("crates/mcp/tests/suite/protocol/scripting.rs", TEST),
    ];
    let [finding] = &findings(&tree)[..] else {
        panic!("expected one finding");
    };
    assert_eq!(finding.rule, RULE);
    assert_eq!(
        finding.path,
        Path::new("crates/mcp/tests/suite/protocol/scripting.rs")
    );
    assert!(
        finding.message.contains("`mod scripting;`")
            && finding
                .message
                .contains("crates/mcp/tests/suite/protocol/mod.rs"),
        "{}",
        finding.message
    );
}
#[test]
fn a_suite_with_no_root_asks_for_its_main_rs() {
    let [finding] = &findings(&[("crates/a/tests/suite/t.rs", TEST)])[..] else {
        panic!("expected one finding");
    };
    assert!(
        finding.message.contains("crates/a/tests/suite/main.rs"),
        "{}",
        finding.message
    );
}

#[test]
fn every_app_test_binary_is_walked_too() {
    let tree = [
        ("app/tests/panels/main.rs", "mod drawing;\n"),
        ("app/tests/panels/drawing.rs", TEST),
        ("app/tests/panels/watchdog/mod.rs", TEST),
        ("app/tests/reload/main.rs", "mod voices;\n"),
        ("app/tests/reload/watching.rs", TEST),
        ("app/tests/icon.rs", TEST),
    ];
    assert_eq!(
        flagged(&tree),
        [
            PathBuf::from("app/tests/panels/watchdog/mod.rs"),
            PathBuf::from("app/tests/reload/watching.rs"),
        ]
    );
}
#[test]
fn an_allow_in_the_opening_comments_excuses_it() {
    let excused =
        "//! Spliced in.\n// lint: allow(unreachable-test) `include!`d by main.rs.\n\nfn a() {}\n";
    let bare = "// lint: allow(unreachable-test)\nfn a() {}\n";
    let tree = [
        (
            "crates/core/tests/suite/main.rs",
            "include!(\"spliced.rs\");\n",
        ),
        ("crates/core/tests/suite/spliced.rs", excused),
        ("crates/core/tests/suite/bare.rs", bare),
    ];
    assert_eq!(
        flagged(&tree),
        [PathBuf::from("crates/core/tests/suite/bare.rs")]
    );
}
