//! Where a picture is written on the way to being sent.
//!
//! Every tool here that answers with an image goes through a file, because the
//! PNG encoding is ffmpeg's and ffmpeg writes files. Two of them share this:
//! `still`, which will keep the file when a caller names a path, and `look`,
//! which never does.

use std::path::PathBuf;

/// Where the PNG is written, and whether it survives the call.
///
/// A caller who named a path gets the file and keeps it. A caller who did not
/// still needs one written somewhere, because the encoding is ffmpeg's and
/// ffmpeg writes files — so it goes to a scratch path that is removed on the
/// way out, including when the call fails partway through. A server left
/// littering the temporary directory with frames is a server nobody notices
/// filling a disk.
pub(crate) struct Scratch {
    pub(crate) path: PathBuf,
    remove: bool,
}

impl Scratch {
    pub(crate) fn at(kept: Option<&str>) -> Self {
        match kept {
            Some(path) => Self {
                path: PathBuf::from(path),
                remove: false,
            },
            None => Self {
                path: std::env::temp_dir().join(format!(
                    "scorsese-frame-{}-{}.png",
                    std::process::id(),
                    unique()
                )),
                remove: true,
            },
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if self.remove {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// A number no other call in this process has used, so two frames asked for at
/// once cannot land on one scratch file.
fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
