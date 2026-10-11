//! Writing a delivery beside its name, so a render that fails costs only
//! itself.

use std::path::{Path, PathBuf};

use crate::error::RenderError;

/// The file an encoder writes while it works: hidden, beside `out`, and only
/// put in `out`'s place once ffmpeg has finished it.
///
/// Encoding straight into `out` truncated whatever was there the moment ffmpeg
/// started, so a re-render of `cut.mp4` that hit a broken source, a page that
/// failed, or a cancel lost the last good cut along with the new one (#1086).
/// Re-rendering to the same name is the normal loop, for an agent and a person
/// alike; a failure in it should cost the new render, not the old one.
///
/// Beside `out` rather than in a scratch directory because a rename is only
/// atomic within one filesystem. The extension does not matter: every muxer
/// here is pinned with `-f`. Dropped without [`Partial::deliver`], the file
/// is removed — whatever stopped the render, nothing half-written is left.
#[derive(Debug)]
pub(crate) struct Partial {
    path: PathBuf,
    out: PathBuf,
}

impl Partial {
    /// Where a render to `out` writes while it works: `.{name}.partial`.
    pub(crate) fn beside(out: &Path) -> Self {
        let name = out
            .file_name()
            .map_or_else(|| "render".into(), |name| name.to_string_lossy());
        Self {
            path: out.with_file_name(format!(".{name}.partial")),
            out: out.to_owned(),
        }
    }

    /// The file ffmpeg is told to write.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// The name the finished file will have — what an error about it says.
    pub(crate) fn out(&self) -> &Path {
        &self.out
    }

    /// Puts the finished file in `out`'s place, replacing what was there.
    pub(crate) fn deliver(self) -> Result<(), RenderError> {
        std::fs::rename(&self.path, &self.out).map_err(|source| RenderError::Deliver {
            path: self.out.clone(),
            source,
        })
        // On failure `self` drops here and takes the partial file with it.
    }
}

impl Drop for Partial {
    fn drop(&mut self) {
        // After a delivery there is nothing at `path`, and this is a no-op.
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("scorsese-partial-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    #[test]
    fn it_is_hidden_beside_the_output() {
        let partial = Partial::beside(Path::new("renders/cut.mp4"));
        assert_eq!(partial.path(), Path::new("renders/.cut.mp4.partial"));
        assert_eq!(partial.out(), Path::new("renders/cut.mp4"));
    }

    #[test]
    fn delivering_replaces_the_output_and_leaves_nothing_beside_it() {
        let dir = dir("deliver");
        let out = dir.join("cut.mp4");
        std::fs::write(&out, b"old").expect("the previous cut");
        let partial = Partial::beside(&out);
        std::fs::write(partial.path(), b"new").expect("the new cut");

        partial.deliver().expect("delivered");

        assert_eq!(std::fs::read(&out).expect("a cut"), b"new");
        assert_eq!(std::fs::read_dir(&dir).expect("listed").count(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn abandoning_leaves_the_previous_output_as_it_was() {
        let dir = dir("abandon");
        let out = dir.join("cut.mp4");
        std::fs::write(&out, b"old").expect("the previous cut");
        let partial = Partial::beside(&out);
        std::fs::write(partial.path(), b"half").expect("half a cut");

        drop(partial);

        assert_eq!(std::fs::read(&out).expect("a cut"), b"old");
        assert_eq!(std::fs::read_dir(&dir).expect("listed").count(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
