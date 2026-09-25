//! Where a user's files live on disk, under the storage root.
//!
//! **`<SCORSESE_STORAGE>/users/<user id>/`**, and nothing of a user's
//! anywhere else. One directory per user is what makes deleting an account
//! delete its files — one `remove_dir_all`, with nothing to hunt for — and
//! what makes a stray path that escapes it easy to see in review. The library
//! (#535), renders (#541) and generated media all go under it; whatever lives
//! at the storage root outside `users/` is the server's own and rebuildable.
//!
//! The directory is named by id, never by email: an id is never reused and
//! never changes, and an email is personal data that has no business in a
//! path, a log line or a backup's file listing.

use std::io;
use std::path::{Path, PathBuf};

use crate::db::UserId;

/// The directory everything `user` owns on disk lives under.
pub fn user_directory(root: &Path, user: UserId) -> PathBuf {
    root.join("users").join(user.get().to_string())
}

/// Remove `user`'s directory and everything in it.
///
/// A directory that does not exist is already removed: an account that never
/// uploaded anything has none. On failure, returns the directory with the
/// error so the caller can name it.
pub fn remove_user(root: &Path, user: UserId) -> Result<(), (PathBuf, io::Error)> {
    let directory = user_directory(root, user);
    match std::fs::remove_dir_all(&directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err((directory, error)),
    }
}
