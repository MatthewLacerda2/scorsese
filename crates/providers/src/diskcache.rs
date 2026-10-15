//! A folder of JSON files, one per key: the mechanism under every listing
//! this crate keeps in a project's `cache/` (#1279).
//!
//! The voice listings and the stock search pages are kept the same way — a
//! file named for the hash of its key, so two queries never overwrite each
//! other; written atomically as pretty JSON, so a half-written file is never
//! read as a short complete one; and read back with every failure counted as
//! a miss, because a cache is an optimisation. That is written here once.
//!
//! **What is fresh is not decided here.** Each caller holds its own rule
//! beside its own entry: stock serves a page for 24 hours and treats one from
//! the future as a miss, while voices keeps a stale listing on purpose, to
//! fall back on when the vendor cannot be reached, and reads a clock that
//! moved backwards as age zero. A read here hands back whatever parsed, and
//! the caller filters it.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use scorsese_core::hash_bytes;

/// The file `key`'s entry is kept in, inside `dir`.
fn file(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{}.json", hash_bytes(key.as_bytes())))
}

/// The entry kept for `key` in `dir`, if there is a readable one. Missing,
/// half-written or no longer the shape `T` reads are all a miss.
pub(crate) fn read<T: DeserializeOwned>(dir: &Path, key: &str) -> Option<T> {
    let text = std::fs::read_to_string(file(dir, key)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Keeps `entry` as `key`'s answer in `dir`, creating `dir` if it is the
/// first, and atomically.
///
/// The error is the file's path and what went wrong there, so a caller with
/// its own error type wraps both without working the path out again.
pub(crate) fn write<T: Serialize>(
    dir: &Path,
    key: &str,
    entry: &T,
) -> Result<(), (PathBuf, std::io::Error)> {
    let path = file(dir, key);
    let kept = std::fs::create_dir_all(dir)
        .and_then(|()| {
            serde_json::to_string_pretty(entry)
                .map_err(|error| std::io::Error::other(error.to_string()))
        })
        .and_then(|text| scorsese_core::write::atomically(&path, text.as_bytes()));
    kept.map_err(|error| (path, error))
}

/// Every readable entry in `dir`, whichever key wrote it: what a caller with
/// no key to spell asks. A folder that is not there is an empty cache, and an
/// entry that will not parse is skipped, as [`read`] would miss it.
pub(crate) fn all<T: DeserializeOwned>(dir: &Path) -> Vec<T> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str(&text).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Entry {
        listing: String,
        count: u32,
    }

    fn entry(listing: &str, count: u32) -> Entry {
        Entry {
            listing: listing.to_owned(),
            count,
        }
    }

    #[test]
    fn an_entry_is_named_for_its_key_and_written_as_pretty_json() {
        let dir = crate::scratch::fresh("scorsese-diskcache-named", "").join("deep");
        write(&dir, "cats p=1", &entry("cats p=1", 3)).unwrap();
        let path = dir.join(format!("{}.json", hash_bytes(b"cats p=1")));
        let text = std::fs::read_to_string(path).unwrap();
        assert_eq!(text, "{\n  \"listing\": \"cats p=1\",\n  \"count\": 3\n}");
        assert_eq!(read(&dir, "cats p=1"), Some(entry("cats p=1", 3)));
    }

    #[test]
    fn two_keys_keep_two_entries_and_a_rewrite_replaces_one() {
        let dir = crate::scratch::fresh("scorsese-diskcache-keys", "");
        write(&dir, "a", &entry("a", 1)).unwrap();
        write(&dir, "b", &entry("b", 2)).unwrap();
        write(&dir, "a", &entry("a", 5)).unwrap();
        assert_eq!(read(&dir, "a"), Some(entry("a", 5)));
        let mut every: Vec<Entry> = all(&dir);
        every.sort_by_key(|one| one.count);
        assert_eq!(every, vec![entry("b", 2), entry("a", 5)]);
    }

    #[test]
    fn missing_and_unreadable_entries_are_misses() {
        let dir = crate::scratch::fresh("scorsese-diskcache-misses", "");
        assert_eq!(read::<Entry>(&dir, "a"), None);
        assert!(all::<Entry>(&dir).is_empty());
        write(&dir, "a", &entry("a", 1)).unwrap();
        std::fs::write(file(&dir, "b"), "{ half").unwrap();
        assert_eq!(read::<Entry>(&dir, "b"), None);
        assert_eq!(all::<Entry>(&dir), vec![entry("a", 1)]);
    }

    #[test]
    fn a_failed_write_names_the_file() {
        let dir = crate::scratch::fresh("scorsese-diskcache-blocked", "");
        std::fs::write(&dir, "a file where the folder should be").unwrap();
        let (path, _) = write(&dir, "a", &entry("a", 1)).unwrap_err();
        assert_eq!(path, file(&dir, "a"));
    }
}
