//! The classifier decides which cap a file is held to, and every path in the
//! repo goes through it. If it silently starts answering `Source` for test
//! files, the gate keeps passing and stops gating — so the rules are pinned
//! here one at a time.

use std::path::Path;

use scorsese_lint::classify::{Kind, SOURCE_LIMIT, TEST_LIMIT, classify, is_skipped_dir};

fn kind_of(path: &str) -> Option<Kind> {
    classify(Path::new(path))
}

#[test]
fn the_two_limits_are_the_ones_claude_md_states() {
    assert_eq!(SOURCE_LIMIT, 300);
    assert_eq!(TEST_LIMIT, 150);
    assert_eq!(Kind::Source.limit(), SOURCE_LIMIT);
    assert_eq!(Kind::Test.limit(), TEST_LIMIT);
}

#[test]
fn a_crate_source_file_is_source() {
    assert_eq!(kind_of("crates/core/src/asset.rs"), Some(Kind::Source));
    assert_eq!(kind_of("crates/render/src/plan/mod.rs"), Some(Kind::Source));
}

#[test]
fn an_integration_test_is_a_test_file() {
    assert_eq!(kind_of("crates/core/tests/round_trip.rs"), Some(Kind::Test));
    assert_eq!(
        kind_of("crates/render/tests/plan/embedded/mod.rs"),
        Some(Kind::Test)
    );
}

#[test]
fn a_helper_module_under_tests_is_a_test_file_too() {
    // `common/` holds no `#[test]` of its own, but it is compiled into the
    // test target and read while reading the tests. Same cap.
    assert_eq!(
        kind_of("crates/render/tests/common/audio/fixtures.rs"),
        Some(Kind::Test)
    );
}

#[test]
fn the_gates_own_tests_are_test_files() {
    assert_eq!(kind_of("tools/lint/tests/classify.rs"), Some(Kind::Test));
    assert_eq!(kind_of("tools/lint/src/classify.rs"), Some(Kind::Source));
}

#[test]
fn test_infrastructure_compiled_as_a_library_is_source() {
    // `crates/golden` exists only to serve tests, but it is a library crate
    // that other crates depend on. The cap follows the file's role in the
    // build, not its subject matter.
    assert_eq!(
        kind_of("crates/golden/src/harness/mod.rs"),
        Some(Kind::Source)
    );
}

#[test]
fn it_is_the_directory_named_tests_that_decides_not_the_file_name() {
    // A `tests.rs` next to the code it tests is an inline-test module living
    // in a source tree: source, 300 lines.
    assert_eq!(kind_of("crates/core/src/tests.rs"), Some(Kind::Source));
    // And any directory named `tests`, at any depth, makes what is under it
    // test files.
    assert_eq!(kind_of("tests/smoke.rs"), Some(Kind::Test));
    assert_eq!(kind_of("a/b/tests/c/d.rs"), Some(Kind::Test));
}

#[test]
fn a_build_script_is_source() {
    assert_eq!(kind_of("crates/render/build.rs"), Some(Kind::Source));
}

#[test]
fn only_rust_is_measured() {
    // Prose and data are long for reasons splitting them would not improve.
    for path in [
        "CLAUDE.md",
        "Cargo.toml",
        "docs/project-format.md",
        "crates/core/tests/fixtures/project.json",
        "LICENSE",
    ] {
        assert_eq!(kind_of(path), None, "{path}");
    }
}

#[test]
fn build_output_and_hidden_directories_are_not_measured() {
    for path in [
        "target/debug/build/scorsese-core-1/out/generated.rs",
        "tools/lint/target/debug/build/x/out/table.rs",
        ".git/hooks/thing.rs",
        ".claude/worktrees/agent-1/crates/core/src/asset.rs",
        "some.scor/generated/x.rs",
        "some.scor/cache/x.rs",
        "node_modules/whatever/x.rs",
    ] {
        assert_eq!(kind_of(path), None, "{path}");
    }
}

#[test]
fn the_skip_list_is_directories_only_not_files_that_look_like_them() {
    assert!(is_skipped_dir("target"));
    assert!(is_skipped_dir(".github"));
    assert!(!is_skipped_dir("tests"));
    assert!(!is_skipped_dir("src"));
    // A *file* named after a skipped directory is still measured — only the
    // directory components of a path are consulted.
    assert_eq!(kind_of("crates/core/src/target.rs"), Some(Kind::Source));
}
