//! What you push is what CI builds (`.claude/skills/issue-batch/cloud-brief.md`).
//!
//! A code file `.gitignore` swallows passes every gate on the machine that has
//! it and is missing from the commit: on 2026-10-07 a bare `cache/` rule
//! dropped `crates/render/src/page/cache/pieces.rs`, #872's gates passed on
//! the coder's disk, and the pushed tree could not compile. So every code file
//! the lint walks is asked of git: an ignored one is a finding.
//!
//! A tracked file is never reported (git commits it whatever the rules say),
//! and a tree that is not a git checkout has nothing to ask, so the rule is
//! silent there — the lint's own tests build such trees.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::Finding;

/// The rule's name. It takes no allow: an ignored code file is never right.
pub const RULE: &str = "ignored";

/// The paths among `paths` (relative to `root`) that git would leave out of
/// a commit, or none when `root` is not a git checkout.
pub fn ignored(root: &Path, paths: &[&Path]) -> Vec<PathBuf> {
    let child = Command::new("git")
        .args(["check-ignore", "--stdin"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return Vec::new();
    };
    if let Some(mut stdin) = child.stdin.take() {
        let list: String = paths
            .iter()
            .map(|path| format!("{}\n", path.display()))
            .collect();
        // A failed write shows up as git's own failure below.
        let _ = stdin.write_all(list.as_bytes());
    }
    let Ok(output) = child.wait_with_output() else {
        return Vec::new();
    };
    // 0: some are ignored; 1: none are; anything else: not a checkout.
    if output.status.code() != Some(0) {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
        .collect()
}

/// A finding for every code file under `root` that git ignores.
pub fn check(root: &Path, paths: &[&Path]) -> Vec<Finding> {
    ignored(root, paths)
        .into_iter()
        .map(|path| Finding {
            path,
            line: None,
            rule: RULE,
            message: "code that .gitignore leaves out of every commit: narrow the rule \
                      swallowing it, or the push will not build"
                .into(),
        })
        .collect()
}
