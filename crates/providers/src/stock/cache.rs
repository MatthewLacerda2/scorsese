//! What a search found, kept 24 hours — as long as Pixabay asks it be, and
//! as long as its picture links stay valid.
//!
//! Under `cache/` and not `generated/`: a result is free to find again and
//! worth nothing on another machine. One file per page of one query, named for
//! the hash of the query, so two searches never overwrite each other. Previews
//! and the small renditions `look` reads sit beside the pages and expire with
//! them, so a cache shared by many projects — the web app's — stays bounded.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use scorsese_core::{CACHE_DIR, Timestamp, hash_bytes};

use super::candidate::{Candidate, Medium};
use super::library::{Page, StockError};

/// How long a cached page is served before the vendor is asked again.
pub const FRESH_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// Where a project's stock cache lives inside it.
pub fn cache_dir(root: &Path) -> PathBuf {
    root.join(CACHE_DIR).join("stock")
}

/// One page of one query, as it was when it was read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Cached {
    /// Which query and page this is, in words, so the file explains itself.
    pub(crate) listing: String,
    /// When it was read, as seconds since the epoch.
    pub(crate) fetched_unix: i64,
    /// What was on it.
    pub(crate) candidates: Vec<Candidate>,
    /// How many results the vendor would hand out in all.
    pub(crate) total: u64,
}

impl Cached {
    /// Whether this was read within [`FRESH_FOR`] of `now`.
    fn fresh(&self, now: i64) -> bool {
        let age = now - self.fetched_unix;
        (0..i64::try_from(FRESH_FOR.as_secs()).unwrap_or(i64::MAX)).contains(&age)
    }
}

/// The folder pages are kept in.
fn pages(cache: &Path) -> PathBuf {
    cache.join("pages")
}

/// The file `key`'s page is kept in.
fn file(cache: &Path, key: &str) -> PathBuf {
    pages(cache).join(format!("{}.json", hash_bytes(key.as_bytes())))
}

/// The cached page for `key`, if a fresh, readable one is there. Anything
/// else — missing, stale, corrupt — is a miss.
pub(crate) fn read(cache: &Path, key: &str) -> Option<Page> {
    let now = Timestamp::unix_now()?;
    let text = std::fs::read_to_string(file(cache, key)).ok()?;
    let cached: Cached = serde_json::from_str(&text).ok()?;
    cached.fresh(now).then_some(Page {
        candidates: cached.candidates,
        total: cached.total,
    })
}

/// Keeps `page` as `key`'s answer, atomically.
pub(crate) fn write(cache: &Path, key: &str, page: &Page) -> Result<(), StockError> {
    let path = file(cache, key);
    let io = |source| StockError::Io {
        path: path.clone(),
        source,
    };
    std::fs::create_dir_all(pages(cache)).map_err(io)?;
    let cached = Cached {
        listing: key.to_owned(),
        fetched_unix: Timestamp::unix_now().unwrap_or_default(),
        candidates: page.candidates.clone(),
        total: page.total,
    };
    let text = serde_json::to_string_pretty(&cached)
        .map_err(|error| io(std::io::Error::other(error.to_string())))?;
    scorsese_core::write::atomically(&path, text.as_bytes()).map_err(io)
}

/// The candidate `medium` `id` names on any fresh cached page — what a call
/// after a search resolves an id against without asking the vendor again.
///
/// No network and no key, so a picker on the web (#901) can hold an id to
/// the results a search really returned.
pub fn find_cached(cache: &Path, medium: Medium, id: u64) -> Option<Candidate> {
    let now = Timestamp::unix_now()?;
    let entries = std::fs::read_dir(pages(cache)).ok()?;
    entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Cached>(&text).ok())
        .filter(|cached| cached.fresh(now))
        .flat_map(|cached| cached.candidates)
        .find(|one| one.medium == medium && one.id == id)
}

/// Removes every cached file older than [`FRESH_FOR`]. Failures are ignored:
/// a cache is an optimisation, and a file that would not go is only disk.
pub(crate) fn prune(cache: &Path) {
    let Some(cutoff) = SystemTime::now().checked_sub(FRESH_FOR) else {
        return;
    };
    for folder in ["pages", "previews", "looks"] {
        let Ok(entries) = std::fs::read_dir(cache.join(folder)) else {
            continue;
        };
        for entry in entries.flatten() {
            let old = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .is_ok_and(|at| at < cutoff);
            if old {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
