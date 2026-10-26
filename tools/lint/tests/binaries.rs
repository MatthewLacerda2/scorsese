//! Each crate links one integration-test binary, `tests/suite/main.rs`: a
//! file Cargo would build as a binary of its own is flagged, a module of the
//! suite, a helper or another tree's tests pass, and an allow excuses one.

use std::path::Path;

use scorsese_lint::rules::binaries::{RULE, check};

const TEST: &str = "//! A test.\n\n#[test]\nfn a() {}\n";

#[test]
fn a_test_file_cargo_would_link_alone_is_flagged() {
    for path in [
        "crates/core/tests/time.rs",
        "crates/server/tests/api/main.rs",
    ] {
        let [finding] = &check(Path::new(path), TEST)[..] else {
            panic!("expected one finding for {path}");
        };
        assert_eq!(finding.rule, RULE);
        assert_eq!(finding.line, None);
        assert!(
            finding.message.contains("tests/suite"),
            "{}",
            finding.message
        );
    }
}

#[test]
fn the_suite_its_modules_and_other_trees_pass() {
    for path in [
        "crates/core/tests/suite/main.rs",
        "crates/core/tests/suite/time.rs",
        "crates/server/tests/suite/api/mod.rs",
        "crates/server/tests/suite/api/main.rs",
        "crates/core/tests/fixtures/make.rs",
        "crates/core/src/main.rs",
        "tools/lint/tests/printing.rs",
        "app/tests/panels/main.rs",
    ] {
        assert!(check(Path::new(path), TEST).is_empty(), "{path}");
    }
}

#[test]
fn an_allow_in_the_opening_comments_excuses_it() {
    let excused = "//! A test.\n// lint: allow(test-binaries) it must run alone, in its own process.\n\n#[test]\nfn a() {}\n";
    assert!(check(Path::new("crates/core/tests/time.rs"), excused).is_empty());
    let bare = "//! A test.\n// lint: allow(test-binaries)\n";
    assert_eq!(check(Path::new("crates/core/tests/time.rs"), bare).len(), 1);
}
