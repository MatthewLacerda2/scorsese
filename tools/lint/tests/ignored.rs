//! What you push is what CI builds: a code file `.gitignore` swallows is
//! found, one it does not is not, and a tree outside git has nothing to ask.

use scorsese_lint::check_tree;
use scorsese_lint::rules::ignored::{RULE, git};
use std::fs;
use std::path::{Path, PathBuf};

/// A throwaway git checkout holding `files`, rebuilt from scratch each run.
fn checkout(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("mkdir");
    // `git()`, never a bare `Command`: run from the pre-commit hook, a plain
    // `git init` inherits `GIT_DIR` and re-initialises the real repository.
    let status = git()
        .args(["init", "--quiet"])
        .current_dir(&root)
        .status()
        .expect("git runs");
    assert!(status.success(), "git init");
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, text).expect("write");
    }
    root
}

/// The paths the `ignored` rule reports under `root`.
fn ignored(root: &Path) -> Vec<PathBuf> {
    check_tree(root)
        .expect("walk")
        .into_iter()
        .filter(|finding| finding.rule == RULE)
        .map(|finding| finding.path)
        .collect()
}

#[test]
fn a_source_file_an_unanchored_rule_swallows_is_found() {
    // PR #872's shape: a bare `cache/` matched a module folder of that name.
    let root = checkout(
        "ignored_swallowed",
        &[
            (".gitignore", "cache/\n"),
            ("crates/render/src/page/cache/pieces.rs", "fn a() {}\n"),
            ("crates/render/src/page/mod.rs", "fn b() {}\n"),
        ],
    );
    assert_eq!(
        ignored(&root),
        [PathBuf::from("crates/render/src/page/cache/pieces.rs")]
    );
}

#[test]
fn an_anchored_rule_leaves_a_module_of_the_same_name_alone() {
    let root = checkout(
        "ignored_anchored",
        &[
            (".gitignore", "/target/\n/debug/\n"),
            ("crates/core/src/debug/mod.rs", "fn a() {}\n"),
            ("crates/core/src/target.rs", "fn b() {}\n"),
        ],
    );
    assert!(ignored(&root).is_empty());
}

#[test]
fn a_tree_outside_git_reports_nothing() {
    // The system temp directory, not CARGO_TARGET_TMPDIR: that one is inside
    // this checkout, under a `.gitignore`d target directory.
    let root = std::env::temp_dir().join(format!("scorsese-lint-no-git-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("mkdir");
    fs::write(root.join("src/a.rs"), "fn a() {}\n").expect("write");
    let found = ignored(&root);
    let _ = fs::remove_dir_all(&root);
    assert!(found.is_empty(), "{found:?}");
}
