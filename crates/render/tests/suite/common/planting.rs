//! Links planted where a writer is about to write (#1672).

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// How many of the next scratch names get a link: more than one call takes,
/// so a name another call in this process took does not slip past.
const PLANTED: u64 = 16;

/// A link to `target` at each of the next names
/// [`scorsese_core::write::scratch_beside`] gives `path` with `suffix`, and at
/// each name with every one of `siblings` added after it. The counter's
/// current number is read off the name it hands out; the ones after it are
/// what a writer will try.
pub(crate) fn plant(path: &Path, suffix: &str, siblings: &[&str], target: &Path) -> Vec<PathBuf> {
    let seen = scorsese_core::write::scratch_beside(path, suffix).expect("a scratch name");
    let seen = seen.file_name().expect("a name").to_string_lossy();
    let (name, number) = seen
        .strip_suffix(&format!(".{suffix}"))
        .and_then(|s| s.rsplit_once('-'))
        .expect("<name>.<pid>-<n>.<suffix>");
    let number: u64 = number.parse().expect("a number");
    let dir = path.parent().expect("a folder");
    let mut planted = Vec::new();
    for k in 1..=PLANTED {
        for sibling in std::iter::once("").chain(siblings.iter().copied()) {
            let link = dir.join(format!("{name}-{}.{suffix}{sibling}", number + k));
            symlink(target, &link).expect("a link");
            planted.push(link);
        }
    }
    planted
}
