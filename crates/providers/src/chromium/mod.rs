//! The page renderer, fetched the first time a page needs drawing (#776).
//!
//! An `html` clip is drawn by the pinned `chrome-headless-shell`
//! (`tools/chromium/pin`, #772). A developer can point `SCORSESE_CHROME` at one;
//! everybody else should never learn it exists. So the CLI, the MCP server and
//! the desktop app hand `scorsese-render` this module's two halves — the build
//! already on this machine ([`installed`]) and how to get it there ([`fetch`])
//! — and the renderer asks for the second only when a page is on screen and no
//! browser was found any other way.
//!
//! **Here, because this crate is the one that reaches a network.** Every
//! request scorsese makes goes through [`crate::api::http`], and the folder the
//! build is kept in is the settings file's (`docs/credentials.md`), resolved by
//! the same function. `scorsese-render` knows the browser and none of this: it
//! is handed two closures and never learns where a byte came from.
//!
//! Kept per machine, not per project: `<settings folder>/chromium/<version>/
//! chrome-headless-shell-<platform>/`. It is ~100–150 MB compressed, and one
//! copy serves every project.

mod install;
mod pin;
#[cfg(test)]
mod tests;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

pub use pin::{Build, Platform};

use crate::api::http::{self, HttpError};

/// The most a download may be. The zips are ~100–150 MB; anything far past
/// that is not the file the pin names, and the checksum would refuse it anyway.
const LIMIT: u64 = 1024 * 1024 * 1024;

/// How far a download has got, for whoever is showing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fetching {
    /// Bytes received so far. Zero on the first call, before any arrive — the
    /// moment to say a download has started.
    pub received: u64,
    /// How many the server said to expect, when it said.
    pub total: Option<u64>,
}

/// Where a download's bytes come from: the network, or a test's file.
pub trait Source {
    /// Writes the file at `url` into `to`, telling `progress` as it arrives.
    fn get(
        &self,
        url: &str,
        to: &mut dyn Write,
        progress: &mut dyn FnMut(Fetching),
    ) -> Result<(), ChromiumError>;
}

/// The network, through [`crate::api::http`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Web;

impl Source for Web {
    fn get(
        &self,
        url: &str,
        to: &mut dyn Write,
        progress: &mut dyn FnMut(Fetching),
    ) -> Result<(), ChromiumError> {
        http::stream_to(url, LIMIT, to, &mut |received, total| {
            progress(Fetching { received, total });
        })?;
        Ok(())
    }
}

/// This machine's folder for downloaded builds, if the platform names one.
pub fn root() -> Option<PathBuf> {
    crate::credentials::machine_folder().map(|folder| folder.join("chromium"))
}

/// The pinned build's binary, if it is already downloaded on this machine.
/// Touches nothing but the disk.
pub fn installed() -> Option<PathBuf> {
    let build = Build::pinned(Platform::this()?).ok()?;
    install::installed_in(&root()?, &build)
}

/// Downloads the pinned build for this platform into [`root`], verifies it
/// against the pinned sha256, unpacks it and answers the binary — or answers
/// the one already there. `progress` hears about a download that happens, and
/// nothing when none does.
pub fn fetch(progress: &mut dyn FnMut(Fetching)) -> Result<PathBuf, ChromiumError> {
    let platform = Platform::this().ok_or(ChromiumError::Unsupported {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    })?;
    let root = root().ok_or(ChromiumError::NoFolder)?;
    fetch_with(&Web, &root, &Build::pinned(platform)?, progress)
}

/// [`fetch`], from `source` into `root` — the seam a test fetches through.
pub fn fetch_with(
    source: &dyn Source,
    root: &Path,
    build: &Build,
    progress: &mut dyn FnMut(Fetching),
) -> Result<PathBuf, ChromiumError> {
    // One download at a time in this program: the window may want two pages
    // at once, and the second should wait for the first rather than fetch the
    // same 150 MB beside it. Another program is `install`'s to tolerate.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(binary) = install::installed_in(root, build) {
        return Ok(binary);
    }
    let folder = root.join(&build.version);
    fs::create_dir_all(&folder)?;
    let zip = folder.join(format!(".download.{}.zip", std::process::id()));
    let fetched = fs::File::create(&zip)
        .map_err(ChromiumError::from)
        .and_then(|mut file| {
            source.get(&build.url, &mut file, progress)?;
            file.flush()?;
            Ok(())
        })
        .and_then(|()| install::unpack(&zip, build, root));
    // The zip is only ever a step: kept, it would be 150 MB beside the install.
    let _ = fs::remove_file(&zip);
    fetched
}

/// Why the page renderer could not be fetched. Never a failed render: the page
/// shows its slug card, with this as the reason.
#[derive(Debug, thiserror::Error)]
pub enum ChromiumError {
    /// Chrome for Testing publishes no build for this machine.
    #[error("there is no page renderer (chrome-headless-shell) for {os}/{arch}")]
    Unsupported {
        /// The operating system.
        os: &'static str,
        /// The processor.
        arch: &'static str,
    },
    /// The platform names no per-user folder to keep it in.
    #[error("no folder to keep the page renderer in: set HOME (or XDG_CONFIG_HOME, or APPDATA)")]
    NoFolder,
    /// The pin file lacks a line this needs.
    #[error("tools/chromium/pin has no {0} line")]
    Pin(String),
    /// The download did not complete.
    #[error("downloading the page renderer: {0}")]
    Download(#[from] HttpError),
    /// What arrived is not the pinned build.
    #[error(
        "the page renderer downloaded from {url} is not the pinned build: \
         expected sha256 {expected} (tools/chromium/pin), got {got}"
    )]
    Checksum {
        /// Where it came from.
        url: String,
        /// What the pin says.
        expected: String,
        /// What arrived.
        got: String,
    },
    /// The zip could not be read, or holds no browser.
    #[error("unpacking the page renderer: {0}")]
    Unpack(String),
    /// The disk refused.
    #[error("installing the page renderer: {0}")]
    Io(#[from] std::io::Error),
}
