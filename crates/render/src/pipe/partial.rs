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
/// atomic within one filesystem — inside a hidden folder this render made
/// ([`hidden_folder`]), so nothing placed beside `out` beforehand (a link,
/// #1672) is where ffmpeg writes. The extension does not matter: every muxer
/// here is pinned with `-f`. Dropped, the folder is removed — whatever stopped
/// the render, nothing half-written is left.
#[derive(Debug)]
pub(crate) struct Partial {
    folder: PathBuf,
    path: PathBuf,
    out: PathBuf,
}

impl Partial {
    /// Where a render to `out` writes while it works: `partial`, in a fresh
    /// `.{name}.<pid>-<n>.rendering` folder beside it.
    pub(crate) fn beside(out: &Path) -> Result<Self, RenderError> {
        let folder = hidden_folder(out, "rendering")?;
        Ok(Self {
            path: folder.join("partial"),
            folder,
            out: out.to_owned(),
        })
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
        // After a delivery the folder is empty; otherwise it holds the
        // half-written file.
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// A scratch folder beside `out`, hidden, named after it and `tag`, and made
/// by this call ([`scorsese_core::write::fresh_dir_beside`]) — so two
/// concurrent renders never collide, and nothing already beside `out` is
/// written through.
pub(crate) fn hidden_folder(out: &Path, tag: &str) -> Result<PathBuf, RenderError> {
    let name = out
        .file_name()
        .map_or_else(|| "render".into(), |name| name.to_string_lossy());
    let hidden = out.with_file_name(format!(".{name}"));
    scorsese_core::write::fresh_dir_beside(&hidden, tag).map_err(|source| RenderError::Scratch {
        path: hidden,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(label: &str) -> PathBuf {
        let dir = crate::scratch::fresh(&format!("scorsese-partial-{label}"), "");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    #[test]
    fn it_is_hidden_beside_the_output() {
        let dir = dir("hidden");
        let out = dir.join("cut.mp4");
        let partial = Partial::beside(&out).expect("a folder");
        let folder = partial.path().parent().expect("a folder");
        assert_eq!(folder.parent(), Some(dir.as_path()));
        let name = folder.file_name().expect("a name").to_string_lossy();
        assert!(
            name.starts_with(".cut.mp4.") && name.ends_with(".rendering"),
            "{name}"
        );
        assert_eq!(partial.out(), out);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn delivering_replaces_the_output_and_leaves_nothing_beside_it() {
        let dir = dir("deliver");
        let out = dir.join("cut.mp4");
        std::fs::write(&out, b"old").expect("the previous cut");
        let partial = Partial::beside(&out).expect("a folder");
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
        let partial = Partial::beside(&out).expect("a folder");
        std::fs::write(partial.path(), b"half").expect("half a cut");

        drop(partial);

        assert_eq!(std::fs::read(&out).expect("a cut"), b"old");
        assert_eq!(std::fs::read_dir(&dir).expect("listed").count(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
