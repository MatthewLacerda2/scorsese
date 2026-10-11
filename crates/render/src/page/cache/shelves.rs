//! A slot's shelves: one per set of places its page was told (#810), and only
//! the [`SHELVES`] written most recently kept.

use std::path::{Path, PathBuf};

use super::super::told::Told;

/// How many shelves a slot keeps: a page placed this many times in one edit,
/// each told different places, is the most that is never captured twice.
pub(super) const SHELVES: usize = 8;

/// The shelf in `slot` for captures told `told`.
pub(crate) fn shelf(slot: &Path, told: &Told) -> PathBuf {
    slot.join(told.folder())
}

/// Every shelf of `slot` but the [`SHELVES`] written most recently: the
/// shelves to drop. Dropping them is the caller's, which knows which still
/// hold a file somebody was handed.
pub(crate) fn retired(slot: &Path) -> Vec<PathBuf> {
    let mut shelves: Vec<(std::time::SystemTime, PathBuf)> = all(slot)
        .into_iter()
        .map(|shelf| {
            let written = std::fs::metadata(&shelf).and_then(|m| m.modified());
            (written.unwrap_or(std::time::UNIX_EPOCH), shelf)
        })
        .collect();
    shelves.sort_by_key(|(written, _)| std::cmp::Reverse(*written));
    shelves
        .into_iter()
        .skip(SHELVES)
        .map(|(_, shelf)| shelf)
        .collect()
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
        assert_eq!(
            retired(&root),
            [root.join("told-01"), root.join("told-00")],
            "the oldest go, newest first"
        );
        assert!(
            !retired(&root).contains(&root.join("fonts")),
            "only shelves are retired"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
