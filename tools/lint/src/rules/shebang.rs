//! A script with a shebang is executable, and an executable is a script.
//!
//! #1283 committed `tools/ci-postgres` as mode `100644`. It starts with `#!`
//! and CI runs it directly, so CI failed with `Permission denied` — and
//! nothing local had caught it, because the hook and the gates run scripts
//! through an interpreter or not at all. So the mode git commits is read from
//! the index (`git ls-files -s`): a tracked file whose first line is a shebang
//! must be `100755`, and a `100755` file that is not binary must have one.
//!
//! Every tracked file is asked, not only the code files the size gate walks:
//! a shell script has no extension to classify it by. A Rust file opening
//! with an inner attribute (`#![…]`) is not a shebang. A tree that is not a
//! git checkout has no index to read, so the rule is silent there.
//!
//! An exception is a `lint: allow(shebang) <why>` comment in the run of
//! comment lines the file opens with.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::Finding;
use super::allow::{grants, is_comment};
use super::ignored::git;

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "shebang";

/// How much of a file is read: enough for its opening comments, and the
/// span git itself searches for a NUL byte to call a file binary.
const HEAD: u64 = 8000;

/// The mode git gives an executable file.
const EXECUTABLE: &str = "100755";

/// The mode git gives any other regular file.
const REGULAR: &str = "100644";

/// Every tracked regular file under `root`, with whether its index mode is
/// executable; none when `root` is not a git checkout.
pub fn tracked(root: &Path) -> Vec<(PathBuf, bool)> {
    let Ok(output) = git()
        .args(["ls-files", "--stage", "-z"])
        .current_dir(root)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter_map(|entry| {
            // `<mode> <object> <stage>\t<path>`
            let (meta, path) = entry.split_once('\t')?;
            let executable = match meta.split(' ').next()? {
                EXECUTABLE => true,
                REGULAR => false,
                // A symlink or a submodule has no mode of its own to get wrong.
                _ => return None,
            };
            Some((PathBuf::from(path), executable))
        })
        .collect()
}

/// What is off about a file opening with `head`, at the given mode, if anything.
pub fn verdict(head: &[u8], executable: bool) -> Option<&'static str> {
    let shebang = head.starts_with(b"#!") && !head.starts_with(b"#![");
    let binary = head.contains(&0);
    if shebang && !executable {
        Some("a shebang on a file git does not mark executable: `git update-index --chmod=+x`")
    } else if executable && !shebang && !binary {
        Some(
            "executable with no shebang: `git update-index --chmod=-x`, \
             or give it the `#!` line it is run with",
        )
    } else {
        None
    }
}

/// Whether the comment lines `head` opens with (its shebang among them)
/// grant an allow.
pub fn excused(head: &[u8]) -> bool {
    String::from_utf8_lossy(head)
        .lines()
        .take_while(|line| line.starts_with("#!") || is_comment(line))
        .any(|line| grants(line, RULE))
}

/// The first [`HEAD`] bytes of `path`, or none when it cannot be read (a
/// tracked file deleted from the working tree has nothing to judge).
fn head(path: &Path) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(HEAD)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

/// A finding for every tracked file under `root` whose mode and first line
/// disagree.
pub fn check(root: &Path) -> Vec<Finding> {
    tracked(root)
        .into_iter()
        .filter_map(|(path, executable)| {
            let head = head(&root.join(&path))?;
            let message = verdict(&head, executable)?;
            (!excused(&head)).then(|| Finding {
                path,
                line: None,
                rule: RULE,
                message: message.into(),
            })
        })
        .collect()
}
