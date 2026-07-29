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
//! than repeating the pattern: `project.json`, authored recipes, and baked
//! media.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
    let scratch = beside(path)?;
    let written = fill(&scratch, contents.as_ref()).and_then(|()| fs::rename(&scratch, path));
    if written.is_err() {
        // The previous document survives a failed write; it must not have to
        // survive alongside the debris of one.
        let _ = fs::remove_file(&scratch);
    }
    written
}

/// A scratch path in the same directory as `path`, unique to this write.
fn beside(path: &Path) -> io::Result<PathBuf> {
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
        ".{}-{}.part",
        std::process::id(),
        WRITES.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(directory.join(scratch))
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
