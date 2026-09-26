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

use crate::db::UserId;

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
