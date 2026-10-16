//! A script's mode is read from the index: a shebang at `100644` is found,
//! an executable with none is found unless it is binary, and the right
//! pairings, a Rust inner attribute and an inline allow all pass.

use scorsese_lint::check_tree;
use scorsese_lint::rules::ignored::git;
use scorsese_lint::rules::shebang::{RULE, verdict};
use std::fs;
use std::path::{Path, PathBuf};

/// A throwaway git checkout with `files` staged, each `(path, bytes,
/// executable)`, rebuilt from scratch each run.
fn checkout(name: &str, files: &[(&str, &[u8], bool)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("mkdir");
    // `git()`, never a bare `Command`: it strips every `GIT_*` redirect, so a
    // run from the pre-commit hook touches this repository and no other (#1312).
    let run = |args: &[&str]| {
        let status = git()
            .args(args)
            .current_dir(&root)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    };
    run(&["init", "--quiet"]);
    for (path, bytes, executable) in files {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
        fs::write(&full, bytes).expect("write");
        run(&["add", path]);
        // The index, not the disk: what git commits is what CI runs.
        let mode = if *executable {
            "--chmod=+x"
        } else {
            "--chmod=-x"
        };
        run(&["update-index", mode, path]);
    }
    root
}

/// The paths the `shebang` rule reports under `root`.
fn found(root: &Path) -> Vec<PathBuf> {
    check_tree(root)
        .expect("walk")
        .into_iter()
        .filter(|finding| finding.rule == RULE)
        .map(|finding| finding.path)
        .collect()
}

#[test]
fn a_shebang_not_marked_executable_is_found() {
    // #1283's shape: a script CI runs directly, committed at 100644.
    let root = checkout(
        "shebang_not_executable",
        &[
            ("tools/ci-postgres", b"#!/bin/sh\nexit 0\n", false),
            ("tools/with-chrome", b"#!/bin/sh\nexit 0\n", true),
        ],
    );
    assert_eq!(found(&root), [PathBuf::from("tools/ci-postgres")]);
}

#[test]
fn an_executable_text_file_without_a_shebang_is_found() {
    let root = checkout(
        "shebang_missing",
        &[
            ("fonts/OFL.txt", b"Copyright\n", true),
            ("tools/blob", b"\x7fELF\0\0\0", true),
            ("notes.md", b"# Notes\n", false),
        ],
    );
    assert_eq!(found(&root), [PathBuf::from("fonts/OFL.txt")]);
}

#[test]
fn an_inner_attribute_and_an_allow_pass() {
    let root = checkout(
        "shebang_passes",
        &[
            ("src/lib.rs", b"#![forbid(unsafe_code)]\n", false),
            (
                "scripts/sourced.sh",
                b"#!/bin/sh\n# lint: allow(shebang) only ever sourced, never run\n",
                false,
            ),
        ],
    );
    assert!(found(&root).is_empty());
}

#[test]
fn the_verdict_pairs_mode_and_first_line() {
    assert!(verdict(b"#!/usr/bin/env python3\n", false).is_some());
    assert!(verdict(b"#!/usr/bin/env python3\n", true).is_none());
    assert!(verdict(b"plain\n", true).is_some());
    assert!(verdict(b"plain\n", false).is_none());
    assert!(verdict(b"#![doc]\n", false).is_none());
    assert!(verdict(b"\0binary", true).is_none());
}

#[test]
fn a_tree_outside_git_reports_nothing() {
    let root = std::env::temp_dir().join(format!("scorsese-lint-shebang-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("mkdir");
    fs::write(root.join("run"), "#!/bin/sh\n").expect("write");
    let reported = found(&root);
    let _ = fs::remove_dir_all(&root);
    assert!(reported.is_empty(), "{reported:?}");
}
