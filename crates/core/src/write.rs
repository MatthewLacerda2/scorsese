//! Writing into a project without ever leaving it half-written.
//!
//! A project is one JSON document, and the CLI, the MCP server and the GUI all
//! read and write it. There is no database behind it and no journal to recover
//! from: if that file is truncated, the work is gone.
//!
//! `fs::write` truncates the target and then fills it. Between those two
//! moments the document on disk is incomplete, and a reader arriving in that
//! window sees something that is not a project and never was — the MCP
//! server's read is stateless and can land at any moment, and a watcher's
//! whole job is to react the instant the file changes, which is precisely the
//! instant it is least likely to be whole.
//!
//! Writing beside the target and renaming into place closes the window.
//! Rename is atomic on every platform scorsese targets, so a reader sees
//! either the whole previous document or the whole new one, and never
//! anything between.
//!
//! Everything that writes a document into a project comes through here rather
//! than repeating the pattern: `project.json`, authored recipes and pages, and
//! baked media.
//!
//! A file of the project is written through [`in_project`], which also
//! refuses a folder on the way that is a link: on the web a project is laid
//! out with links into a member's storage, and a write through one would land
//! outside the project.
//!
//! Importing does not, and needs not: `pool` copies a file in under a name no
//! asset points at yet and records it only once the copy is whole, so a copy
//! cut short leaves a stray file nothing reads, never a half-written one a
//! project does.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::path::ProjectPath;

/// Tells concurrent writers' scratch files apart. Two saves of one project —
/// a drag in the GUI and an assistant's edit over MCP — must not land on each
/// other's scratch file, or the rename publishes a document neither of them
/// wrote.
static WRITES: AtomicU64 = AtomicU64::new(0);

/// Writes `contents` over `path`, without any reader ever observing a partial
/// document.
///
/// The bytes go to a scratch file in the **same directory** — a rename across
/// filesystems is a copy, and a copy is not atomic — and reach the device
/// before that file is renamed into place.
///
/// The parent directory has to exist already: this replaces a document, it
/// does not build the project around it.
///
/// # Errors
///
/// If the scratch file cannot be written or renamed. In that case `path` still
/// holds whatever it held before, and no scratch file is left beside it.
pub fn atomically(path: &Path, contents: impl AsRef<[u8]>) -> io::Result<()> {
    let scratch = scratch_beside(path, "part")?;
    let written = fill(&scratch, contents.as_ref()).and_then(|()| fs::rename(&scratch, path));
    if written.is_err() {
        // The previous document survives a failed write; it must not have to
        // survive alongside the debris of one.
        let _ = fs::remove_file(&scratch);
    }
    written
}

/// A scratch path in the same directory as `path` that no other writer is
/// using, in this process or another: `path`'s name, this process's id, a
/// number no other call in it has had, and `suffix`. What a file is written
/// under before it is renamed into place — beside it, because a rename is only
/// atomic within one filesystem.
///
/// # Errors
///
/// If `path` names no file (it ends in `..`, or is a root).
pub fn scratch_beside(path: &Path, suffix: &str) -> io::Result<PathBuf> {
    let name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} does not name a file to write", path.display()),
        )
    })?;
    // A path of one component has an empty parent, and joining onto that gives
    // the name back — so a relative `project.json` still writes beside itself.
    let directory = path.parent().unwrap_or_else(|| Path::new(""));
    let mut scratch = name.to_os_string();
    scratch.push(format!(
        ".{}-{}.{suffix}",
        std::process::id(),
        WRITES.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(directory.join(scratch))
}

/// Writes `contents` as the project file `path` under `root`, atomically
/// ([`atomically`]), making the folders it needs — and never through a link.
///
/// A folder on the way that is a link is refused: what it leads to is not the
/// project's, so nothing is written there. A file that is a link itself is
/// replaced by the new file, as the rename always does, and what it named is
/// left as it was. The real path written is the answer.
///
/// # Errors
///
/// If `path` names no file inside the project, a folder on the way is a link
/// or a file, or the write fails as [`atomically`]'s does.
pub fn in_project(
    root: &Path,
    path: &ProjectPath,
    contents: impl AsRef<[u8]>,
) -> io::Result<PathBuf> {
    let file = parent_in_project(root, path)?;
    atomically(&file, contents)?;
    Ok(file)
}

/// The real path of the project file `path` under `root`, every folder above it
/// made and none of them a link — what a writer that is not [`in_project`]
/// asks before it writes.
///
/// # Errors
///
/// If `path` names no file inside the project, or a folder on the way is a
/// link, a file, or cannot be made.
pub fn parent_in_project(root: &Path, path: &ProjectPath) -> io::Result<PathBuf> {
    let file = path.resolve(root)?;
    let segments: Vec<&str> = path.as_str().split('/').collect();
    let mut at = root.to_path_buf();
    for (index, segment) in segments.iter().enumerate().take(segments.len() - 1) {
        at.push(segment);
        let shown = || segments[..=index].join("/");
        match fs::symlink_metadata(&at) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "`{}` is a link, and nothing in a project is written through one",
                        shown()
                    ),
                ));
            }
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("`{}` is a file, not a folder", shown()),
                ));
            }
            // Not there, or not readable: made, or the reason it cannot be.
            Err(_) => {
                if let Err(error) = fs::create_dir(&at) {
                    // A real folder another writer made in between is as good;
                    // a link made in between is not one.
                    if !fs::symlink_metadata(&at).is_ok_and(|meta| meta.is_dir()) {
                        return Err(error);
                    }
                }
            }
        }
    }
    Ok(file)
}

/// Fills the scratch file and makes sure it is really on the device.
fn fill(scratch: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = File::create(scratch)?;
    file.write_all(contents)?;
    // The rename is what publishes the document; this is what makes it *there*.
    // Without it, a crash between the two can publish a file whose bytes never
    // reached the disk — atomic, and empty.
    file.sync_all()
}
