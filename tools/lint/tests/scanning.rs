//! Walking a tree end to end: what gets measured, what gets skipped, and what
//! the failure says.

use std::fs;
use std::path::{Path, PathBuf};

use scorsese_lint::{Kind, check_dir};

/// A throwaway tree under the test target dir, rebuilt from scratch each run.
fn tree(name: &str, files: &[(&str, usize)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    for (path, lines) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, "x\n".repeat(*lines)).expect("write");
    }
    root
}

#[test]
fn a_tree_within_the_limits_passes_and_says_what_it_measured() {
    let root = tree(
        "clean",
        &[
            ("crates/core/src/lib.rs", 300),
            ("crates/core/tests/round_trip.rs", 150),
            ("tools/lint/src/main.rs", 12),
        ],
    );

    let report = check_dir(&root).expect("scan");

    assert!(report.is_clean(), "{:?}", report.violations);
    // A run that measured nothing would otherwise be indistinguishable from a
    // run that measured everything and found it fine.
    assert_eq!(report.checked, 3);
}

#[test]
fn a_source_file_one_line_over_is_reported_against_the_source_limit() {
    let root = tree("long_source", &[("crates/core/src/lib.rs", 301)]);

    let report = check_dir(&root).expect("scan");

    let [violation] = &report.violations[..] else {
        panic!("expected one violation, got {:?}", report.violations);
    };
    assert_eq!(violation.path, Path::new("crates/core/src/lib.rs"));
    assert_eq!(violation.lines, 301);
    assert_eq!(violation.kind, Kind::Source);
}

#[test]
fn a_test_file_is_held_to_the_tighter_limit() {
    let root = tree(
        "long_test",
        &[
            ("crates/render/tests/plan/embedded.rs", 154),
            // The same length in a source tree is fine, which is the whole
            // reason the classifier exists.
            ("crates/render/src/plan.rs", 154),
        ],
    );

    let report = check_dir(&root).expect("scan");

    let [violation] = &report.violations[..] else {
        panic!("expected one violation, got {:?}", report.violations);
    };
    assert_eq!(violation.kind, Kind::Test);
    assert_eq!(violation.lines, 154);
}

#[test]
fn the_message_names_the_file_its_length_and_the_limit_it_broke() {
    let root = tree("message", &[("crates/render/tests/plan/embedded.rs", 154)]);

    let message = check_dir(&root).expect("scan").violations[0].to_string();

    assert!(
        message.contains("crates/render/tests/plan/embedded.rs"),
        "{message}"
    );
    assert!(message.contains("154"), "{message}");
    assert!(message.contains("150"), "{message}");
    assert!(message.contains("test"), "{message}");
}

#[test]
fn nothing_outside_the_gates_reach_is_walked_or_counted() {
    let root = tree(
        "skipped",
        &[
            ("crates/core/src/lib.rs", 10),
            ("target/debug/build/out/generated.rs", 5_000),
            (".git/objects/stray.rs", 5_000),
            ("docs/very-long.md", 5_000),
            ("crates/core/tests/fixtures/project.json", 5_000),
        ],
    );

    let report = check_dir(&root).expect("scan");

    assert!(report.is_clean(), "{:?}", report.violations);
    assert_eq!(report.checked, 1);
}

#[test]
fn violations_come_out_in_path_order() {
    let root = tree(
        "ordered",
        &[
            ("crates/z/src/lib.rs", 400),
            ("crates/a/src/lib.rs", 400),
            ("crates/m/src/lib.rs", 400),
        ],
    );

    let paths: Vec<_> = check_dir(&root)
        .expect("scan")
        .violations
        .iter()
        .map(|v| v.path.clone())
        .collect();

    assert_eq!(
        paths,
        [
            PathBuf::from("crates/a/src/lib.rs"),
            PathBuf::from("crates/m/src/lib.rs"),
            PathBuf::from("crates/z/src/lib.rs"),
        ]
    );
}
