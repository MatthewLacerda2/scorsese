//! The folder a render job works in, and its lifetime.

use std::path::PathBuf;

/// A job's scratch folder: emptied when the job starts — a recovered job
/// begins again from nothing — and removed when it ends, however it ends.
pub(super) struct Scratch(pub(super) PathBuf);

impl Scratch {
    pub(super) fn new(path: PathBuf) -> Self {
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
