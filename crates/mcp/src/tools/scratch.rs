//! Where a picture is written on the way to being sent.
//!
//! Every tool here that answers with an image goes through a file, because the
//! PNG encoding is ffmpeg's and ffmpeg writes files. [`Scratch::png`] is that
//! round trip for `hear`, `look` and `still`; only `still` keeps the file, when
//! a caller names a path.

use std::path::{Path, PathBuf};

use scorsese_render::{Frame, Tools, frames};

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
    pub(crate) fn at(kept: Option<&Path>) -> Self {
        match kept {
            Some(path) => Self {
                path: path.to_path_buf(),
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

    /// `frame` as PNG bytes, written to `kept` (or a scratch file that is
    /// removed) and read back. `what` names the picture in the refusal when
    /// ffmpeg cannot write it: "the frame", "the sheet".
    pub(crate) fn png(
        tools: &Tools,
        frame: &Frame,
        kept: Option<&Path>,
        what: &str,
    ) -> Result<Vec<u8>, String> {
        let png = Self::at(kept);
        frames::write_png(tools, &png.path, frame)
            .map_err(|error| format!("writing {what}: {error}"))?;
        std::fs::read(&png.path)
            .map_err(|error| format!("reading {} back: {error}", png.path.display()))
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
