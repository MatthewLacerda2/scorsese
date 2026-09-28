//! Where a user's files live on disk: kept under the storage root, rebuilt
//! under the cache.
//!
//! **`<SCORSESE_STORAGE>/users/<user id>/`** holds what cannot be made again —
//! the library (#535), `library/<sha256>.<extension>` — and is backed up off the
//! machine every night. **`<SCORSESE_CACHE>/users/<user id>/`** holds what can:
//! thumbnails, and uploads still on their way in. The two are kept apart
//! because the backup ships the first whole (`docs/web.md`, *Running the
//! service*), and a thumbnail or half an upload is not worth that bandwidth.
//!
//! One directory per user in each is what makes deleting an account delete its
//! files — two `remove_dir_all`s, with nothing to hunt for — and what makes a
//! stray path that escapes them easy to see in review.
//!
//! The directories are named by id, never by email: an id is never reused and
//! never changes, and an email is personal data that has no business in a
//! path, a log line or a backup's file listing.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::db::UserId;

/// Each user's scratch folders, under their cache directory.
const SCRATCH: &str = "scratch";

/// The directory everything `user` owns under `root` lives in.
pub fn user_directory(root: &Path, user: UserId) -> PathBuf {
    root.join("users").join(user.get().to_string())
}

/// The two roots, and every path the server keeps a user's file at.
#[derive(Debug, Clone)]
pub struct Storage {
    kept: PathBuf,
    cache: PathBuf,
}

impl Storage {
    /// Users' files kept under `kept` (`SCORSESE_STORAGE`), and what can be
    /// rebuilt under `cache` (`SCORSESE_CACHE`).
    pub fn new(kept: impl Into<PathBuf>, cache: impl Into<PathBuf>) -> Self {
        Self {
            kept: kept.into(),
            cache: cache.into(),
        }
    }

    /// Create both roots, naming the one that could not be.
    pub fn create(&self) -> Result<(), (PathBuf, io::Error)> {
        for root in [&self.kept, &self.cache] {
            std::fs::create_dir_all(root).map_err(|error| (root.clone(), error))?;
        }
        Ok(())
    }

    /// Where the library keeps the file with this hash.
    pub fn library_file(&self, user: UserId, sha256: &str, extension: &str) -> PathBuf {
        user_directory(&self.kept, user)
            .join("library")
            .join(format!("{sha256}.{extension}"))
    }

    /// Where the bytes of upload `id` collect while it arrives. The extension
    /// is the file's own, so a prober that goes by it reads the file right.
    pub fn upload_file(&self, user: UserId, id: i64, extension: &str) -> PathBuf {
        user_directory(&self.cache, user)
            .join("uploads")
            .join(format!("{id}.{extension}"))
    }

    /// Where the thumbnail of the file with this hash is kept.
    pub fn thumbnail(&self, user: UserId, sha256: &str, extension: &str) -> PathBuf {
        user_directory(&self.cache, user)
            .join("thumbnails")
            .join(format!("{sha256}.{extension}"))
    }

    /// Where the preview proxy of the file with this hash is kept (#542).
    /// Under the cache, beside the thumbnails, for their reason: it is made
    /// again from the file whenever it is missing.
    pub fn proxy(&self, user: UserId, sha256: &str) -> PathBuf {
        user_directory(&self.cache, user)
            .join("proxies")
            .join(scorsese_render::preview::file_name(sha256))
    }

    /// A folder nothing is at yet, for laying one of `user`'s projects out
    /// while a tool runs on it (#539). Under the cache, because it is gone
    /// the moment the tool answers; [`Storage::clear_scratch`] takes whatever
    /// a stopped server left.
    pub fn scratch(&self, user: UserId) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        user_directory(&self.cache, user)
            .join(SCRATCH)
            .join(format!("{nanos}-{unique}.scor"))
    }

    /// Remove every user's scratch folders. Only while nothing runs on them
    /// — the server calls it before it serves. Best-effort: a folder that
    /// will not go is only disk, and the next start tries again.
    pub fn clear_scratch(&self) {
        let Ok(users) = std::fs::read_dir(self.cache.join("users")) else {
            return;
        };
        for user in users.flatten() {
            let _ = std::fs::remove_dir_all(user.path().join(SCRATCH));
        }
    }

    /// Remove everything `user` has on disk, kept and cached.
    ///
    /// A directory that does not exist is already removed: an account that
    /// never uploaded anything has none. On failure, returns the directory
    /// with the error so the caller can name it.
    pub fn remove_user(&self, user: UserId) -> Result<(), (PathBuf, io::Error)> {
        for root in [&self.kept, &self.cache] {
            let directory = user_directory(root, user);
            match std::fs::remove_dir_all(&directory) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => {
                    return Err((directory, error));
                }
                _ => {}
            }
        }
        Ok(())
    }
}
