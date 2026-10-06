//! Turning a downloaded zip into an installed browser: verify, unpack, stamp.
//!
//! Everything here is a local file operation — the download itself is
//! [`super::Source`]'s — so all of it is tested on a zip a test writes.
//!
//! The layout is `tools/chromium/fetch`'s, under a different root:
//! `<root>/<version>/chrome-headless-shell-<platform>/`, with a `.sha256` stamp
//! naming the checksum the zip was verified against. A pin that changes a
//! checksum without changing the version therefore still reinstalls.

use std::fs;
use std::path::{Path, PathBuf};

use super::ChromiumError;
use super::pin::Build;

/// The file beside the binary naming the checksum it was installed from.
const STAMP: &str = ".sha256";

/// The folder `build` is installed in under `root`.
fn target(root: &Path, build: &Build) -> PathBuf {
    root.join(&build.version).join(build.platform.folder())
}

/// The binary of `build` under `root`, if it is installed there: present, and
/// stamped with the checksum the pin names now.
pub(super) fn installed_in(root: &Path, build: &Build) -> Option<PathBuf> {
    let target = target(root, build);
    let binary = target.join(build.platform.binary());
    let stamp = fs::read_to_string(target.join(STAMP)).ok()?;
    (binary.is_file() && stamp.trim() == build.sha256).then_some(binary)
}

/// Installs `build` from `zip` (a downloaded file) under `root`, and answers
/// the binary's path.
///
/// The zip is refused unless it hashes to the pinned sha256: a binary this
/// installs is always the pinned build, which is what makes a golden rendered
/// with it mean anything. It is unpacked beside the target and moved into place
/// in one step, so an interrupted install never leaves a folder that looks
/// installed — and under a name of this process's own, so two programs
/// installing at once cannot interleave. Whichever finishes second finds the
/// first one's install and keeps it.
pub(super) fn unpack(zip: &Path, build: &Build, root: &Path) -> Result<PathBuf, ChromiumError> {
    let got = scorsese_core::pool::hash_file(zip)?;
    if got != build.sha256 {
        return Err(ChromiumError::Checksum {
            url: build.url.clone(),
            expected: build.sha256.clone(),
            got,
        });
    }
    let version = root.join(&build.version);
    let work = version.join(format!(".unpack.{}", std::process::id()));
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    let unpacked = extract(zip, &work).and_then(|()| {
        let folder = work.join(build.platform.folder());
        if !folder.join(build.platform.binary()).is_file() {
            return Err(ChromiumError::Unpack(format!(
                "the zip has no {}/{}",
                build.platform.folder(),
                build.platform.binary()
            )));
        }
        fs::write(folder.join(STAMP), &build.sha256)?;
        unquarantine(&folder);
        Ok(folder)
    });
    let placed = unpacked.and_then(|folder| place(&folder, root, build));
    // Best effort: what is left of the work folder is ours alone, and a
    // leftover is a few files in a folder nobody reads, never a wrong install.
    let _ = fs::remove_dir_all(&work);
    placed
}

/// Moves the unpacked `folder` to its target, replacing whatever was there.
fn place(folder: &Path, root: &Path, build: &Build) -> Result<PathBuf, ChromiumError> {
    let target = target(root, build);
    if let Some(binary) = installed_in(root, build) {
        return Ok(binary);
    }
    if target.exists() {
        fs::remove_dir_all(&target)?;
    }
    match fs::rename(folder, &target) {
        Ok(()) => Ok(target.join(build.platform.binary())),
        // Another program installed it between the look above and the move.
        Err(error) => installed_in(root, build).ok_or(ChromiumError::Io(error)),
    }
}

/// Unpacks every entry of `zip` under `into`, keeping each file's Unix mode so
/// the browser stays executable. Entries naming a path outside `into` are
/// refused by the zip reader itself.
fn extract(zip: &Path, into: &Path) -> Result<(), ChromiumError> {
    let mut archive = zip::ZipArchive::new(fs::File::open(zip)?)
        .map_err(|error| ChromiumError::Unpack(error.to_string()))?;
    archive
        .extract(into)
        .map_err(|error| ChromiumError::Unpack(error.to_string()))
}

/// Takes macOS's quarantine attribute off everything in `folder`.
///
/// A file a browser downloads is marked quarantined, and Gatekeeper then
/// refuses to run it until somebody clicks through a dialog — which for a
/// headless helper nobody sees would be a page that silently never captures.
/// A download made by this program is not normally marked, but one the person
/// unpacked into place themselves, or a zip a browser fetched, would be. So it
/// is cleared every time, and a failure to clear it is not an error: the
/// launch that follows says what is wrong far better than a guess here.
#[cfg(target_os = "macos")]
fn unquarantine(folder: &Path) {
    let _ = std::process::Command::new("xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(folder)
        .output();
}

#[cfg(not(target_os = "macos"))]
fn unquarantine(_folder: &Path) {}
