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

/// The variables git sets for a hook, and a caller may export, that point a
/// git command at some repository other than the one in its directory.
const REDIRECTS: [&str; 6] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
    "GIT_PREFIX",
];

/// `git`, asking about the repository its working directory is in and no
/// other.
///
/// The pre-commit hook runs the lint and its tests with `GIT_DIR` and
/// `GIT_INDEX_FILE` set. A `git init` that inherited them re-initialised the
/// operator's own repository as bare (2026-10-15, the first commit after
/// #1282), and a `check-ignore` that inherited them would ask that repository
/// rather than `root`.
pub fn git() -> Command {
    let mut command = Command::new("git");
    for name in REDIRECTS {
        command.env_remove(name);
    }
    command
}

/// The paths among `paths` (relative to `root`) that git would leave out of
/// a commit, or none when `root` is not a git checkout.
pub fn ignored(root: &Path, paths: &[&Path]) -> Vec<PathBuf> {
    let child = git()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_never_inherits_a_redirect() {
        let command = git();
        let removed: Vec<_> = command
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
            assert!(removed.iter().any(|r| r == name), "{name} is inherited");
        }
    }
}
