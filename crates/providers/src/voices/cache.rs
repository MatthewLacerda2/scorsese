//! The listings, kept under the project's `cache/`.
//!
//! `cache/` and not `generated/`, and the distinction is the project format's
//! rather than a preference: `generated/` holds work somebody paid for and
//! deleting it loses money, while `cache/` is rebuildable and gitignored.
//! A voice list is the second kind exactly — free to fetch again, and worth
//! nothing to carry between machines.
//!
//! **A cache with no expiry is the bug this module was written to avoid.** The
//! whole reason voices are resolved at runtime is that they expire; a list
//! saved once and served for ever would reintroduce the hardcoded id with extra
//! steps, and the voice it kept showing would be one that no longer answers. So
//! an entry older than [`REFRESH_AFTER_DAYS`] is re-read, and every answer says
//! how old it is even when it is fresh.
//!
//! One file per query rather than one file for everything, named for the hash
//! of the query: the Voice Library is filterable, so *the list* is not a single
//! thing, and a shared file would have one search overwriting another's answer.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use scorsese_core::{CACHE_DIR, Timestamp, hash_bytes};

use super::catalogue::{More, Page, Voice};

/// How long a cached listing is served before it is fetched again.
///
/// A week. Long enough that browsing in a working session never touches the
/// network twice, short enough that a voice withdrawn on Monday is not still
/// being offered in a fortnight. It is a judgement rather than a rule — the
/// vendor publishes no schedule for retiring a voice — and the number is here,
/// named, so that it can be argued with.
pub const REFRESH_AFTER_DAYS: i64 = 7;

/// Where the listings live inside `cache/`.
const VOICES_DIR: &str = "voices";

/// One listing, as it was when it was read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Cached {
    /// Which listing this is, in words. Nothing reads it — the file name
    /// already identifies the query — but a hash makes for a directory nobody
    /// can look through, and this is the line that makes one of these files
    /// explain itself.
    pub(crate) listing: String,
    /// When it was read, as a person reads a date.
    pub(crate) fetched_at: Option<Timestamp>,
    /// The same instant as seconds since the epoch.
    ///
    /// Both, and not by oversight: the string is what makes the file readable,
    /// and the number is what makes the age arithmetic possible without parsing
    /// a formatted stamp back — which `scorsese-core` deliberately does not do,
    /// since a stamp there is a thing to record rather than to calculate with.
    pub(crate) fetched_unix: Option<i64>,
    /// What was in it.
    pub(crate) voices: Vec<Voice>,
    /// Whether the vendor had more than it sent, saved along with the voices.
    ///
    /// Not recomputed on the way out, because there is nothing left to
    /// recompute it from: the reply that said so is a week gone by then. A
    /// truncated search whose cached copy read as complete would be the silent
    /// cap again, with a longer life than the one that produced it.
    ///
    /// Defaulted, so an entry written before this field existed reads as a
    /// complete listing rather than as an unparseable one. That is the wrong
    /// answer for at most a week — [`REFRESH_AFTER_DAYS`] — and a cache miss
    /// would cost a fetch to correct something that expires on its own.
    #[serde(default)]
    pub(crate) more: Option<More>,
}

impl Cached {
    /// How many days ago this was read, when that is knowable.
    ///
    /// `None` covers a file written by a machine with no working clock, and a
    /// clock that has since moved backwards. Neither is worth failing over, and
    /// both are worth refusing to answer about rather than reporting a
    /// negative age.
    pub(crate) fn age_days(&self, now_unix: Option<i64>) -> Option<i64> {
        let then = self.fetched_unix?;
        let now = now_unix?;
        Some((now - then).max(0) / 86_400)
    }

    /// Whether this is old enough to be worth reading again.
    ///
    /// An entry whose age cannot be worked out counts as stale. That is the
    /// safe direction: the cost of refreshing something that did not need it is
    /// one request, and the cost of serving something indefinitely is the
    /// expired-voice failure this whole module exists to prevent.
    pub(crate) fn is_stale(&self, now_unix: Option<i64>) -> bool {
        self.age_days(now_unix)
            .is_none_or(|days| days >= REFRESH_AFTER_DAYS)
    }
}

/// Where a project's cached listings live, whether or not anything has written
/// one yet.
///
/// Public because the directory itself is worth knowing about from outside:
/// [`cached`](super::cached) answers what is in it *now*, and a window that
/// stays open needs to notice it change — which is a question about the
/// directory rather than about any listing in it. Handed out as a path rather
/// than as two string constants so that where a listing lands stays this
/// module's business.
pub fn cache_dir(root: &Path) -> PathBuf {
    root.join(CACHE_DIR).join(VOICES_DIR)
}

/// Where a query's answer is kept, inside `root`.
fn file(root: &Path, key: &str) -> PathBuf {
    cache_dir(root).join(format!("{}.json", hash_bytes(key.as_bytes())))
}

/// The cached answer to `key`, if there is a readable one.
///
/// Every failure is a miss, and none of them is reported. A cache is an
/// optimisation, so a corrupt entry, a half-written file or a directory nobody
/// has created yet all mean the same thing to a caller: ask the vendor. Failing
/// a listing because a rebuildable file could not be parsed would be the
/// optimisation breaking the thing it exists to speed up.
pub(crate) fn read(root: &Path, key: &str) -> Option<Cached> {
    let text = std::fs::read_to_string(file(root, key)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Every listing this project has cached, whichever query wrote it.
///
/// The one read here that does not know what it is looking for. Every other
/// caller asks after a named listing and can spell the key; a surface with
/// neither a key nor a network can only ask *what is there at all*, and the
/// honest answer to that is the whole directory.
///
/// Unreadable entries are skipped for exactly the reason [`read`] skips them: a
/// cache is an optimisation, and a corrupt file in one is a miss rather than a
/// failure.
pub(crate) fn all(root: &Path) -> Vec<Cached> {
    let Ok(entries) = std::fs::read_dir(root.join(CACHE_DIR).join(VOICES_DIR)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Cached>(&text).ok())
        .collect()
}

/// Writes a listing into the cache, creating `cache/voices/` if it is the
/// project's first.
///
/// Atomically, for the reason every other write in this crate is: a truncated
/// file here is not a crash but something worse — a short list that looks like
/// a complete one, served for a week.
pub(crate) fn write(root: &Path, key: &str, page: &Page) -> Result<(), std::io::Error> {
    let path = file(root, key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let unix = Timestamp::unix_now();
    let cached = Cached {
        listing: key.to_owned(),
        fetched_at: unix.and_then(Timestamp::from_unix),
        fetched_unix: unix,
        voices: page.voices.clone(),
        more: page.more,
    };
    let text = serde_json::to_string_pretty(&cached)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    scorsese_core::write::atomically(&path, text.as_bytes())
}
