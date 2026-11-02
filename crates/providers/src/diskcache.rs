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

/// The folder `folder` below `base`, made with `base` if they are not there,
/// and `folder` never a link (#1715): a `.scor` received from someone else may
/// carry one in its `cache/`, and nothing kept there is written where it
/// leads. `base` itself is the caller's — on the web a project's `cache/` is
/// a link to where its captures are kept.
///
/// # Errors
///
/// If `base` or `folder` cannot be made, or `folder` is a link or a file.
pub(crate) fn folder(base: &Path, folder: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(base)?;
    scorsese_core::write::folder_below(base, Path::new(folder))
}

/// Keeps `entry` as `key`'s answer in the folder `folder` below `base`,
/// making it through [`folder`] if it is the first, and atomically.
///
/// The error is the file's path and what went wrong there, so a caller with
/// its own error type wraps both without working the path out again.
pub(crate) fn write<T: Serialize>(
    base: &Path,
    folder: &str,
    key: &str,
    entry: &T,
) -> Result<(), (PathBuf, std::io::Error)> {
    let path = file(&base.join(folder), key);
    let kept = self::folder(base, folder)
        .and_then(|_| {
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
        let base = crate::scratch::fresh("scorsese-diskcache-named", "").join("deep");
        let dir = base.join("pages");
        write(&base, "pages", "cats p=1", &entry("cats p=1", 3)).unwrap();
        let path = dir.join(format!("{}.json", hash_bytes(b"cats p=1")));
        let text = std::fs::read_to_string(path).unwrap();
        assert_eq!(text, "{\n  \"listing\": \"cats p=1\",\n  \"count\": 3\n}");
        assert_eq!(read(&dir, "cats p=1"), Some(entry("cats p=1", 3)));
    }

    #[test]
    fn two_keys_keep_two_entries_and_a_rewrite_replaces_one() {
        let base = crate::scratch::fresh("scorsese-diskcache-keys", "");
        let dir = base.join("pages");
        write(&base, "pages", "a", &entry("a", 1)).unwrap();
        write(&base, "pages", "b", &entry("b", 2)).unwrap();
        write(&base, "pages", "a", &entry("a", 5)).unwrap();
        assert_eq!(read(&dir, "a"), Some(entry("a", 5)));
        let mut every: Vec<Entry> = all(&dir);
        every.sort_by_key(|one| one.count);
        assert_eq!(every, vec![entry("b", 2), entry("a", 5)]);
    }

    #[test]
    fn missing_and_unreadable_entries_are_misses() {
        let base = crate::scratch::fresh("scorsese-diskcache-misses", "");
        let dir = base.join("pages");
        assert_eq!(read::<Entry>(&dir, "a"), None);
        assert!(all::<Entry>(&dir).is_empty());
        write(&base, "pages", "a", &entry("a", 1)).unwrap();
        std::fs::write(file(&dir, "b"), "{ half").unwrap();
        assert_eq!(read::<Entry>(&dir, "b"), None);
        assert_eq!(all::<Entry>(&dir), vec![entry("a", 1)]);
    }

    #[test]
    fn a_failed_write_names_the_file() {
        let base = crate::scratch::fresh("scorsese-diskcache-blocked", "");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("pages"), "a file where the folder should be").unwrap();
        let (path, _) = write(&base, "pages", "a", &entry("a", 1)).unwrap_err();
        assert_eq!(path, file(&base.join("pages"), "a"));
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_that_is_a_link_is_never_written_through() {
        let base = crate::scratch::fresh("scorsese-diskcache-linked", "");
        let outside = crate::scratch::fresh("scorsese-diskcache-outside", "");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, base.join("pages")).unwrap();
        let (path, _) = write(&base, "pages", "a", &entry("a", 1)).unwrap_err();
        assert_eq!(path, file(&base.join("pages"), "a"));
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    }
}
