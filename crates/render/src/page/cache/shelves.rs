//! A slot's shelves: one per set of places its page was told (#810), and only
//! the [`SHELVES`] written most recently kept.

use std::path::{Path, PathBuf};

use super::super::told::Told;

/// How many shelves a slot keeps: a page placed this many times in one edit,
/// each told different places, is the most that is never captured twice.
const SHELVES: usize = 8;

/// The shelf in `slot` for captures told `told`.
pub(crate) fn shelf(slot: &Path, told: &Told) -> PathBuf {
    slot.join(told.folder())
}

/// Drops every shelf of `slot` but the [`SHELVES`] written most recently,
/// `kept` always among them.
pub(crate) fn retire(slot: &Path, kept: &Path) {
    let mut others: Vec<(std::time::SystemTime, PathBuf)> = all(slot)
        .into_iter()
        .filter(|shelf| shelf != kept)
        .map(|shelf| {
            let written = std::fs::metadata(&shelf).and_then(|m| m.modified());
            (written.unwrap_or(std::time::UNIX_EPOCH), shelf)
        })
        .collect();
    others.sort_by_key(|(written, _)| std::cmp::Reverse(*written));
    for (_, shelf) in others.into_iter().skip(SHELVES - 1) {
        let _ = std::fs::remove_dir_all(shelf);
    }
}

/// Every shelf in `slot`.
pub(super) fn all(slot: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(slot) else {
        return Vec::new();
    };
    let mut shelves: Vec<PathBuf> = read
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("told-"))
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    shelves.sort();
    shelves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_keeps_only_the_shelves_written_last() {
        let root = std::env::temp_dir().join(format!("scorsese-shelves-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for i in 0..SHELVES + 2 {
            std::fs::create_dir_all(root.join(format!("told-{i:02}"))).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::fs::create_dir_all(root.join("fonts")).unwrap();
        let kept = root.join("told-00");
        retire(&root, &kept);
        let left = all(&root);
        assert_eq!(left.len(), SHELVES);
        assert!(left.contains(&kept), "the shelf just written stays");
        assert!(!left.contains(&root.join("told-01")), "the oldest go");
        assert!(root.join("fonts").is_dir(), "only shelves are retired");
        let _ = std::fs::remove_dir_all(&root);
    }
}
