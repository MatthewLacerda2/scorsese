//! The one place a file may be longer than its kind's cap: an exact path, a
//! raised number and the reason, listed in `scorsese_lint::raised` (#1783).
//! A raised cap is still a cap, it reaches no other path, and an entry whose
//! file has gone is stale and fails.

use std::fs;
use std::path::{Path, PathBuf};

use scorsese_lint::raised::{RAISED, limit_for};
use scorsese_lint::{Kind, SOURCE_LIMIT, check_dir};

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

const ENUM: &str = "crates/zimmer/src/error/mod.rs";

#[test]
fn the_listed_file_may_pass_the_source_cap() {
    let raised = limit_for(Path::new(ENUM), Kind::Source);
    assert!(raised > SOURCE_LIMIT);
    let root = tree("raised_within", &[(ENUM, SOURCE_LIMIT + 50)]);
    let report = check_dir(&root).expect("scan");
    assert!(report.is_clean(), "{:?}", report.violations);
}

#[test]
fn a_raised_cap_is_still_a_cap_and_the_message_says_which() {
    let raised = limit_for(Path::new(ENUM), Kind::Source);
    let root = tree("raised_over", &[(ENUM, raised + 1)]);
    let report = check_dir(&root).expect("scan");
    let [violation] = &report.violations[..] else {
        panic!("expected one violation, got {:?}", report.violations);
    };
    assert_eq!(violation.limit, raised);
    let message = violation.to_string();
    assert!(message.contains(&raised.to_string()), "{message}");
    assert!(message.contains("raised"), "{message}");
}

#[test]
fn the_raise_reaches_no_other_path() {
    let root = tree(
        "raised_elsewhere",
        &[
            ("crates/zimmer/src/error/wording/mod.rs", SOURCE_LIMIT + 1),
            ("crates/other/src/error/mod.rs", SOURCE_LIMIT + 1),
        ],
    );
    let report = check_dir(&root).expect("scan");
    assert_eq!(report.violations.len(), 2, "{:?}", report.violations);
    for violation in &report.violations {
        assert_eq!(violation.limit, SOURCE_LIMIT);
    }
}

/// Every entry names a file that exists in this repository, says why, and
/// raises a source cap: a test file is never raised.
#[test]
fn every_entry_is_live_and_explained() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(!RAISED.is_empty());
    for entry in RAISED {
        assert!(repo.join(entry.path).is_file(), "{} is gone", entry.path);
        assert!(entry.why.len() > 40, "{} needs its reason", entry.path);
        assert!(entry.limit > SOURCE_LIMIT, "{} raises nothing", entry.path);
        assert_eq!(
            scorsese_lint::classify(Path::new(entry.path)),
            Some(Kind::Source),
            "{} is not a source file",
            entry.path
        );
    }
}
